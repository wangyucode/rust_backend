use axum::{
    response::{IntoResponse, Json},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use uuid::Uuid;

use super::ApiResponse;

#[derive(Debug, Serialize, Deserialize)]
struct JWTToken {
    access_token: String,
    token_type: String,
    expires_in: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct TokenRequest {
    grant_type: String,
    duration_seconds: u32,
    scope: Option<String>,
}

pub async fn get_token() -> impl IntoResponse {
    // 读取环境变量
    let app_id = env::var("COZE_APP_ID").expect("COZE_APP_ID must be set");
    let aud = "api.coze.cn";
    let key_id = env::var("COZE_KEY_ID").expect("COZE_KEY_ID must be set");
    let private_key = env::var("COZE_PRIVATE_KEY").expect("COZE_PRIVATE_KEY must be set");
    let base_url = env::var("COZE_BASE_URL").unwrap_or("https://api.coze.cn".to_string());
    let session_name = Uuid::new_v4().to_string();

    // 生成JWT token
    let now = Utc::now();
    let exp = now + Duration::hours(1);

    let claims = serde_json::Value::Object(serde_json::Map::from_iter(vec![
        ("iss".to_string(), serde_json::Value::String(app_id)),
        (
            "aud".to_string(),
            serde_json::Value::String(aud.to_string()),
        ),
        (
            "iat".to_string(),
            serde_json::Value::Number(serde_json::Number::from(now.timestamp())),
        ),
        (
            "exp".to_string(),
            serde_json::Value::Number(serde_json::Number::from(exp.timestamp())),
        ),
        (
            "jti".to_string(),
            serde_json::Value::String(Uuid::new_v4().to_string()),
        ),
        (
            "session_name".to_string(),
            serde_json::Value::String(session_name),
        ),
    ]));

    let header = Header {
        kid: Some(key_id),
        alg: Algorithm::RS256,
        ..Default::default()
    };

    let encoding_key =
        EncodingKey::from_rsa_pem(private_key.as_bytes()).expect("Failed to parse private key");

    let jwt_token =
        jsonwebtoken::encode(&header, &claims, &encoding_key).expect("Failed to encode JWT");

    // 交换OAuth token
    let client = Client::new();
    let token_request = TokenRequest {
        grant_type: "urn:ietf:params:oauth:grant-type:jwt-bearer".to_string(),
        duration_seconds: 900, // 15 minutes
        scope: None,
    };

    let api_url = format!("{}/api/permission/oauth2/token", base_url);

    let response = client
        .post(&api_url)
        .json(&token_request)
        .bearer_auth(jwt_token)
        .send()
        .await
        .expect("Failed to send token request");

    let jwt_response: JWTToken = response
        .json()
        .await
        .expect("Failed to parse token response");

    Json(ApiResponse::data_success(jwt_response))
}
