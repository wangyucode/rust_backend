use crate::controller::ApiResponse;
use axum::{
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde_json::{self, Value};
use std::fs;
use std::path::Path;

#[derive(serde::Deserialize)]
pub struct ConfigQuery {
    pub key: String,
}

pub async fn get_config(Query(query): Query<ConfigQuery>) -> impl IntoResponse {
    let key = &query.key;
    let file_path = format!("./db/config/{}.json", key);
    let path = Path::new(&file_path);

    if !path.exists() {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("Config not found".to_string())),
        )
            .into_response();
    }

    match fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str::<Value>(&content) {
            Ok(json_data) => Json(ApiResponse::data_success(json_data)).into_response(),
            Err(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error("Failed to parse JSON".to_string())),
            )
                .into_response(),
        },
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error("Failed to read file".to_string())),
        )
            .into_response(),
    }
}
