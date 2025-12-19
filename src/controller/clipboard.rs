use actix_web::{HttpResponse, Responder, web};
use sqlx::SqlitePool;
use std::sync::Arc;

use super::ApiResponse;
use crate::dao::clipboard::{get_clipboard_by_id, Clipboard, ClipboardResponse};

// 获取剪贴板内容的路径参数结构体
#[derive(Debug, serde::Deserialize)]
pub struct ClipboardPath {
    id: String,
}

// 转换Clipboard为响应格式
fn to_response(clipboard: &Clipboard) -> ClipboardResponse {
    ClipboardResponse {
        id: clipboard.id.clone(),
        content: clipboard.content.clone(),
        create_time: clipboard.create_time,
        update_time: clipboard.update_time,
    }
}

// 根据id获取剪贴板内容的处理函数
pub async fn get_by_id(
    pool: web::Data<Arc<SqlitePool>>,
    path: web::Path<ClipboardPath>,
) -> impl Responder {
    // 验证id参数
    if path.id.is_empty() {
        return HttpResponse::BadRequest().json(ApiResponse::<()>::error("id required".to_string()));
    }

    // 获取剪贴板内容
    match get_clipboard_by_id(pool.as_ref(), &path.id).await {
        Ok(Some(clipboard)) => {
            // 转换为响应格式
            let response = to_response(&clipboard);
            HttpResponse::Ok().json(ApiResponse::data_success(response))
        },
        Ok(None) => {
            HttpResponse::NotFound().json(ApiResponse::<()>::error("未找到".to_string()))
        },
        Err(e) => {
            eprintln!("Error getting clipboard: {:?}", e);
            HttpResponse::InternalServerError().json(ApiResponse::<()>::error(
                "Failed to get clipboard content".to_string(),
            ))
        },
    }
}