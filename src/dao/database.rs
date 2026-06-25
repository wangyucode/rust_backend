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

// =============================================================================
// LEGACY MIGRATION: 计划在 2026-07 之后移除
// =============================================================================
/// 一次性逻辑：将旧版存储在主库中的日志数据搬迁到独立的日志数据库
async fn migrate_legacy_logs_to_standalone(main_pool: &SqlitePool) -> Result<()> {
    // 1. 检查主库中是否存在旧表数据（如果表不存在或没数据，则跳过）
    let old_data_exists: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='caddy_file_state'",
    )
    .fetch_one(main_pool)
    .await?;

    if old_data_exists.0 == 0 {
        return Ok(());
    }

    println!("🔄 [Legacy Migration] 检测到旧版日志数据，正在搬迁至独立数据库...");

    // 使用直接连接执行 ATTACH (SQLite 不允许在事务中执行 ATTACH)
    let mut conn = main_pool.acquire().await?;

    sqlx::query(&format!("ATTACH DATABASE '{}' AS log_db", LOG_DB_FILE))
        .execute(&mut *conn)
        .await?;

    // 搬迁状态数据
    sqlx::query(
        "INSERT OR IGNORE INTO log_db.caddy_file_state SELECT * FROM main.caddy_file_state",
    )
    .execute(&mut *conn)
    .await?;

    // 搬迁日志数据（如果有 caddy_access_log 表）
    let has_log_table: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='caddy_access_log'",
    )
    .fetch_one(main_pool)
    .await?;

    if has_log_table.0 > 0 {
        sqlx::query(
            "INSERT OR IGNORE INTO log_db.caddy_access_log SELECT * FROM main.caddy_access_log",
        )
        .execute(&mut *conn)
        .await?;
    }

    sqlx::query("DETACH DATABASE log_db")
        .execute(&mut *conn)
        .await?;

    println!("✅ [Legacy Migration] 数据搬迁完成。旧表将由 SQL 迁移脚本清理。");
    Ok(())
}
// =============================================================================

/// 初始化主数据库连接池 + 执行主库迁移
pub async fn init_database_pool() -> Result<Arc<SqlitePool>> {
    // 确保日志数据库文件存在
    if !Path::new(LOG_DB_FILE).exists() {
        if let Some(parent) = Path::new(LOG_DB_FILE).parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        std::fs::File::create(LOG_DB_FILE)?;
    }

    let pool = init_pool(MAIN_DB_FILE, 4).await?;

    // 执行一次性搬迁逻辑 (必须在 SQL 迁移脚本 DROP 表之前运行)
    if let Err(e) = migrate_legacy_logs_to_standalone(&pool).await {
        eprintln!("⚠️ [Legacy Migration] 日志搬迁失败: {:?}", e);
    }

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
