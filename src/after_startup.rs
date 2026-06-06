use anyhow::Result;
use sqlx::{Row, SqlitePool};
use std::sync::Arc;

use crate::task::{caddy, daily};
use crate::util::email;

/// 启动前业务逻辑
pub async fn after_startup(pool: &Arc<SqlitePool>) -> Result<()> {
    // 启动每日任务
    daily::start_daily_tasks(Arc::clone(pool));

    // 打印数据库表和数据量
    let tables = sqlx::query("SELECT name FROM sqlite_master WHERE type='table'")
        .fetch_all(pool.as_ref())
        .await?;
    let mut tables_info = String::new();
    for table in tables {
        let table_name: String = table.get(0);
        let row_count =
            sqlx::query_scalar::<_, i64>(&format!("SELECT COUNT(*) FROM {}", table_name))
                .fetch_one(pool.as_ref())
                .await?;
        let table_info = format!("表：{} 共 {} 条数据\n", table_name, row_count);
        tables_info.push_str(&table_info);
    }

    // 启动 Caddy 日志导入任务 (5s)
    caddy::start_caddy_log_task(Arc::clone(pool));

    // 发送启动通知邮件
    let start_notification = format!(
        "Rust后端服务已成功启动！\n\n版本：{}\n\n数据库表信息：\n{}",
        env!("CARGO_PKG_VERSION"),
        tables_info
    );
    let email_config = email::EmailConfig::new(
        Some("【Rust】后端服务启动通知".to_string()),
        start_notification,
        None,
    );

    match email::send_email(email_config).await {
        Ok(_) => {
            println!("✅ 已发送启动通知邮件");
        }
        Err(e) => {
            eprintln!("❌ 发送启动通知邮件失败：{}", e);
        }
    }

    Ok(())
}
