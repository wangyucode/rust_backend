use sqlx::{SqlitePool, sqlite::SqlitePoolOptions};
use std::sync::Arc;

pub async fn init_database_pool() -> Result<Arc<SqlitePool>, sqlx::Error> {
    // 创建SQLite数据库连接池
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite::memory:")
        .await?;

    // 创建apps表（如果不存在）
    sqlx::query("CREATE TABLE IF NOT EXISTS apps (appid TEXT PRIMARY KEY, name TEXT NOT NULL, image TEXT NOT NULL, description TEXT NOT NULL)")
    .execute(&pool)
    .await?;

    // 插入一些测试数据（如果表为空）
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM apps")
        .fetch_one(&pool)
        .await?;

    if count.0 == 0 {
        sqlx::query("INSERT INTO apps (appid, name, image, description) VALUES (?, ?, ?, ?), (?, ?, ?, ?), (?, ?, ?, ?)")
            .bind("app1")
            .bind("应用1")
            .bind("https://example.com/image1.jpg")
            .bind("这是应用1的简介")
            .bind("app2")
            .bind("应用2")
            .bind("https://example.com/image2.jpg")
            .bind("这是应用2的简介")
            .bind("app3")
            .bind("应用3")
            .bind("https://example.com/image3.jpg")
            .bind("这是应用3的简介")
            .execute(&pool)
            .await?;
    }

    Ok(Arc::new(pool))
}
