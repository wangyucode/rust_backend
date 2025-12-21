use actix_web::{HttpResponse, Responder, web};
use reqwest;
use sqlx::SqlitePool;
use std::sync::Arc;

use super::ApiResponse;
use crate::dao::app::get_all_apps;

const SESSION_URL: &str = "https://api.weixin.qq.com/sns/jscode2session";

// 获取微信会话信息
pub async fn get_wechat_session(appid: &str, secret: &str, jscode: &str) -> Result<serde_json::Value, reqwest::Error> {
    let url = format!("{SESSION_URL}?appid={appid}&secret={secret}&js_code={jscode}&grant_type=authorization_code");
    let client = reqwest::Client::new();
    let res = client.get(&url).send().await?;
    let json = res.json::<serde_json::Value>().await?;
    Ok(json)
}

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
