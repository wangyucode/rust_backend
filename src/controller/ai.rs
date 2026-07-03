use axum::{
    extract::Request,
    http::StatusCode,
    response::{IntoResponse, Response, Sse},
};
use serde::{Deserialize, Serialize};
use tokio_stream::StreamExt;

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
struct VolcengineMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct VolcengineRequest {
    model: String,
    messages: Vec<VolcengineMessage>,
    stream: bool,
}

/// 读取系统提示词文件，作为 system message
async fn load_system_prompt() -> String {
    let prompt_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("data")
        .join("prompt")
        .join("secretary.md");

    tokio::fs::read_to_string(&prompt_path)
        .await
        .unwrap_or_else(|_| "You are a helpful assistant.".to_string())
}

pub async fn chat_handler(req: Request) -> Response {
    // 读取请求体中的 messages
    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
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

    // 加载系统提示词
    let system_prompt = load_system_prompt().await;

    // 构建火山引擎请求体（OpenAI 兼容格式）
    let mut volc_messages: Vec<VolcengineMessage> = vec![VolcengineMessage {
        role: "system".to_string(),
        content: system_prompt,
    }];

    for msg in &chat_req.messages {
        volc_messages.push(VolcengineMessage {
            role: msg.role.clone(),
            content: msg.content.clone(),
        });
    }

    let volc_req = VolcengineRequest {
        model: "doubao-seed-2-0-mini-260428".to_string(),
        messages: volc_messages,
        stream: true,
    };

    let body_json = match serde_json::to_string(&volc_req) {
        Ok(json) => json,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to serialize request: {}", e),
            )
                .into_response();
        }
    };

    // 转发到火山引擎 API
    let client = reqwest::Client::new();
    let url = "https://ark.cn-beijing.volces.com/api/v3/chat/completions";

    let api_key = std::env::var("DOUBAO_API_KEY").unwrap_or_else(|_| "".to_string());

    match client
        .post(url)
        .header("Content-Type", "application/json")
        .header(
            "Authorization",
            format!("Bearer {}", api_key),
        )
        .body(body_json)
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();

            if !status.is_success() {
                // 模型错误返回 4xx
                let error_text = match response.text().await {
                    Ok(t) => t,
                    Err(_) => "Unknown error".to_string(),
                };
                return (StatusCode::BAD_REQUEST, format!("Volcengine API error: {}", error_text))
                    .into_response();
            }

            // SSE 流式转发：逐 chunk 转发 Volcengine stream response
            let stream = response.bytes_stream().map(|result| {
                match result {
                    Ok(bytes) => {
                        // 解析火山引擎的 SSE 格式: "data: {...}\n\n"
                        let text = String::from_utf8_lossy(&bytes);
                        for line in text.lines() {
                            if let Some(data_str) = line.strip_prefix("data: ") {
                                // 检查是否是 [DONE]
                                if data_str.trim() == "[DONE]" {
                                    return Ok::<axum::response::sse::Event, axum::Error>(
                                        axum::response::sse::Event::default()
                                            .event("message")
                                            .data("[DONE]")
                                            .clone(),
                                    );
                                }

                                // 转发为 SSE event，统一用 "message" 作为 event name
                                let event = axum::response::sse::Event::default()
                                    .event("message")
                                    .data(data_str)
                                    .clone();
                                return Ok(event);
                            }
                        }
                        // 没有匹配到 data: 的行，跳过
                        Ok(axum::response::sse::Event::default().clone())
                    }
                    Err(e) => {
                        eprintln!("Stream error: {}", e);
                        Ok::<axum::response::sse::Event, axum::Error>(
                            axum::response::sse::Event::default()
                                .event("message")
                                .data("[ERROR]")
                                .clone(),
                        )
                    }
                }
            });

            Sse::new(stream).into_response()
        }
        Err(e) => {
            // 网络超时返回 502
            eprintln!("Failed to forward to Volcengine: {}", e);
            (StatusCode::BAD_GATEWAY, "Gateway error: failed to reach AI service").into_response()
        }
    }
}
