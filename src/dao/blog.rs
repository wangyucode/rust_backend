use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

// BlogVisit数据结构
#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct BlogVisit {
    pub id: String,
    pub timestamp: i64,
}

// 记录或更新博客访问
pub async fn record_blog_visit(
    pool: &SqlitePool,
    id: &str,
    timestamp: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO blog_visits (id, timestamp) VALUES (?, ?) ON CONFLICT(id) DO UPDATE SET timestamp = ?"
    )
    .bind(id)
    .bind(timestamp)
    .bind(timestamp)
    .execute(pool)
    .await?;

    Ok(())
}
