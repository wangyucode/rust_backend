use anyhow::Result;
use chrono;
use sqlx::{Row, SqlitePool};

use crate::util::email;

/// 启动前业务逻辑
pub async fn after_startup(pool: &SqlitePool) -> Result<()> {
    // 打印数据库表和数据量
    let tables = sqlx::query("SELECT name FROM sqlite_master WHERE type='table'")
        .fetch_all(pool)
        .await?;
    let mut tables_info = String::new();
    for table in tables {
        let table_name: String = table.get(0);
        let row_count =
            sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {}", table_name))
                .fetch_one(pool)
                .await?;
        let table_info = format!("表：{} 共 {} 条数据\n", table_name, row_count);
        println!("🗂️ {}", table_info.trim());
        tables_info.push_str(&table_info);
    }

    println!("\n🚀 服务启动成功");

    // 发送启动通知邮件
    let start_notification = format!(
        "Rust后端服务已成功启动！\n\n时间：{}\n版本：{}\n\n数据库表信息：\n{}",
        chrono::Local::now().to_string(),
        env!("CARGO_PKG_VERSION"),
        tables_info
    );
    let email_config = email::EmailConfig::new(
        Some("【Rust】后端服务启动通知".to_string()),
        start_notification,
        None,
    );

    if let Err(e) = email::send_email(email_config) {
        eprintln!("发送启动通知邮件失败：{}", e);
    } else {
        println!("✅ 已发送启动通知邮件");
    }

    Ok(())
}
