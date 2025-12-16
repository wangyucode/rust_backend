use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool};

// 应用数据结构
#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct App {
    pub appid: String,
    pub name: String,
    pub image: String,
    pub description: String,
}

// 获取所有应用列表
pub async fn get_all_apps(pool: &SqlitePool) -> Result<Vec<App>, sqlx::Error> {
    let apps = sqlx::query_as("SELECT id, appid, name, image, description FROM apps")
        .fetch_all(pool)
        .await?;

    Ok(apps)
}
