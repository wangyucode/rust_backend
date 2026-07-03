use anyhow::Result;
use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    SqlitePool,
};
use std::path::Path;
use std::sync::Arc;

const MAIN_DB_FILE: &str = "./data/db/sqlite.db";
const LOG_DB_FILE: &str = "./data/db/access_log.db";

async fn init_pool(db_file: &str, max_connections: u32) -> Result<SqlitePool> {
    // 确保父目录存在
    if let Some(parent) = Path::new(db_file).parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let options = SqliteConnectOptions::new()
        .filename(db_file)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .foreign_keys(true)
        .create_if_missing(true);

    let pool = SqlitePoolOptions::new()
        .max_connections(max_connections)
        .connect_with(options)
        .await?;

    Ok(pool)
}

/// 初始化主数据库连接池 + 执行主库迁移
pub async fn init_database_pool() -> Result<Arc<SqlitePool>> {
    let pool = init_pool(MAIN_DB_FILE, 4).await?;

    let migrations_dir = Path::new("./data/migrations");
    if migrations_dir.exists() {
        let migrator = Migrator::new(migrations_dir).await?;
        migrator.run(&pool).await?;
    }

    Ok(Arc::new(pool))
}

/// 初始化 Caddy 日志数据库连接池
pub async fn init_log_database_pool() -> Result<Arc<SqlitePool>> {
    let pool = init_pool(LOG_DB_FILE, 2).await?;
    
    let migrations_dir = Path::new("./data/migrations_log");
    if migrations_dir.exists() {
        let migrator = Migrator::new(migrations_dir).await?;
        migrator.run(&pool).await?;
    }
    
    Ok(Arc::new(pool))
}
