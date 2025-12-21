use lettre::message::header::ContentType;
use lettre::{Message, SmtpTransport, Transport};
use std::env;
use std::result::Result;

pub struct EmailConfig {
    pub subject: String,
    pub content: String,
    pub to: String,
    pub from: String,
}

impl Default for EmailConfig {
    fn default() -> Self {
        Self {
            subject: "【Rust】后端推送".to_string(),
            content: "".to_string(),
            to: "wangyu@wycode.cn".to_string(),
            from: "wayne001@vip.qq.com".to_string(),
        }
    }
}

impl EmailConfig {
    pub fn new(subject: Option<String>, content: String, to: Option<String>) -> Self {
        Self {
            subject: subject.unwrap_or("【Rust】后端推送".to_string()),
            content,
            to: to.unwrap_or("wangyu@wycode.cn".to_string()),
            from: "wayne001@vip.qq.com".to_string(),
        }
    }
}

pub fn send_email(config: EmailConfig) -> Result<(), String> {
    if config.content.is_empty() {
        return Err("content is required".to_string());
    }

    let mail_password = env::var("MAIL_PASSWORD").unwrap_or_default();
    let smtp_server = env::var("SMTP_SERVER").unwrap_or("smtp.qq.com".to_string());
    let smtp_port = env::var("SMTP_PORT")
        .unwrap_or("465".to_string())
        .parse::<u16>()
        .unwrap_or(465);

    // 开发环境判断：如果不是生产环境，则只打印日志不发送邮件
    if env::var("APP_ENV").unwrap_or_default() != "production" {
        println!("[开发环境] 邮件发送模拟:");
        println!("From: {}", config.from);
        println!("To: {}", config.to);
        println!("Subject: {}", config.subject);
        println!("[开发环境] 邮件发送模拟完成");
        return Ok(());
    }

    let email = Message::builder()
        .from(
            config
                .from
                .parse()
                .map_err(|e| format!("invalid from email: {:?}", e))?,
        )
        .to(config
            .to
            .parse()
            .map_err(|e| format!("invalid to email: {:?}", e))?)
        .subject(config.subject)
        .header(ContentType::TEXT_PLAIN)
        .body(config.content)
        .map_err(|e| format!("error creating email: {:?}", e))?;

    let mailer = SmtpTransport::relay(&smtp_server)
        .map_err(|e| format!("error creating smtp transport: {:?}", e))?
        .credentials(lettre::transport::smtp::authentication::Credentials::new(
            config.from.clone(),
            mail_password,
        ))
        .port(smtp_port)
        .build();

    mailer
        .send(&email)
        .map_err(|e| format!("error sending email: {:?}", e))?;

    println!("Sent email to: {}", config.to);
    Ok(())
}
