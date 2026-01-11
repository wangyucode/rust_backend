use crate::controller::ApiResponse;
use axum::{
    extract::Path,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use std::path::PathBuf;
use tokio::fs;

pub async fn get_yml(
    Path(path): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let accept_header = headers
        .get("accept")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    let is_json = accept_header.contains("application/json") || accept_header.contains("json");

    // Helper to return error response
    let return_error = |code: StatusCode, msg: String| -> axum::response::Response {
        if is_json {
            (code, Json(ApiResponse::<()>::error(msg))).into_response()
        } else {
            let resp = ApiResponse::<()>::error(msg);
            let body = serde_yaml::to_string(&resp).unwrap_or_else(|_| "Internal Serialization Error".to_string());
            (
                code,
                [(axum::http::header::CONTENT_TYPE, "application/yaml")],
                body,
            )
                .into_response()
        }
    };

    // Sanitize path to prevent directory traversal
    if path.contains("..") {
        return return_error(StatusCode::BAD_REQUEST, "Invalid path".to_string());
    }

    let file_path = PathBuf::from("db/yml").join(&path);

    if !file_path.exists() {
        return return_error(StatusCode::NOT_FOUND, "File not found".to_string());
    }

    let content = match fs::read_to_string(&file_path).await {
        Ok(c) => c,
        Err(e) => return return_error(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };

    // Parse YAML content
    let yaml_value: serde_yaml::Value = match serde_yaml::from_str(&content) {
        Ok(v) => v,
        Err(e) => return return_error(StatusCode::INTERNAL_SERVER_ERROR, format!("Invalid YAML: {}", e)),
    };

    // Return success response
    if is_json {
        // For JSON, we can use serde_yaml::Value directly as it implements Serialize,
        // or convert to serde_json::Value if strictly needed. 
        // ApiResponse<T> where T: Serialize works with serde_yaml::Value.
        Json(ApiResponse::data_success(yaml_value)).into_response()
    } else {
        // For YAML, we serialize the ApiResponse wrapper to YAML
        let resp = ApiResponse::data_success(yaml_value);
        match serde_yaml::to_string(&resp) {
            Ok(body) => (
                StatusCode::OK,
                [(axum::http::header::CONTENT_TYPE, "application/yaml")],
                body,
            )
                .into_response(),
            Err(e) => return_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to serialize response: {}", e),
            ),
        }
    }
}
