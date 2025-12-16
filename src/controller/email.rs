use actix_web::{HttpResponse, Responder, web};
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

pub async fn send_email_handler(req: web::Json<EmailRequest>) -> impl Responder {
    // 验证content是否存在
    if req.content.is_empty() {
        return HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error("content is required".to_string()));
    }

    // 验证key是否正确
    let mail_password = env::var("MAIL_PASSWORD").unwrap_or_default();
    if req.key != mail_password {
        return HttpResponse::Forbidden().json(ApiResponse::<()>::error("invalid key".to_string()));
    }

    // 创建邮件配置
    let config = EmailConfig::new(req.subject.clone(), req.content.clone(), req.to.clone());

    // 发送邮件
    match send_email(config) {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::<()>::message_success("ok".to_string())),
        Err(e) => {
            eprintln!("Error sending email: {:?}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(e))
        }
    }
}
