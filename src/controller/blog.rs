use actix_web::{HttpResponse, Responder, web};
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
    pool: web::Data<Arc<SqlitePool>>,
    query: web::Query<BlogViewQuery>,
) -> impl Responder {
    let blog_id = query.id.clone();

    // 获取当前时间戳（毫秒）
    let timestamp = Utc::now().timestamp_millis();

    // 使用DAO函数记录或更新博客访问
    let result = blog_dao::record_blog_visit(pool.as_ref(), &blog_id, timestamp).await;

    match result {
        Ok(_) => HttpResponse::Ok().json(ApiResponse::<()>::message_success("success".to_string())),
        Err(e) => {
            eprintln!("Failed to record view: {:?}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error("failed".to_string()))
        }
    }
}

pub async fn get_popular_posts(
    pool: web::Data<Arc<SqlitePool>>,
    query: web::Query<PopularPostsQuery>,
) -> impl Responder {
    // Ensure days doesn't exceed 30
    let days = query.days.min(30);
    let limit = query.limit;

    // 使用DAO函数获取热门文章
    let result = blog_dao::get_popular_posts(pool.as_ref(), days, limit).await;

    match result {
        Ok(posts) => HttpResponse::Ok().json(ApiResponse::data_success(posts)),
        Err(e) => {
            eprintln!("Failed to get popular posts: {:?}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                "Failed to get popular posts".to_string(),
            ))
        }
    }
}
