use actix_web::{HttpResponse, Responder, web};
use chrono::{Local, TimeZone};
use regex::Regex;
use sqlx::SqlitePool;
use std::sync::Arc;

use super::ApiResponse;
use crate::dao::comment::{
    Comment, CommentResponse, ToResponse, get_comments_by_app_topic, validate_app_key,
};

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
