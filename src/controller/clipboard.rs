use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    Json as AxumJson,
};
use rand;
use sqlx::SqlitePool;
use std::env;
use std::sync::Arc;

use super::ApiResponse;
use super::wechat::get_wechat_session;
use crate::dao::clipboard::{
    get_clipboard_by_id, get_clipboard_by_openid, insert_clipboard,
    update_clipboard_by_id, Clipboard, ClipboardResponse,
};
use crate::util::email::{EmailConfig, send_email};
use crate::util::uuid::generate_short_uuid;

// 获取剪贴板内容的路径参数结构体
#[derive(Debug, serde::Deserialize)]
pub struct ClipboardPath {
    id: String,
}

// 根据openid获取剪贴板内容的路径参数结构体
#[derive(Debug, serde::Deserialize)]
pub struct ClipboardOpenidPath {
    openid: String,
}

// 根据微信code获取剪贴板内容的路径参数结构体
#[derive(Debug, serde::Deserialize)]
pub struct ClipboardWxCodePath {
    code: String,
}

// 保存剪贴板内容的请求体结构体
#[derive(Debug, serde::Deserialize)]
pub struct SaveClipboardRequest {
    #[serde(rename = "_id")]
    pub id: String,
    pub content: String,
}

// 转换Clipboard为响应格式
fn to_response(clipboard: Clipboard) -> ClipboardResponse {
    ClipboardResponse {
        id: clipboard.id,
        content: clipboard.content,
        create_time: clipboard.create_time,
        update_time: clipboard.update_time,
    }
}

// 根据id获取剪贴板内容的处理函数
pub async fn get_by_id(
    State(pool): State<Arc<SqlitePool>>,
    Path(path): Path<ClipboardPath>,
) -> impl IntoResponse {
    // 验证id参数
    if path.id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("id required".to_string())),
        )
            .into_response();
    }

    // 获取剪贴板内容
    match get_clipboard_by_id(pool.as_ref(), &path.id).await {
        Ok(Some(clipboard)) => {
            // 转换为响应格式
            let response = to_response(clipboard);
            Json(ApiResponse::data_success(response)).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("未找到".to_string())),
        )
            .into_response(),
        Err(e) => {
            eprintln!("Error getting clipboard: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    "Failed to get clipboard content".to_string(),
                )),
            )
                .into_response()
        }
    }
}

// 根据openid获取剪贴板内容的处理函数
pub async fn get_by_openid(
    State(pool): State<Arc<SqlitePool>>,
    Path(path): Path<ClipboardOpenidPath>,
) -> impl IntoResponse {
    // 验证openid参数
    if path.openid.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("openid required".to_string())),
        )
            .into_response();
    }

    // 获取剪贴板内容
    match get_clipboard_by_openid(pool.as_ref(), &path.openid).await {
        Ok(Some(clipboard)) => {
            // 转换为响应格式
            let response = to_response(clipboard);
            Json(ApiResponse::data_success(response)).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("未找到".to_string())),
        )
            .into_response(),
        Err(e) => {
            eprintln!("Error getting clipboards by openid: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    "Failed to get clipboard content".to_string(),
                )),
            )
                .into_response()
        }
    }
}

// 保存剪贴板内容的处理函数
pub async fn save_by_id(
    State(pool): State<Arc<SqlitePool>>,
    AxumJson(body): AxumJson<SaveClipboardRequest>,
) -> impl IntoResponse {
    // 验证id参数
    if body.id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("_id required".to_string())),
        )
            .into_response();
    }

    // 获取当前时间戳（毫秒）
    let update_time = chrono::Utc::now().timestamp();

    // 更新剪贴板内容
    match update_clipboard_by_id(pool.as_ref(), &body.id, &body.content, update_time).await {
        Ok(rows_affected) if rows_affected > 0 => {
            // 更新成功，返回更新后的剪贴板内容
            match get_clipboard_by_id(pool.as_ref(), &body.id).await {
                Ok(Some(clipboard)) => {
                    let response = to_response(clipboard);
                    Json(ApiResponse::data_success(response)).into_response()
                }
                Ok(None) => (
                    StatusCode::NOT_FOUND,
                    Json(ApiResponse::<()>::error("未找到".to_string())),
                )
                    .into_response(),
                Err(e) => {
                    eprintln!("Error getting updated clipboard: {:?}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<()>::error(
                            "Failed to get updated clipboard content".to_string(),
                        )),
                    )
                        .into_response()
                }
            }
        }
        Ok(_) => {
            // 没有找到要更新的记录
            (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<()>::error("未找到".to_string())),
            )
                .into_response()
        }
        Err(e) => {
            eprintln!("Error updating clipboard: {:?}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(
                    "Failed to update clipboard content".to_string(),
                )),
            )
                .into_response()
        }
    }
}

// 根据微信code获取剪贴板内容的处理函数
pub async fn get_by_wx_code(
    State(pool): State<Arc<SqlitePool>>,
    Path(path): Path<ClipboardWxCodePath>,
) -> impl IntoResponse {
    // 验证code参数
    if path.code.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("code required".to_string())),
        )
            .into_response();
    }

    // 获取环境变量
    let appid = env::var("WX_APPID_CLIPBOARD").unwrap_or_default();
    let secret = env::var("WX_SECRET_CLIPBOARD").unwrap_or_default();

    // 获取微信会话信息
    match get_wechat_session(&appid, &secret, &path.code).await {
        Ok(session) => {
            // 提取openid
            if let Some(openid) = session.get("openid").and_then(|id| id.as_str()) {
                // 查询该openid是否已有剪贴板
                match get_clipboard_by_openid(pool.as_ref(), openid).await {
                    Ok(Some(clipboard)) => {
                        // 已存在，返回
                        let response = to_response(clipboard);
                        Json(ApiResponse::data_success(response)).into_response()
                    }
                    Ok(None) => {
                        // 不存在，创建新的剪贴板
                        let mut id = generate_short_uuid();

                        // 确保id唯一
                        while get_clipboard_by_id(pool.as_ref(), &id)
                            .await
                            .unwrap_or(None)
                            .is_some()
                        {
                            id.push_str(&rand::random::<u8>().to_string());
                        }

                        // 获取当前时间戳
                        let now = chrono::Utc::now().timestamp();

                        // 创建新的剪贴板
                        let new_clipboard = Clipboard {
                            id,
                            content: "请输入你想保存的内容,内容可在网页端: `https://wycode.cn/clipboard`  使用查询码查询,或小程序免登录查询。".to_string(),
                            openid: openid.to_string(),
                            create_time: now,
                            update_time: now,
                        };

                        // 插入数据库
                        match insert_clipboard(pool.as_ref(), &new_clipboard).await {
                            Ok(clipboard) => {
                                // 发送邮件通知
                                let email_config = EmailConfig::new(
                                    Some("有新的用户注册了Clipboard服务".to_string()),
                                    "剪贴板服务".to_string(),
                                    None,
                                );
                                if let Err(e) = send_email(email_config).await {
                                    eprintln!("Error sending email: {:?}", e);
                                }

                                // 返回新创建的剪贴板
                                let response = to_response(clipboard);
                                Json(ApiResponse::data_success(response)).into_response()
                            }
                            Err(e) => {
                                eprintln!("Error inserting clipboard: {:?}", e);
                                (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    Json(ApiResponse::<()>::error(
                                        "Failed to create clipboard".to_string(),
                                    )),
                                )
                                    .into_response()
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Error checking clipboard by openid: {:?}", e);
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<()>::error(
                                "Failed to check clipboard".to_string(),
                            )),
                        )
                            .into_response()
                    }
                }
            } else {
                // 没有获取到openid
                (
                    StatusCode::UNAUTHORIZED,
                    Json(ApiResponse::<()>::error("登录失败".to_string())),
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
