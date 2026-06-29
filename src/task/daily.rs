use crate::dao::blog;
use crate::task::caddy;
use chrono::Local;
use sqlx::SqlitePool;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

/// 启动每日任务
pub fn start_daily_tasks(main_pool: Arc<SqlitePool>, log_pool: Arc<SqlitePool>) {
    tokio::spawn(async move {
        // tokio::time::interval 默认会在第一次 tick().await 时立即返回
        let mut interval = time::interval(Duration::from_secs(24 * 60 * 60));
        loop {
            interval.tick().await;

            // 1. 备份数据库 (第一步)
            if let Err(e) = backup_database(&main_pool).await {
                eprintln!("❌ 数据库备份失败: {:?}", e);
            }

            // 2. 清理旧备份 (7天前)
            if let Err(e) = clean_old_backups() {
                eprintln!("❌ 清理旧备份失败: {:?}", e);
            }

            // 3. 清理访问记录 (30天前)
            if let Err(e) = blog::clean_old_visits(&main_pool).await {
                eprintln!("❌ 清理旧访问记录失败: {:?}", e);
            }

            // 4. 清理 Caddy 旧日志 (7天前)
            if let Err(e) = caddy::clean_old_logs(&log_pool).await {
                eprintln!("❌ Caddy旧日志清理失败: {:?}", e);
            }
        }
    });
}

/// 备份 SQLite 数据库
async fn backup_database(pool: &SqlitePool) -> anyhow::Result<()> {
    let backup_dir = "./data/backups";
    if !Path::new(backup_dir).exists() {
        fs::create_dir_all(backup_dir)?;
    }

    let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_path = format!("{}/sqlite_backup_{}.db", backup_dir, timestamp);

    // 使用 VACUUM INTO 进行安全在线备份
    sqlx::query(&format!("VACUUM INTO '{}'", backup_path))
        .execute(pool)
        .await?;

    println!("✅ 数据库已备份至: {}", backup_path);
    Ok(())
}

/// 清理旧备份，仅保留最近 7 天的数据库备份
fn clean_old_backups() -> anyhow::Result<()> {
    let backup_dir = "./data/backups";
    if !Path::new(backup_dir).exists() {
        return Ok(());
    }

    let entries = fs::read_dir(backup_dir)?;
    let now = Local::now().timestamp();
    let retention_secs = 7 * 24 * 60 * 60;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let metadata = fs::metadata(&path)?;
            let modified = metadata
                .modified()?
                .duration_since(std::time::UNIX_EPOCH)?
                .as_secs();
            if (now as u64) - modified > retention_secs {
                fs::remove_file(&path)?;
                println!("🗑️ 已清理旧备份文件: {:?}", path);
            }
        }
    }

    Ok(())
}
