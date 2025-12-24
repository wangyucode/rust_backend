use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

// 热门文章数据结构
#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
pub struct PopularPost {
    pub id: String,
    pub view_count: i64,
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

// 获取热门文章
pub async fn get_popular_posts(
    pool: &SqlitePool,
    days: i64,
    limit: i64,
) -> Result<Vec<PopularPost>, sqlx::Error> {
    // 计算指定天数前的时间戳（毫秒）
    let cutoff_time = chrono::Utc::now().timestamp_millis() - (days * 24 * 60 * 60 * 1000);

    // 查询热门文章
    let popular_posts = sqlx::query_as(
        "SELECT id, COUNT(id) as view_count FROM blog_visits WHERE timestamp >= ? GROUP BY id ORDER BY COUNT(id) DESC LIMIT ?"
    )
    .bind(cutoff_time)
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(popular_posts)
}

// 清理超过30天的访问记录
pub async fn clean_old_visits(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    // 首先检查表是否存在
    let table_exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='blog_visits'"
    )
    .fetch_one(pool)
    .await?;

    if table_exists == 0 {
        // 表不存在，直接返回
        return Ok(());
    }

    // 计算30天前的时间戳（毫秒）
    let cutoff_time = chrono::Utc::now().timestamp_millis() - (30 * 24 * 60 * 60 * 1000);

    // 删除超过30天的记录
    sqlx::query("DELETE FROM blog_visits WHERE timestamp < ?")
        .bind(cutoff_time)
        .execute(pool)
        .await?;

    Ok(())
}
