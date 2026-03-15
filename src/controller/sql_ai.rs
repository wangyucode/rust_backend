use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
    Json as AxumJson,
};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::env;
use std::sync::Arc;

use super::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct SqlCorrectionRequest {
    pub sql: String,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SqlCorrectionResponse {
    pub original_sql: String,
    pub corrected_sql: String,
    pub explanation: String,
}

// OpenAI-compatible API types
#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

/// 获取数据库的schema信息
async fn get_database_schema(pool: &SqlitePool) -> Result<String, sqlx::Error> {
    let tables = sqlx::query("SELECT name, sql FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_sqlx_%'")
        .fetch_all(pool)
        .await?;

    let mut schema = String::new();
    for table in tables {
        let sql: String = table.get("sql");
        schema.push_str(&sql);
        schema.push_str(";\n\n");
    }

    Ok(schema)
}

/// AI纠正SQL的处理函数
pub async fn correct_sql(
    State(pool): State<Arc<SqlitePool>>,
    AxumJson(body): AxumJson<SqlCorrectionRequest>,
) -> impl IntoResponse {
    // 验证输入
    if body.sql.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("sql不能为空".to_string())),
        )
            .into_response();
    }

    // 获取数据库schema
    let schema = match get_database_schema(pool.as_ref()).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to get database schema: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    "获取数据库schema失败".to_string(),
                )),
            )
                .into_response();
        }
    };

    // 读取环境变量
    let api_base = env::var("OPENAI_API_BASE")
        .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
    let api_key = match env::var("OPENAI_API_KEY") {
        Ok(key) => key,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    "未配置OPENAI_API_KEY".to_string(),
                )),
            )
                .into_response();
        }
    };
    let model = env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());

    // 构建prompt
    let mut user_content = format!(
        "以下是SQLite数据库的schema:\n```sql\n{}\n```\n\n请纠正以下SQL语句:\n```sql\n{}\n```",
        schema, body.sql
    );

    if let Some(ref error) = body.error {
        user_content.push_str(&format!("\n\n执行时的错误信息:\n```\n{}\n```", error));
    }

    let system_prompt = "你是一个SQLite SQL专家。用户会给你一个可能有语法错误或逻辑错误的SQL语句，以及数据库的schema信息。\n\
        请分析并纠正SQL语句。\n\n\
        你必须严格按照以下JSON格式返回，不要包含其他内容:\n\
        {\"corrected_sql\": \"纠正后的SQL语句\", \"explanation\": \"简短的修改说明\"}\n\n\
        注意:\n\
        - 如果SQL本身没有问题，corrected_sql返回原SQL，explanation说明无需修改\n\
        - 只返回JSON，不要有markdown代码块或其他文字";

    let chat_request = ChatRequest {
        model,
        messages: vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_content,
            },
        ],
        temperature: 0.0,
    };

    // 调用AI API
    let client = Client::new();
    let api_url = format!("{}/chat/completions", api_base.trim_end_matches('/'));

    let response = match client
        .post(&api_url)
        .bearer_auth(&api_key)
        .json(&chat_request)
        .send()
        .await
    {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Failed to call AI API: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    format!("调用AI API失败: {}", e),
                )),
            )
                .into_response();
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().await.unwrap_or_default();
        eprintln!("AI API error: {} - {}", status, body_text);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(
                format!("AI API返回错误: {}", status),
            )),
        )
            .into_response();
    }

    let chat_response: ChatResponse = match response.json().await {
        Ok(resp) => resp,
        Err(e) => {
            eprintln!("Failed to parse AI response: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    "解析AI响应失败".to_string(),
                )),
            )
                .into_response();
        }
    };

    let ai_content = match chat_response.choices.first() {
        Some(choice) => &choice.message.content,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    "AI未返回有效响应".to_string(),
                )),
            )
                .into_response();
        }
    };

    // 解析AI返回的JSON
    // 尝试清理可能的markdown代码块
    let cleaned = ai_content
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    #[derive(Debug, Deserialize)]
    struct AiResult {
        corrected_sql: String,
        explanation: String,
    }

    let ai_result: AiResult = match serde_json::from_str(cleaned) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("Failed to parse AI JSON result: {:?}, raw: {}", e, ai_content);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    "解析AI返回结果失败".to_string(),
                )),
            )
                .into_response();
        }
    };

    let result = SqlCorrectionResponse {
        original_sql: body.sql,
        corrected_sql: ai_result.corrected_sql,
        explanation: ai_result.explanation,
    };

    Json(ApiResponse::data_success(result)).into_response()
}
