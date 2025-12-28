use axum::{
    extract::Json as AxumJson,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use std::env;

use super::ApiResponse;
use crate::util::email::{EmailConfig, send_email};

#[derive(Deserialize)]
pub struct EmailRequest {
    key: String,
    subject: Option<String>,
    content: String,
    to: Option<String>,
}

pub async fn send_email_handler(AxumJson(req): AxumJson<EmailRequest>) -> impl IntoResponse {
    // 验证content是否存在
    if req.content.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("content is required".to_string())),
        )
            .into_response();
    }

    // 验证key是否正确
    let mail_password = env::var("MAIL_PASSWORD").unwrap_or_default();
    if req.key != mail_password {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::<()>::error("invalid key".to_string())),
        )
            .into_response();
    }

    // 创建邮件配置
    let config = EmailConfig::new(req.subject.clone(), req.content.clone(), req.to.clone());

    // 发送邮件
    match send_email(config).await {
        Ok(_) => Json(ApiResponse::<()>::message_success("ok".to_string())).into_response(),
        Err(e) => {
            eprintln!("Error sending email: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(e.to_string())),
            )
                .into_response()
        }
    }
}
