use sqlx::SqlitePool;
use anyhow::Result;
use chrono::Utc;

pub async fn insert_message(pool: &SqlitePool, user_id: &str, role: &str, content: &str) -> Result<i64> {
    let now = Utc::now().timestamp();
    let id = sqlx::query(
        "INSERT INTO ai_message (user_id, role, content, created_at) VALUES (?, ?, ?, ?)"
    )
    .bind(user_id)
    .bind(role)
    .bind(content)
    .bind(now)
    .execute(pool)
    .await?
    .last_insert_rowid();

    Ok(id)
}

pub async fn clean_old_messages(pool: &SqlitePool) -> Result<u64> {
    let one_week_ago = Utc::now().timestamp() - 7 * 24 * 60 * 60;
    let rows_affected = sqlx::query(
        "DELETE FROM ai_message WHERE created_at < ?"
    )
    .bind(one_week_ago)
    .execute(pool)
    .await?
    .rows_affected();

    Ok(rows_affected)
}
