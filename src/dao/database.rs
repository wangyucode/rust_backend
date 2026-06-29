use anyhow::Result;
use sqlx::{
    migrate::Migrator,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
    SqlitePool,
};
use std::path::Path;
use std::sync::Arc;

const MAIN_DB_FILE: &str = "./db/sqlite.db";
const LOG_DB_FILE: &str = "./db/access_log.db";

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

async fn run_caddy_log_schema(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS caddy_file_state (
            file_name TEXT PRIMARY KEY,
            file_id TEXT,
            offset INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS caddy_access_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            ts REAL NOT NULL,
            level TEXT,
            logger TEXT,
            msg TEXT,
            remote_ip TEXT,
            remote_port INTEGER,
            client_ip TEXT,
            proto TEXT,
            method TEXT,
            host TEXT,
            uri TEXT,
            req_headers TEXT,
            tls TEXT,
            bytes_read INTEGER,
            user_id TEXT,
            duration REAL,
            size INTEGER,
            status INTEGER,
            resp_headers TEXT,
            domain TEXT
        )
        "#,
    )
    .execute(pool)
    .await?;

    // 创建索引
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_caddy_access_log_ts ON caddy_access_log (ts)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_caddy_access_log_domain ON caddy_access_log (domain)")
        .execute(pool)
        .await?;

    Ok(())
}

/// 初始化主数据库连接池 + 执行主库迁移
pub async fn init_database_pool() -> Result<Arc<SqlitePool>> {
    let pool = init_pool(MAIN_DB_FILE, 4).await?;

    let migrations_dir = Path::new("./db/migrations");
    if migrations_dir.exists() {
        let migrator = Migrator::new(migrations_dir).await?;
        migrator.run(&pool).await?;
    }

    Ok(Arc::new(pool))
}

/// 初始化 Caddy 日志数据库连接池
pub async fn init_log_database_pool() -> Result<Arc<SqlitePool>> {
    let pool = init_pool(LOG_DB_FILE, 2).await?;
    run_caddy_log_schema(&pool).await?;
    Ok(Arc::new(pool))
}
