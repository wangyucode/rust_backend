use actix_web::{HttpResponse, Responder, web};
use lettre::message::header::ContentType;
use lettre::{Message, SmtpTransport, Transport};
use serde::Deserialize;
use std::env;

use super::ApiResponse;

#[derive(Deserialize)]
pub struct EmailRequest {
    key: String,
    subject: Option<String>,
    content: String,
    to: Option<String>,
}

pub async fn send_email(req: web::Json<EmailRequest>) -> impl Responder {
    // 验证content是否存在
    if req.content.is_empty() {
        return HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error("content is required".to_string()));
    }

    // 验证key是否正确
    let mail_password = env::var("MAIL_PASSWORD").unwrap_or_default();
    println!("mail_password: {}", mail_password);
    if req.key != mail_password {
        return HttpResponse::Forbidden().json(ApiResponse::<()>::error("invalid key".to_string()));
    }

    // 设置默认值
    let subject = req
        .subject
        .clone()
        .unwrap_or("【Rust】后端推送".to_string());
    let to_email = req.to.clone().unwrap_or("wangyu@wycode.cn".to_string());
    let from_email = "wayne001@vip.qq.com".to_string();

    // 创建邮件
    let email = match Message::builder()
        .from(from_email.parse().unwrap())
        .to(to_email.parse().unwrap())
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(req.content.clone())
    {
        Ok(email) => email,
        Err(e) => {
            eprintln!("Error creating email: {:?}", e);
            return HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("error creating email".to_string()));
        }
    };

    // 配置SMTP服务器
    let smtp_server = env::var("SMTP_SERVER").unwrap_or("smtp.qq.com".to_string());
    let smtp_port = env::var("SMTP_PORT")
        .unwrap_or("465".to_string())
        .parse::<u16>()
        .unwrap_or(465);
    let smtp_user = from_email.clone();
    let smtp_pass = mail_password;

    let mailer = SmtpTransport::relay(&smtp_server)
        .unwrap()
        .credentials(lettre::transport::smtp::authentication::Credentials::new(
            smtp_user, smtp_pass,
        ))
        .port(smtp_port)
        .build();

    // 发送邮件
    match mailer.send(&email) {
        Ok(_) => {
            println!("Sent email to: {}", to_email);
            HttpResponse::Ok().json(ApiResponse::<()>::message_success("ok".to_string()))
        }
        Err(e) => {
            eprintln!("Error sending email: {:?}", e);
            HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("error sending email".to_string()))
        }
    }
}
