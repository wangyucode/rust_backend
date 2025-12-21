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
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                "Failed to record view".to_string(),
            ))
        }
    }
}
