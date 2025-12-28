use chrono::Utc;
use lazy_static::lazy_static;
use lettre::Tokio1Executor;
use lettre::message::header::ContentType;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::env;
use std::result::Result;
use std::sync::Mutex;

// 全局缓存，存储邮件内容哈希和发送时间戳（秒）
lazy_static! {
    static ref EMAIL_CACHE: Mutex<HashMap<String, i64>> = Mutex::new(HashMap::new());
}

// 生成邮件主题和内容的SHA256哈希值
fn generate_email_hash(subject: &str, content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(subject.as_bytes());
    hasher.update(b"|"); // 使用分隔符确保不同组合的唯一性
    hasher.update(content.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

// 清理过期的缓存条目（超过1小时的条目将被清理）
fn cleanup_cache() -> Result<(), String> {
    let now = Utc::now().timestamp();
    let cleanup_threshold = 3600; // 清理超过1小时的缓存

    let mut cache = EMAIL_CACHE
        .lock()
        .map_err(|_| "Failed to lock cache".to_string())?;

    // 过滤出未过期的条目
    let valid_entries: HashMap<String, i64> = cache
        .drain()
        .filter(|(_, timestamp)| now - timestamp < cleanup_threshold)
        .collect();

    // 将未过期的条目放回缓存
    cache.extend(valid_entries);

    Ok(())
}

#[derive(Clone)]
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

pub async fn send_email(config: EmailConfig) -> Result<(), String> {
    if config.content.is_empty() {
        return Err("content is required".to_string());
    }

    let now = Utc::now().timestamp();
    let throttle_duration = 60; // 1分钟节流

    // 清理过期缓存
    let _ = cleanup_cache();

    // 生成邮件主题和内容的哈希
    let email_hash = generate_email_hash(&config.subject, &config.content);

    // 检查缓存
    {
        let mut cache = EMAIL_CACHE
            .lock()
            .map_err(|_| "Failed to lock cache".to_string())?;
        if let Some(last_sent) = cache.get(&email_hash) {
            if now - last_sent < throttle_duration {
                println!(
                    "[节流] 邮件内容在{}秒内已发送，跳过本次发送",
                    throttle_duration
                );
                return Ok(());
            }
        }

        // 更新缓存
        cache.insert(email_hash.clone(), now);
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

    // 生产环境下检查必要的配置
    if mail_password.is_empty() {
        return Err("MAIL_PASSWORD 环境变量未设置".to_string());
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

    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_server)
        .map_err(|e| format!("error creating smtp transport: {:?}", e))?
        .credentials(lettre::transport::smtp::authentication::Credentials::new(
            config.from.clone(),
            mail_password,
        ))
        .port(smtp_port)
        .build();

    mailer
        .send(email)
        .await
        .map_err(|e| format!("error sending email: {:?}", e))?;

    println!("Sent email to: {}", config.to);
    Ok(())
}
