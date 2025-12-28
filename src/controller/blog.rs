use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::Utc;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::sync::Arc;

use super::ApiResponse;
use crate::dao::blog as blog_dao;

#[derive(Deserialize)]
pub struct BlogViewQuery {
    id: String,
}

#[derive(Deserialize)]
pub struct PopularPostsQuery {
    #[serde(default = "default_days")]
    days: i64,
    #[serde(default = "default_limit")]
    limit: i64,
}

fn default_days() -> i64 {
    30
}

fn default_limit() -> i64 {
    10
}

pub async fn record_blog_view(
    State(pool): State<Arc<SqlitePool>>,
    Query(query): Query<BlogViewQuery>,
) -> impl IntoResponse {
    let blog_id = query.id.clone();

    // 获取当前时间戳（毫秒）
    let timestamp = Utc::now().timestamp_millis();

    // 使用DAO函数记录或更新博客访问
    let result = blog_dao::record_blog_visit(pool.as_ref(), &blog_id, timestamp).await;

    match result {
        Ok(_) => Json(ApiResponse::<()>::message_success("success".to_string())).into_response(),
        Err(e) => {
            eprintln!("Failed to record view: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error("failed".to_string())),
            )
                .into_response()
        }
    }
}

pub async fn get_popular_posts(
    State(pool): State<Arc<SqlitePool>>,
    Query(query): Query<PopularPostsQuery>,
) -> impl IntoResponse {
    // Ensure days doesn't exceed 30
    let days = query.days.min(30);
    let limit = query.limit;

    // 使用DAO函数获取热门文章
    let result = blog_dao::get_popular_posts(pool.as_ref(), days, limit).await;

    match result {
        Ok(posts) => Json(ApiResponse::data_success(posts)).into_response(),
        Err(e) => {
            eprintln!("Failed to get popular posts: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    "Failed to get popular posts".to_string(),
                )),
            )
                .into_response()
        }
    }
}
