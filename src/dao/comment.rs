use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

// Comment数据结构
#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub app: String,
    pub topic: String,
    pub content: String,
    pub create_time: i64,
    pub user: String,
    pub like: i64,
    pub to_user: Option<String>,
    pub to_content: Option<String>,
}

// To响应数据结构
#[derive(Debug, Serialize, Deserialize)]
pub struct ToResponse {
    pub content: String,
    pub user: String,
}

// Comment响应数据结构
#[derive(Debug, Serialize, Deserialize)]
pub struct CommentResponse {
    #[serde(rename = "_id")]
    pub id: String,
    pub content: String,
    pub user: String,
    pub like: i64,
    #[serde(rename = "createTime")]
    pub create_time: String,
    pub to: Option<ToResponse>,
}

// 验证app和key是否匹配
pub async fn validate_app_key(
    pool: &SqlitePool,
    app_id: &str,
    key: &str,
) -> Result<bool, sqlx::Error> {
    let result =
        sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM comment_apps WHERE id = ? AND key = ?")
            .bind(app_id)
            .bind(key)
            .fetch_one(pool)
            .await?;

    Ok(result > 0)
}

// 根据app和topic获取评论列表
pub async fn get_comments_by_app_topic(
    pool: &SqlitePool,
    app_id: &str,
    topic: &str,
) -> Result<Vec<Comment>, sqlx::Error> {
    let comments = sqlx::query_as(
        "SELECT id, app, topic, content, create_time, user, like, to_user, to_content FROM comment WHERE app = ? AND topic = ? ORDER BY create_time DESC",
    )
        .bind(app_id)
        .bind(topic)
        .fetch_all(pool)
        .await?;

    Ok(comments)
}
