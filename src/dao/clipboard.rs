use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

// Clipboard数据结构
#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Clipboard {
    pub id: String,
    pub content: String,
    pub openid: String,
    pub create_time: i64,
    pub update_time: i64,
}

// Clipboard响应数据结构
#[derive(Debug, Serialize, Deserialize)]
pub struct ClipboardResponse {
    #[serde(rename = "_id")]
    pub id: String,
    pub content: String,
    #[serde(rename = "createDate")]
    pub create_time: i64,
    #[serde(rename = "lastUpdate")]
    pub update_time: i64,
}

// 根据id获取剪贴板内容
pub async fn get_clipboard_by_id(
    pool: &SqlitePool,
    id: &str,
) -> Result<Option<Clipboard>, sqlx::Error> {
    let clipboard = sqlx::query_as(
        "SELECT id, content, openid, create_time, update_time FROM clipboard WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    Ok(clipboard)
}
