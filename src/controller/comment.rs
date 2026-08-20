use axum::{
    extract::{Json as AxumJson, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::{Local, TimeZone, Utc};
use regex::Regex;
use std::sync::Arc;
use uuid::Uuid;
use crate::app_state::AppState;

use super::ApiResponse;
use crate::dao::comment::{
    Comment, CommentResponse, ToResponse, get_comments_by_app_topic, insert_comment,
    update_comment_like, validate_app_key,
};
use crate::util::email::{EmailConfig, send_email};

// 请求查询参数结构体
#[derive(Debug, serde::Deserialize)]
pub struct CommentQuery {
    a: String, // app
    k: String, // key
    t: String, // topic
}

// 格式化时间戳为字符串
fn format_timestamp(ts: i64) -> String {
    let dt = Local.timestamp_opt(ts, 0).unwrap();
    dt.format("%Y/%m/%d %H:%M:%S").to_string()
}

// 隐藏用户邮箱（用 chars() 按字符处理，避免 UTF-8 多字节字符的字节索引问题）
fn hide_email(user: &str) -> String {
    lazy_static::lazy_static! {
        static ref EMAIL_REGEX: Regex = Regex::new(r"^\S+@\w+(\.[\w]+)+").unwrap();
    }

    if EMAIL_REGEX.is_match(user) {
        if let Some((local, domain)) = user.split_once('@') {
            let mut chars = local.chars();
            if let Some(first_char) = chars.next() {
                let remaining_chars = chars.count();
                if remaining_chars > 0 {
                    let asterisks = "*".repeat(remaining_chars);
                    return format!("{}{}@{}", first_char, asterisks, domain);
                }
            }
        }
    }

    user.to_string()
}

// 转换Comment为CommentResponse
fn convert_to_response(comment: &Comment) -> CommentResponse {
    // 处理to字段：只要有 to_user 就返回，前端需要展示被回复人；content 缺失时用空字符串兼容历史数据
    let to = comment.to_user.as_ref().map(|to_user| ToResponse {
        content: comment.to_content.clone().unwrap_or_default(),
        user: hide_email(to_user),
    });

    CommentResponse {
        id: comment.id.clone(),
        content: comment.content.clone(),
        user: hide_email(&comment.user),
        like: comment.like,
        create_time: format_timestamp(comment.create_time),
        to,
    }
}

// 回复对象：前端发送 {user, content} 对象，对应 GET 返回的 ToResponse 结构
#[derive(Debug, serde::Deserialize)]
pub struct ToObject {
    pub user: String,
    pub content: Option<String>,
}

// POST评论请求体结构体
#[derive(Debug, serde::Deserialize)]
pub struct PostCommentBody {
    #[serde(rename = "type")]
    c_type: i32, // 评论类型，0.评论，1.点赞
    content: Option<String>, // 评论内容
    app: String,             // 应用ID
    key: String,             // 应用密钥
    topic: String,           // 话题
    user: String,            // 用户
    to: Option<ToObject>,    // 回复对象，前端发送 {user, content} 对象
    #[serde(rename = "toId")]
    to_id: Option<String>, // 回复对象ID
    // 前端会额外发送 like 字段，需声明以避免潜在的 deny_unknown_fields 影响，默认忽略
    #[serde(default)]
    #[allow(dead_code)]
    like: Option<serde_json::Value>,
}

// 获取评论列表的处理函数
pub async fn get_comments(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CommentQuery>,
) -> impl IntoResponse {
    let pool = &state.pool;
    // 验证查询参数
    if query.a.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("a required".to_string())),
        )
            .into_response();
    }
    if query.k.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("k required".to_string())),
        )
            .into_response();
    }
    if query.t.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("topic required".to_string())),
        )
            .into_response();
    }

    // 验证app和key
    match validate_app_key(pool.as_ref(), &query.a, &query.k).await {
        Ok(true) => {
            // 获取评论列表
            match get_comments_by_app_topic(pool.as_ref(), &query.a, &query.t).await {
                Ok(comments) => {
                    // 转换为响应格式
                    let response_comments: Vec<CommentResponse> =
                        comments.iter().map(convert_to_response).collect();

                    Json(ApiResponse::data_success(response_comments)).into_response()
                }
                Err(e) => {
                    eprintln!("Error getting comments: {:?}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<()>::error(
                            "Failed to get comments".to_string(),
                        )),
                    )
                        .into_response()
                }
            }
        }
        Ok(false) => (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<()>::error("Unauthorized".to_string())),
        )
            .into_response(),
        Err(e) => {
            eprintln!("Error validating app key: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    "Failed to validate app key".to_string(),
                )),
            )
                .into_response()
        }
    }
}

// 提交评论的处理函数
pub async fn post_comment(
    State(state): State<Arc<AppState>>,
    AxumJson(body): AxumJson<PostCommentBody>,
) -> impl IntoResponse {
    let pool = &state.pool;
    // 验证评论类型
    if body.c_type < 0 || body.c_type > 1 {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("评论类型不合法".to_string())),
        )
            .into_response();
    }

    // 验证评论内容
    if body.c_type == 0 {
        if body.content.as_ref().map_or(true, |s| s.is_empty()) {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error("内容不能为空".to_string())),
            )
                .into_response();
        }
        if let Some(content) = &body.content {
            if content.len() > 1023 {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<()>::error("内容不能超过1000个字".to_string())),
                )
                    .into_response();
            }
        }
    }

    // 验证app和key
    match validate_app_key(pool.as_ref(), &body.app, &body.key).await {
        Ok(true) => {
            match body.c_type {
                // 添加新评论
                0 => {
                    let content = body.content.as_ref().unwrap().clone();
                    let (to_user, to_content) = match &body.to {
                        Some(o) => (Some(o.user.clone()), o.content.clone()),
                        None => (None, None),
                    };

                    // 创建新评论
                    let comment = Comment {
                        id: Uuid::new_v4().to_string(),
                        app: body.app.clone(),
                        topic: body.topic.clone(),
                        content,
                        create_time: Utc::now().timestamp(),
                        user: body.user.clone(),
                        like: 0,
                        to_user,
                        to_content,
                    };

                    // 插入评论
                    match insert_comment(pool.as_ref(), &comment).await {
                        Ok(inserted_id) => {
                            // 发送邮件通知
                            let email_content = format!(
                                "评论已保存: {} - {}\n{}",
                                comment.app,
                                comment.topic,
                                serde_json::to_string_pretty(&comment).unwrap_or_default()
                            );

                            if let Err(e) = send_email(EmailConfig::new(
                                Some(format!("新评论通知: {} - {}", comment.app, comment.topic)),
                                email_content,
                                None,
                            ))
                            .await
                            {
                                eprintln!("Failed to send email: {:?}", e);
                            }

                            Json(ApiResponse::data_success(inserted_id)).into_response()
                        }
                        Err(e) => {
                            eprintln!("Error inserting comment: {:?}", e);
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<()>::error(
                                    "Failed to insert comment".to_string(),
                                )),
                            )
                                .into_response()
                        }
                    }
                }
                // 点赞评论
                1 => {
                    if body.to_id.is_none() || body.to_id.as_ref().unwrap().is_empty() {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(ApiResponse::<()>::error("toId required".to_string())),
                        )
                            .into_response();
                    }

                    let comment_id = body.to_id.as_ref().unwrap();
                    match update_comment_like(pool.as_ref(), comment_id).await {
                        Ok(modified_count) => {
                            Json(ApiResponse::data_success(modified_count)).into_response()
                        }
                        Err(e) => {
                            eprintln!("Error updating comment like: {:?}", e);
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                Json(ApiResponse::<()>::error(
                                    "Failed to update comment like".to_string(),
                                )),
                            )
                                .into_response()
                        }
                    }
                }
                _ => (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<()>::error("暂不支持".to_string())),
                )
                    .into_response(),
            }
        }
        Ok(false) => (
            StatusCode::UNAUTHORIZED,
            Json(ApiResponse::<()>::error("Unauthorized".to_string())),
        )
            .into_response(),
        Err(e) => {
            eprintln!("Error validating app key: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    "Failed to validate app key".to_string(),
                )),
            )
                .into_response()
        }
    }
}
