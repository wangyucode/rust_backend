use crate::app_state::{AiConfig, AppState};
use crate::controller::ai_tools as tools;
use crate::dao::ai as ai_dao;
use axum::{
    extract::{Request, State},
    http::StatusCode,
    response::{IntoResponse, Response, Sse, sse::Event},
};
use futures::StreamExt as _;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::codec::{FramedRead, LinesCodec};

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub user_id: String,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Option<String>,
    pub parts: Option<Vec<MessagePart>>,
}

#[derive(Debug, Deserialize)]
pub struct MessagePart {
    #[serde(rename = "type")]
    pub part_type: String,
    pub text: Option<String>,
}

impl ChatMessage {
    pub fn get_content(&self) -> String {
        if let Some(c) = &self.content {
            if !c.is_empty() {
                return c.clone();
            }
        }
        if let Some(parts) = &self.parts {
            return parts
                .iter()
                .filter(|p| p.part_type == "text")
                .filter_map(|p| p.text.clone())
                .collect::<Vec<_>>()
                .join("\n");
        }
        String::new()
    }
}

// ---------- Agent / OpenAI 兼容协议 ----------

/// Agent 单次请求最多 LLM 轮数（含工具轮 + 最终轮）
const MAX_ROUNDS: usize = 5;
/// 单轮 LLM 调用的超时
const ROUND_TIMEOUT: Duration = Duration::from_secs(90);
/// 回放 text-delta 时的切片大小（模拟流式观感，省一次 LLM 调用）
const REPLAY_CHUNK_CHARS: usize = 1500;

#[derive(Debug, Clone, Serialize)]
struct AgentMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ToolCall {
    id: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none", default)]
    kind: Option<String>,
    function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FunctionCall {
    name: String,
    arguments: String,
}

#[derive(Debug, Serialize)]
struct AgentBackendRequest {
    model: String,
    messages: Vec<AgentMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    #[serde(default)]
    choices: Vec<ChatCompletionChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionChoice {
    message: AssistantMessage,
}

#[derive(Debug, Deserialize)]
struct AssistantMessage {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionChunk {
    choices: Vec<ChatCompletionChunkChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionChunkChoice {
    delta: ChatCompletionChunkDelta,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionChunkDelta {
    content: Option<String>,
}

/// 一次 read 的前端展示事件（reading + done 共用同一 rf_id）。
struct ReadEvent {
    rf_id: String,
    path: String,
    bytes: usize,
    truncated: bool,
    is_error: bool,
}

/// 读取系统提示词文件，作为 system message
async fn load_system_prompt() -> String {
    // 优先 from 当前目录下的 data 文件夹读取，适配运行时路径
    let prompt_path = std::path::Path::new("data")
        .join("prompt")
        .join("secretary.md");

    match tokio::fs::read_to_string(&prompt_path).await {
        Ok(content) => content,
        Err(e) => {
            eprintln!(
                "⚠️  Failed to load system prompt from {:?}: {}",
                prompt_path, e
            );
            "You are a helpful assistant.".to_string()
        }
    }
}

/// 非流式调用 LLM 一轮（工具轮）。返回 assistant 消息。
async fn call_llm_once(
    client: &Client,
    cfg: &AiConfig,
    messages: &[AgentMessage],
    with_tools: bool,
) -> anyhow::Result<AssistantMessage> {
    let req = AgentBackendRequest {
        model: cfg.model.clone(),
        messages: messages.to_vec(),
        stream: false,
        tools: if with_tools {
            Some(vec![tools::read_tool_definition()])
        } else {
            None
        },
    };
    let body = serde_json::to_string(&req)?;
    let fut = client
        .post(&cfg.base_url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", cfg.api_key))
        .body(body)
        .send();
    let resp = tokio::time::timeout(ROUND_TIMEOUT, fut)
        .await
        .map_err(|_| {
            anyhow::anyhow!("AI request timed out after {}s", ROUND_TIMEOUT.as_secs())
        })??;
    if !resp.status().is_success() {
        let text = resp
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        anyhow::bail!("AI API error: {}", text);
    }
    let parsed: ChatCompletionResponse = resp.json().await?;
    parsed
        .choices
        .into_iter()
        .next()
        .map(|c| c.message)
        .ok_or_else(|| anyhow::anyhow!("AI returned no choices"))
}

fn extract_read_path(tc: &ToolCall) -> Option<String> {
    if tc.function.name != "read" {
        return None;
    }
    serde_json::from_str::<serde_json::Value>(&tc.function.arguments)
        .ok()
        .and_then(|v| v.get("path")?.as_str().map(|s| s.to_string()))
}

/// Agent 循环（工具轮非流式 + 最终答案回放）。
/// 成功返回 (read 事件, 最终回答全文)；失败返回错误描述（调用方降级为纯转发）。
async fn run_agent_loop(
    client: &Client,
    cfg: &AiConfig,
    system_prompt: &str,
    history: &[ChatMessage],
) -> Result<(Vec<ReadEvent>, String), String> {
    let mut messages: Vec<AgentMessage> = Vec::with_capacity(history.len() + 8);
    messages.push(AgentMessage {
        role: "system".to_string(),
        content: Some(system_prompt.to_string()),
        tool_calls: None,
        tool_call_id: None,
    });
    for msg in history {
        messages.push(AgentMessage {
            role: msg.role.clone(),
            content: Some(msg.get_content()),
            tool_calls: None,
            tool_call_id: None,
        });
    }

    let mut read_events: Vec<ReadEvent> = Vec::new();
    let mut read_count: usize = 0;

    for round in 0..MAX_ROUNDS {
        let last_round = round + 1 == MAX_ROUNDS;
        let assistant = call_llm_once(client, cfg, &messages, !last_round)
            .await
            .map_err(|e| {
                eprintln!("Agent round {round} failed: {e:?}");
                e.to_string()
            })?;

        let pending: Vec<ToolCall> = assistant.tool_calls.clone().unwrap_or_default();
        if pending.is_empty() || last_round {
            // 最终回答（最后一轮即使带 tools 也强制收尾，不再执行）
            let content = assistant.content.unwrap_or_default();
            eprintln!(
                "Agent done after {} round(s), {} read(s), answer {} chars",
                round + 1,
                read_count,
                content.len()
            );
            return Ok((read_events, content));
        }

        // 工具轮：执行 read，回填 tool 消息
        eprintln!("Agent round {round}: {} tool_call(s)", pending.len());
        let mut tool_msgs: Vec<AgentMessage> = Vec::new();
        // assistant(tool_calls) 本体先入队，保持 OpenAI 协议完整
        messages.push(AgentMessage {
            role: "assistant".to_string(),
            content: assistant.content.clone(),
            tool_calls: Some(pending.clone()),
            tool_call_id: None,
        });
        for tc in &pending {
            match extract_read_path(tc) {
                Some(path) if read_count < tools::MAX_READS_PER_REQUEST => {
                    read_count += 1;
                    let rf_id = format!("rf_{}", uuid::Uuid::new_v4().simple());
                    let outcome = tools::execute_read(&cfg.www_root, &path).await;
                    eprintln!(
                        "read #{read_count} {} -> {} bytes{}",
                        outcome.requested_path,
                        outcome.bytes,
                        if outcome.is_error { " (error)" } else { "" }
                    );
                    read_events.push(ReadEvent {
                        rf_id,
                        path: outcome.requested_path.clone(),
                        bytes: outcome.bytes,
                        truncated: outcome.truncated,
                        is_error: outcome.is_error,
                    });
                    tool_msgs.push(AgentMessage {
                        role: "tool".to_string(),
                        content: Some(outcome.text),
                        tool_calls: None,
                        tool_call_id: Some(tc.id.clone()),
                    });
                }
                Some(path) => {
                    // 次数超限：不再执行，明确告诉模型收尾
                    eprintln!("read limit reached, skip {path}");
                    tool_msgs.push(AgentMessage {
                        role: "tool".to_string(),
                        content: Some(format!(
                            "read 次数已达上限（{} 次），请基于已有信息直接作答，不要再调用工具：{path}",
                            tools::MAX_READS_PER_REQUEST
                        )),
                        tool_calls: None,
                        tool_call_id: Some(tc.id.clone()),
                    });
                }
                None => {
                    eprintln!("unsupported tool call: {}", tc.function.name);
                    tool_msgs.push(AgentMessage {
                        role: "tool".to_string(),
                        content: Some(format!(
                            "不支持的工具 {}，仅可使用 read 工具读取 /www/ 下的文件。",
                            tc.function.name
                        )),
                        tool_calls: None,
                        tool_call_id: Some(tc.id.clone()),
                    });
                }
            }
        }
        messages.extend(tool_msgs);
    }
    // 正常不会到达（循环内必 return），兜底：不带 tools 强制收尾
    let assistant = call_llm_once(client, cfg, &messages, false)
        .await
        .map_err(|e| e.to_string())?;
    Ok((read_events, assistant.content.unwrap_or_default()))
}

/// 把最终全文切成多片 text-delta 回放，模拟流式观感。
fn chunk_text(s: &str) -> Vec<String> {
    if s.is_empty() {
        return vec![];
    }
    let chars: Vec<char> = s.chars().collect();
    chars
        .chunks(REPLAY_CHUNK_CHARS)
        .map(|c| c.iter().collect())
        .collect()
}

/// 构造 Agent SSE 回放响应：text-start → data-read_file* → text-delta* → text-end。
fn agent_sse_response(
    message_id: &str,
    reads: Vec<ReadEvent>,
    final_content: String,
    persist_tx: tokio::sync::mpsc::Sender<String>,
) -> Response {
    let start = Event::default()
        .event("message")
        .data(format!(r#"{{"type":"text-start","id":"{}"}}"#, message_id));
    let mut payloads: Vec<String> = Vec::new();
    for r in &reads {
        payloads.push(
            serde_json::json!({
                "type": "data-read_file",
                "id": r.rf_id,
                "data": {"path": r.path, "status": "reading"}
            })
            .to_string(),
        );
        let mut done_data = serde_json::json!({"path": r.path, "status": "done", "bytes": r.bytes, "truncated": r.truncated});
        if r.is_error {
            done_data["error"] = serde_json::Value::Bool(true);
        }
        payloads.push(
            serde_json::json!({
                "type": "data-read_file",
                "id": r.rf_id,
                "data": done_data
            })
            .to_string(),
        );
    }
    for chunk in chunk_text(&final_content) {
        let _ = persist_tx.try_send(chunk.clone());
        payloads.push(
            serde_json::json!({
                "type": "text-delta",
                "id": message_id,
                "delta": chunk
            })
            .to_string(),
        );
    }
    // 空回答兜底，避免前端一直转圈
    if final_content.is_empty() {
        let fallback = "抱歉，我暂时没能组织好回答，请换个问法再试一次～";
        let _ = persist_tx.try_send(fallback.to_string());
        payloads.push(
            serde_json::json!({
                "type": "text-delta",
                "id": message_id,
                "delta": fallback
            })
            .to_string(),
        );
    }
    drop(persist_tx);
    let end = Event::default()
        .event("message")
        .data(format!(r#"{{"type":"text-end","id":"{}"}}"#, message_id));

    let mid_events = futures::stream::iter(
        payloads
            .into_iter()
            .map(|d| Ok::<Event, axum::Error>(Event::default().event("message").data(d))),
    );
    let start_stream =
        futures::stream::once(futures::future::ready(Ok::<Event, axum::Error>(start)));
    let end_stream = futures::stream::once(futures::future::ready(Ok::<Event, axum::Error>(end)));
    Sse::new(start_stream.chain(mid_events).chain(end_stream)).into_response()
}

/// 直接流式转发（无工具）：Agent 降级通道 & tools 关闭时的路径。
async fn direct_stream_response(
    client: &Client,
    cfg: &AiConfig,
    system_prompt: String,
    history: &[ChatMessage],
    persist_tx: tokio::sync::mpsc::Sender<String>,
) -> Response {
    #[derive(Debug, Serialize)]
    struct SimpleMessage {
        role: String,
        content: String,
    }
    #[derive(Debug, Serialize)]
    struct SimpleRequest {
        model: String,
        messages: Vec<SimpleMessage>,
        stream: bool,
    }
    let mut backend_messages = vec![SimpleMessage {
        role: "system".to_string(),
        content: system_prompt,
    }];
    for msg in history {
        backend_messages.push(SimpleMessage {
            role: msg.role.clone(),
            content: msg.get_content(),
        });
    }
    let body_json = match serde_json::to_string(&SimpleRequest {
        model: cfg.model.clone(),
        messages: backend_messages,
        stream: true,
    }) {
        Ok(j) => j,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to serialize request: {}", e),
            )
                .into_response();
        }
    };
    match client
        .post(&cfg.base_url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", cfg.api_key))
        .body(body_json)
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            if !status.is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                return (
                    StatusCode::BAD_REQUEST,
                    format!("AI API error: {}", error_text),
                )
                    .into_response();
            }
            let byte_stream =
                response
                    .bytes_stream()
                    .map(|res: Result<axum::body::Bytes, reqwest::Error>| {
                        res.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
                    });
            let reader = tokio_util::io::StreamReader::new(byte_stream);
            let lines = FramedRead::new(reader, LinesCodec::new());
            let message_id = format!("msg_{}", uuid::Uuid::new_v4().simple());
            let start_event =
                futures::stream::once(futures::future::ready(Ok::<Event, axum::Error>(
                    Event::default()
                        .event("message")
                        .data(format!(r#"{{"type":"text-start","id":"{}"}}"#, message_id)),
                )));
            let stream = lines.filter_map(
                move |result: Result<String, tokio_util::codec::LinesCodecError>| {
                    let tx = persist_tx.clone();
                    let message_id = message_id.clone();
                    futures::future::ready(match result {
                        Ok(line) => {
                            if line.is_empty() {
                                None
                            } else if let Some(data_str) = line.strip_prefix("data: ") {
                                let data_str = data_str.trim();
                                if data_str == "[DONE]" {
                                    Some(Ok::<Event, axum::Error>(
                                        Event::default().event("message").data(format!(
                                            r#"{{"type":"text-end","id":"{}"}}"#,
                                            message_id
                                        )),
                                    ))
                                } else {
                                    let mut output_event = None;
                                    if let Ok(chunk) =
                                        serde_json::from_str::<ChatCompletionChunk>(data_str)
                                    {
                                        if let Some(choice) = chunk.choices.first() {
                                            if let Some(content) = &choice.delta.content {
                                                let _ = tx.try_send(content.clone());
                                                let chunk_json = serde_json::json!({
                                                    "type": "text-delta",
                                                    "id": message_id,
                                                    "delta": content
                                                });
                                                output_event = Some(Ok::<Event, axum::Error>(
                                                    Event::default()
                                                        .event("message")
                                                        .data(chunk_json.to_string()),
                                                ));
                                            }
                                        }
                                    }
                                    output_event
                                }
                            } else {
                                None
                            }
                        }
                        Err(e) => {
                            eprintln!("Stream error: {}", e);
                            Some(Ok::<Event, axum::Error>(
                                Event::default()
                                    .event("message")
                                    .data(format!(r#"{{"type":"error","errorText":"{}"}}"#, e)),
                            ))
                        }
                    })
                },
            );
            Sse::new(start_event.chain(stream)).into_response()
        }
        Err(e) => {
            eprintln!("Failed to forward to AI API: {}", e);
            (
                StatusCode::BAD_GATEWAY,
                "Gateway error: failed to reach AI service",
            )
                .into_response()
        }
    }
}

fn spawn_persist_task(
    pool: Arc<sqlx::SqlitePool>,
    user_id: String,
    mut rx: tokio::sync::mpsc::Receiver<String>,
) {
    tokio::spawn(async move {
        let mut full_content = String::new();
        let timeout = tokio::time::timeout(std::time::Duration::from_secs(30), async {
            while let Some(content) = rx.recv().await {
                full_content.push_str(&content);
            }
        })
        .await;
        if !full_content.is_empty() {
            if let Err(e) =
                ai_dao::insert_message(&pool, &user_id, "assistant", &full_content).await
            {
                eprintln!("Failed to persist assistant message: {:?}", e);
            }
        } else if timeout.is_err() {
            eprintln!("⚠️  AI stream timed out after 30s with no content");
        }
    });
}

pub async fn chat_handler(State(state): State<Arc<AppState>>, req: Request) -> Response {
    // 限制请求体大小为 2MB，防止恶意大报文攻击 (Issue 1)
    let body_bytes = match axum::body::to_bytes(req.into_body(), 2 * 1024 * 1024).await {
        Ok(bytes) => bytes,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("Failed to read request body: {}", e),
            )
                .into_response();
        }
    };

    let chat_req: ChatRequest = match serde_json::from_slice(&body_bytes) {
        Ok(req) => req,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, format!("Invalid JSON: {}", e)).into_response();
        }
    };

    // 加载系统提示词 (Issue 4: 使用运行时相对路径)
    let system_prompt = load_system_prompt().await;

    // 持久化用户最后一条消息
    if let Some(last_msg) = chat_req.messages.last() {
        let pool = Arc::clone(&state.pool);
        let user_id = chat_req.user_id.clone();
        let role = last_msg.role.clone();
        let content = last_msg.get_content();
        tokio::spawn(async move {
            if let Err(e) = ai_dao::insert_message(&pool, &user_id, &role, &content).await {
                eprintln!("Failed to persist user message: {:?}", e);
            }
        });
    }

    // 后台持久化 AI 消息（带超时保护）
    let (tx, rx) = tokio::sync::mpsc::channel::<String>(100);
    spawn_persist_task(Arc::clone(&state.pool), chat_req.user_id.clone(), rx);

    // 实时加载模型配置：改 `.env` 后无需重启，下一个请求自动生效
    let ai_cfg = AiConfig::load_live();
    eprintln!(
        "AI chat: model={} base_url={} tools={}",
        ai_cfg.model, ai_cfg.base_url, ai_cfg.tools_enabled
    );

    // 工具关闭时直接走纯转发通道
    if !ai_cfg.tools_enabled {
        eprintln!("AI tools disabled, direct streaming");
        return direct_stream_response(
            &state.client,
            &ai_cfg,
            system_prompt,
            &chat_req.messages,
            tx,
        )
        .await;
    }

    // Agent 循环（工具轮非流式；异常时降级纯转发，保证聊天不挂）
    match run_agent_loop(&state.client, &ai_cfg, &system_prompt, &chat_req.messages).await {
        Ok((reads, final_content)) => {
            let message_id = format!("msg_{}", uuid::Uuid::new_v4().simple());
            agent_sse_response(&message_id, reads, final_content, tx)
        }
        Err(err) => {
            eprintln!("Agent loop failed ({}), fallback to direct streaming", err);
            direct_stream_response(
                &state.client,
                &ai_cfg,
                system_prompt,
                &chat_req.messages,
                tx,
            )
            .await
        }
    }
}
