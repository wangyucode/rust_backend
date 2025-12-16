use actix_web::{HttpResponse, Responder, web};
use sqlx::SqlitePool;
use std::sync::Arc;

use super::ApiResponse;
use crate::dao::app::get_all_apps;

// 获取所有应用列表
pub async fn get_apps(pool: web::Data<Arc<SqlitePool>>) -> impl Responder {
    match get_all_apps(pool.as_ref()).await {
        Ok(apps) => HttpResponse::Ok().json(ApiResponse::data_success(apps)),
        Err(e) => {
            eprintln!("Error getting apps: {:?}", e);
            HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to get apps".to_string()))
        }
    }
}
