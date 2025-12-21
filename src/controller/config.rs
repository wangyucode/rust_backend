use crate::controller::ApiResponse;
use actix_web::{HttpResponse, Responder, web};
use serde_json::{self, Value};
use std::fs;
use std::path::Path;

pub async fn get_config(query: web::Query<ConfigQuery>) -> impl Responder {
    let key = &query.key;
    let file_path = format!("./db/config/{}.json", key);
    let path = Path::new(&file_path);

    if !path.exists() {
        return HttpResponse::NotFound()
            .json(ApiResponse::<()>::error("Config not found".to_string()));
    }

    match fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str::<Value>(&content) {
            Ok(json_data) => HttpResponse::Ok().json(ApiResponse::data_success(json_data)),
            Err(_) => HttpResponse::InternalServerError()
                .json(ApiResponse::<()>::error("Failed to parse JSON".to_string())),
        },
        Err(_) => HttpResponse::InternalServerError()
            .json(ApiResponse::<()>::error("Failed to read file".to_string())),
    }
}

#[derive(serde::Deserialize)]
pub struct ConfigQuery {
    pub key: String,
}
