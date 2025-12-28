use anyhow::Result;
use sqlx::{Row, SqlitePool};
use std::sync::Arc;
use tokio::time::Duration;

use crate::dao::blog;
use crate::util::email;

/// 启动前业务逻辑
pub async fn after_startup(pool: &Arc<SqlitePool>) -> Result<()> {
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

    // 启动定时清理任务
    let pool_for_cleanup = Arc::clone(pool);

    let cleanup_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(24 * 60 * 60));

        loop {
            interval.tick().await;
            if let Err(e) = clean_old_visits_task(&pool_for_cleanup).await {
                eprintln!("❌ 清理旧访问记录失败: {}", e);
            }
        }
    });

    // 监控定时清理任务的状态，但不使用unwrap_err()
    tokio::spawn(async move {
        let result = cleanup_handle.await;
        match result {
            Err(e) => {
                eprintln!("❌ 定时清理任务结束并返回错误: {:?}", e);
            }
            Ok(_) => {
                eprintln!("❌ 定时清理任务意外结束");
            }
        }
    });

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

/// 清理旧访问记录的任务
async fn clean_old_visits_task(pool: &Arc<SqlitePool>) -> Result<()> {
    // 执行清理
    blog::clean_old_visits(pool.as_ref()).await?;
    Ok(())
}
