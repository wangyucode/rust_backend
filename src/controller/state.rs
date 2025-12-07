use actix_web::{HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct DataResult {
    state: String,
    time: u128,
    uuid: String,
    version: String,
}
#[derive(Serialize, Deserialize)]
struct Response {
    data: DataResult,
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
    let response = Response { data };
    HttpResponse::Ok().json(response)
}
