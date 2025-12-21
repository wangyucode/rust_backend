use actix_web::{HttpResponse, Responder, web};
use chrono::{Local, TimeZone, Utc};
use regex::Regex;
use sqlx::SqlitePool;
use std::sync::Arc;
use uuid::Uuid;

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

// 隐藏用户邮箱
fn hide_email(user: &str) -> String {
    lazy_static::lazy_static! {
        static ref EMAIL_REGEX: Regex = Regex::new(r"^\S+@\w+(\.[\w]+)+").unwrap();
    }

    if EMAIL_REGEX.is_match(user) {
        if let Some(at_index) = user.rfind('@') {
            if at_index > 1 {
                let first_char = &user[0..1];
                let asterisks = "*".repeat(at_index - 1);
                let domain = &user[at_index..];
                format!("{}{}{}", first_char, asterisks, domain)
            } else {
                user.to_string()
            }
        } else {
            user.to_string()
        }
    } else {
        user.to_string()
    }
}

// 转换Comment为CommentResponse
fn convert_to_response(comment: &Comment) -> CommentResponse {
    // 处理to字段
    let to = if let (Some(to_user), Some(to_content)) = (&comment.to_user, &comment.to_content) {
        Some(ToResponse {
            content: to_content.clone(),
            user: hide_email(to_user),
        })
    } else {
        None
    };

    CommentResponse {
        id: comment.id.clone(),
        content: comment.content.clone(),
        user: hide_email(&comment.user),
        like: comment.like,
        create_time: format_timestamp(comment.create_time),
        to,
    }
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
    to: Option<String>,      // 回复对象用户
    #[serde(rename = "toId")]
    to_id: Option<String>, // 回复对象ID
}

// 获取评论列表的处理函数
pub async fn get_comments(
    pool: web::Data<Arc<SqlitePool>>,
    query: web::Query<CommentQuery>,
) -> impl Responder {
    // 验证查询参数
    if query.a.is_empty() {
        return HttpResponse::BadRequest().json(ApiResponse::<()>::error("a required".to_string()));
    }
    if query.k.is_empty() {
        return HttpResponse::BadRequest().json(ApiResponse::<()>::error("k required".to_string()));
    }
    if query.t.is_empty() {
        return HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error("topic required".to_string()));
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

                    HttpResponse::Ok().json(ApiResponse::data_success(response_comments))
                }
                Err(e) => {
                    eprintln!("Error getting comments: {:?}", e);
                    HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                        "Failed to get comments".to_string(),
                    ))
                }
            }
        }
        Ok(false) => {
            HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Unauthorized".to_string()))
        }
        Err(e) => {
            eprintln!("Error validating app key: {:?}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                "Failed to validate app key".to_string(),
            ))
        }
    }
}

// 提交评论的处理函数
pub async fn post_comment(
    pool: web::Data<Arc<SqlitePool>>,
    body: web::Json<PostCommentBody>,
) -> impl Responder {
    // 验证评论类型
    if body.c_type < 0 || body.c_type > 1 {
        return HttpResponse::BadRequest()
            .json(ApiResponse::<()>::error("评论类型不合法".to_string()));
    }

    // 验证评论内容
    if body.c_type == 0 {
        if body.content.as_ref().map_or(true, |s| s.is_empty()) {
            return HttpResponse::BadRequest()
                .json(ApiResponse::<()>::error("内容不能为空".to_string()));
        }
        if let Some(content) = &body.content {
            if content.len() > 1023 {
                return HttpResponse::BadRequest()
                    .json(ApiResponse::<()>::error("内容不能超过1000个字".to_string()));
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
                    let to_user = body.to.clone();
                    let to_content = None; // 这里可以根据toId获取被回复评论的内容，目前暂时设为None

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
                            )) {
                                eprintln!("Failed to send email: {:?}", e);
                            }

                            HttpResponse::Ok().json(ApiResponse::data_success(inserted_id))
                        }
                        Err(e) => {
                            eprintln!("Error inserting comment: {:?}", e);
                            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                                "Failed to insert comment".to_string(),
                            ))
                        }
                    }
                }
                // 点赞评论
                1 => {
                    if body.to_id.is_none() || body.to_id.as_ref().unwrap().is_empty() {
                        return HttpResponse::BadRequest()
                            .json(ApiResponse::<()>::error("toId required".to_string()));
                    }

                    let comment_id = body.to_id.as_ref().unwrap();
                    match update_comment_like(pool.as_ref(), comment_id).await {
                        Ok(modified_count) => {
                            HttpResponse::Ok().json(ApiResponse::data_success(modified_count))
                        }
                        Err(e) => {
                            eprintln!("Error updating comment like: {:?}", e);
                            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                                "Failed to update comment like".to_string(),
                            ))
                        }
                    }
                }
                _ => HttpResponse::BadRequest()
                    .json(ApiResponse::<()>::error("暂不支持".to_string())),
            }
        }
        Ok(false) => {
            HttpResponse::Unauthorized().json(ApiResponse::<()>::error("Unauthorized".to_string()))
        }
        Err(e) => {
            eprintln!("Error validating app key: {:?}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                "Failed to validate app key".to_string(),
            ))
        }
    }
}
