use anyhow::Result;
use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use std::env;
use std::sync::Arc;

/// 初始化数据库连接池 + 执行迁移
pub async fn init_database_pool() -> Result<Arc<SqlitePool>> {
    // 从环境变量读取数据库URL
    let db_url =
        env::var("DATABASE_URL").map_err(|_| anyhow::anyhow!("DATABASE_URL 环境变量未设置"))?;
    println!("📁 数据库连接URL: {}", db_url);

    // 创建连接池
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&db_url)
        .await?;
    println!("✅ 数据库连接池初始化成功");

    // 执行迁移：加载migrations目录下的所有未执行脚本
    sqlx::migrate!("./db/migrations").run(&pool).await?;
    println!("✅ 数据库迁移执行成功");

    Ok(Arc::new(pool))
}
