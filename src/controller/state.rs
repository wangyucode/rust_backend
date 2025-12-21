use actix_web::{HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::ApiResponse;

#[derive(Serialize, Deserialize)]
struct DataResult {
    state: String,
    time: u128,
    uuid: String,
    version: String,
}

pub async fn state() -> impl Responder {
    let data = DataResult {
        state: "UP".to_string(),
        time: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        uuid: Uuid::new_v4().to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    HttpResponse::Ok().json(ApiResponse::data_success(data))
}
