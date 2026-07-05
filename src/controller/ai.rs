use axum::{
    extract::{Request, State},
    http::StatusCode,
    response::{IntoResponse, Response, Sse, sse::Event},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use crate::app_state::AppState;
use crate::dao::ai as ai_dao;
use tokio_util::codec::{FramedRead, LinesCodec};
use futures::StreamExt as _;

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

#[derive(Debug, Serialize)]
struct BackendMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct BackendRequest {
    model: String,
    messages: Vec<BackendMessage>,
    stream: bool,
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

/// 读取系统提示词文件，作为 system message
async fn load_system_prompt() -> String {
    // 优先 from 当前目录下的 data 文件夹读取，适配运行时路径
    let prompt_path = std::path::Path::new("data")
        .join("prompt")
        .join("secretary.md");

    match tokio::fs::read_to_string(&prompt_path).await {
        Ok(content) => content,
        Err(e) => {
            eprintln!("⚠️  Failed to load system prompt from {:?}: {}", prompt_path, e);
            "You are a helpful assistant.".to_string()
        }
    }
}

pub async fn chat_handler(
    State(state): State<Arc<AppState>>,
    req: Request
) -> Response {
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
            return (
                StatusCode::BAD_REQUEST,
                format!("Invalid JSON: {}", e),
            )
                .into_response();
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

    // 构建 AI 请求体（OpenAI 兼容格式）
    let mut backend_messages: Vec<BackendMessage> = vec![BackendMessage {
        role: "system".to_string(),
        content: system_prompt,
    }];

    for msg in &chat_req.messages {
        backend_messages.push(BackendMessage {
            role: msg.role.clone(),
            content: msg.get_content(),
        });
    }

    let backend_req = BackendRequest {
        model: state.ai_config.model.clone(), // 使用配置的环境变量
        messages: backend_messages,
        stream: true,
    };

    let body_json = match serde_json::to_string(&backend_req) {
        Ok(json) => json,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to serialize request: {}", e),
            )
                .into_response();
        }
    };

    // 转发到 AI API (Issue 2: 使用共享 Client)
    let url = &state.ai_config.base_url;

    match state.client
        .post(url)
        .header("Content-Type", "application/json")
        .header(
            "Authorization",
            format!("Bearer {}", state.ai_config.api_key), // 使用配置的环境变量 (Issue 5)
        )
        .body(body_json)
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();

            if !status.is_success() {
                let error_text = match response.text().await {
                    Ok(t) => t,
                    Err(_) => "Unknown error".to_string(),
                };
                return (StatusCode::BAD_REQUEST, format!("AI API error: {}", error_text))
                    .into_response();
            }

            // SSE 流式转发 (Issue 3: 使用 LinesCodec 稳健解析 SSE)
            let byte_stream = response.bytes_stream().map(|res: Result<axum::body::Bytes, reqwest::Error>| {
                res.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))
            });
            let reader = tokio_util::io::StreamReader::new(byte_stream);
            let lines = FramedRead::new(reader, LinesCodec::new());

            let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(100);

            // 后台持久化 AI 消息（带超时保护，防止客户端断开后永远等待）
            let pool = Arc::clone(&state.pool);
            let user_id = chat_req.user_id.clone();
            tokio::spawn(async move {
                let mut full_content = String::new();
                let timeout = tokio::time::timeout(
                    std::time::Duration::from_secs(30),
                    async {
                        while let Some(content) = rx.recv().await {
                            full_content.push_str(&content);
                        }
                    }
                ).await;
                if !full_content.is_empty() {
                    if let Err(e) = ai_dao::insert_message(&pool, &user_id, "assistant", &full_content).await {
                        eprintln!("Failed to persist assistant message: {:?}", e);
                    }
                } else if timeout.is_err() {
                    eprintln!("⚠️  AI stream timed out after 30s with no content");
                }
            });

            let message_id = format!("msg_{}", uuid::Uuid::new_v4().simple());
            
            let start_event = futures::stream::once(futures::future::ready(Ok::<Event, axum::Error>(
                Event::default().event("message").data(format!(r#"{{"type":"text-start","id":"{}"}}"#, message_id))
            )));

            let stream = lines.filter_map(move |result: Result<String, tokio_util::codec::LinesCodecError>| {
                let tx = tx.clone();
                let message_id = message_id.clone();
                futures::future::ready(match result {
                    Ok(line) => {
                        if line.is_empty() {
                            None
                        } else if let Some(data_str) = line.strip_prefix("data: ") {
                            let data_str = data_str.trim();
                            if data_str == "[DONE]" {
                                Some(Ok::<Event, axum::Error>(Event::default().event("message").data(format!(r#"{{"type":"text-end","id":"{}"}}"#, message_id))))
                            } else {
                                // 尝试解析内容并发送到持久化通道
                                let mut output_event = None;
                                if let Ok(chunk) = serde_json::from_str::<ChatCompletionChunk>(data_str) {
                                    if let Some(choice) = chunk.choices.first() {
                                        if let Some(content) = &choice.delta.content {
                                            let _ = tx.try_send(content.clone());
                                            
                                            // 转换为 ai-sdk 格式
                                            let chunk_json = serde_json::json!({
                                                "type": "text-delta",
                                                "id": message_id,
                                                "delta": content
                                            });
                                            output_event = Some(Ok::<Event, axum::Error>(Event::default().event("message").data(chunk_json.to_string())));
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
                        Some(Ok::<Event, axum::Error>(Event::default().event("message").data(format!(r#"{{"type":"error","errorText":"{}"}}"#, e))))
                    }
                })
            });

            Sse::new(start_event.chain(stream)).into_response()
        }
        Err(e) => {
            eprintln!("Failed to forward to AI API: {}", e);
            (StatusCode::BAD_GATEWAY, "Gateway error: failed to reach AI service").into_response()
        }
    }
}
