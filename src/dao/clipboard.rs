use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

// Clipboard数据结构
#[derive(Debug, FromRow, Serialize, Deserialize, Clone)]
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

// 根据openid获取剪贴板内容
pub async fn get_clipboard_by_openid(
    pool: &SqlitePool,
    openid: &str,
) -> Result<Vec<Clipboard>, sqlx::Error> {
    let clipboards = sqlx::query_as(
        "SELECT id, content, openid, create_time, update_time FROM clipboard WHERE openid = ?",
    )
    .bind(openid)
    .fetch_all(pool)
    .await?;

    Ok(clipboards)
}

// 根据openid获取单个剪贴板内容
pub async fn get_single_clipboard_by_openid(
    pool: &SqlitePool,
    openid: &str,
) -> Result<Option<Clipboard>, sqlx::Error> {
    let clipboard = sqlx::query_as(
        "SELECT id, content, openid, create_time, update_time FROM clipboard WHERE openid = ? LIMIT 1",
    )
    .bind(openid)
    .fetch_optional(pool)
    .await?;

    Ok(clipboard)
}

// 根据id更新剪贴板内容
pub async fn update_clipboard_by_id(
    pool: &SqlitePool,
    id: &str,
    content: &str,
    update_time: i64,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("UPDATE clipboard SET content = ?, update_time = ? WHERE id = ?")
        .bind(content)
        .bind(update_time)
        .bind(id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

// 插入新的剪贴板内容
pub async fn insert_clipboard(
    pool: &SqlitePool,
    clipboard: &Clipboard,
) -> Result<Clipboard, sqlx::Error> {
    sqlx::query(
        "INSERT INTO clipboard (id, content, openid, create_time, update_time) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(&clipboard.id)
    .bind(&clipboard.content)
    .bind(&clipboard.openid)
    .bind(clipboard.create_time)
    .bind(clipboard.update_time)
    .execute(pool)
    .await?;

    Ok(clipboard.clone())
}
