use axum::{
    Json as AxumJson,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use hex;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::env;
use std::sync::Arc;

use super::ApiResponse;
use super::wechat::get_wechat_session;
use crate::dao::roll::{
    add_user_score, create_team_if_not_exists, get_all_teams, get_user_by_openid, get_user_rank,
    update_user_team, upsert_user_session,
};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub platform: String,
    pub data: HashMap<String, String>,
}

#[derive(Serialize)]
pub struct Team {
    pub name: String,
    pub score: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub openid: String,
    pub session_key: String,
    pub team: Option<String>,
    pub score: i64,
    pub rank: String,
    #[serde(rename = "teamRank")]
    pub team_rank: String,
    pub teams: Vec<Team>,
}

#[derive(Deserialize)]
pub struct SetTeamRequest {
    pub openid: String,
    pub team: Option<String>,
}

#[derive(Deserialize)]
pub struct ScoreRequest {
    pub openid: String,
    pub signature: String,
    pub score: i64,
}

type HmacSha256 = Hmac<Sha256>;

fn verify_signature(session_key: &str, data: &str, signature: &str) -> bool {
    let mut mac = match HmacSha256::new_from_slice(session_key.as_bytes()) {
        Ok(m) => m,
        Err(_) => return false,
    };
    mac.update(data.as_bytes());
    let result = mac.finalize();
    let expected_signature = hex::encode(result.into_bytes());
    signature == expected_signature
}

pub async fn login(
    State(pool): State<Arc<SqlitePool>>,
    AxumJson(body): AxumJson<LoginRequest>,
) -> impl IntoResponse {
    if body.platform != "wechat" {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("Unsupported platform".to_string())),
        )
            .into_response();
    }

    let code = match body.data.get("code") {
        Some(c) if !c.is_empty() => c,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error("code required".to_string())),
            )
                .into_response();
        }
    };

    let appid = env::var("WX_APPID_ROLL").unwrap_or_default();
    let secret = env::var("WX_SECRET_ROLL").unwrap_or_default();

    match get_wechat_session(&appid, &secret, code).await {
        Ok(session_info) => {
            let openid = session_info.get("openid").and_then(|id| id.as_str());
            let session_key = session_info.get("session_key").and_then(|sk| sk.as_str());

            if let (Some(openid), Some(session_key)) = (openid, session_key) {
                // Upsert user
                if let Err(e) = upsert_user_session(pool.as_ref(), openid, session_key).await {
                    eprintln!("Error upserting user: {:?}", e);
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<()>::error("Failed to login".to_string())),
                    )
                        .into_response();
                }

                // Get user info
                let user = match get_user_by_openid(pool.as_ref(), openid).await {
                    Ok(Some(u)) => u,
                    _ => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<()>::error("Failed to get user".to_string())),
                        )
                            .into_response();
                    }
                };

                let user_rank = get_user_rank(pool.as_ref(), user.score).await.unwrap_or(1);

                let teams_data = get_all_teams(pool.as_ref()).await.unwrap_or_default();
                let mut team_rank_val = 0;
                let mut teams = Vec::new();

                for (i, t) in teams_data.into_iter().enumerate() {
                    if let Some(ref user_team) = user.team_name {
                        if &t.name == user_team {
                            team_rank_val = i as i64 + 1;
                        }
                    }
                    teams.push(Team {
                        name: t.name,
                        score: t.score.to_string(),
                    });
                }

                let resp = LoginResponse {
                    openid: openid.to_string(),
                    session_key: session_key.to_string(),
                    team: user.team_name,
                    score: user.score,
                    rank: user_rank.to_string(),
                    team_rank: if team_rank_val > 0 {
                        team_rank_val.to_string()
                    } else {
                        "-".to_string()
                    },
                    teams,
                };

                Json(ApiResponse::data_success(resp)).into_response()
            } else {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(ApiResponse::<()>::error("Login failed".to_string())),
                )
                    .into_response()
            }
        }
        Err(e) => {
            eprintln!("Error getting wechat session: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    "Failed to get wechat session".to_string(),
                )),
            )
                .into_response()
        }
    }
}

pub async fn set_team(
    State(pool): State<Arc<SqlitePool>>,
    AxumJson(body): AxumJson<SetTeamRequest>,
) -> impl IntoResponse {
    let team_name = match body.team.as_deref() {
        Some(t) if t.trim().is_empty() => None,
        Some(t) => Some(t.trim()),
        None => None,
    };

    if let Some(t) = team_name {
        if let Err(e) = create_team_if_not_exists(pool.as_ref(), t).await {
            eprintln!("Error creating team: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    "Failed to create team".to_string(),
                )),
            )
                .into_response();
        }
    }

    match update_user_team(pool.as_ref(), &body.openid, team_name).await {
        Ok(rows) if rows > 0 => {
            Json(ApiResponse::<()>::message_success("success".to_string())).into_response()
        }
        Ok(_) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("User not found".to_string())),
        )
            .into_response(),
        Err(e) => {
            eprintln!("Error setting team: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error("Failed to set team".to_string())),
            )
                .into_response()
        }
    }
}

pub async fn report_score(
    State(pool): State<Arc<SqlitePool>>,
    AxumJson(body): AxumJson<ScoreRequest>,
) -> impl IntoResponse {
    let user = match get_user_by_openid(pool.as_ref(), &body.openid).await {
        Ok(Some(u)) => u,
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiResponse::<()>::error("Unauthorized".to_string())),
            )
                .into_response();
        }
    };

    let data_to_sign = body.score.to_string();

    if !verify_signature(&user.session, &data_to_sign, &body.signature) {
        return (
            StatusCode::FORBIDDEN,
            Json(ApiResponse::<()>::error("Invalid signature".to_string())),
        )
            .into_response();
    }

    match add_user_score(pool.as_ref(), &body.openid, body.score).await {
        Ok(Some(added_score)) => Json(ApiResponse::data_success(added_score)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("User not found".to_string())),
        )
            .into_response(),
        Err(e) => {
            eprintln!("Error reporting score: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    "Failed to report score".to_string(),
                )),
            )
                .into_response()
        }
    }
}
