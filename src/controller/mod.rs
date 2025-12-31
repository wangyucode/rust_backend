use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub payload: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn data_success(data: T) -> Self {
        ApiResponse {
            success: true,
            message: "success".to_string(),
            payload: Some(data),
        }
    }

    pub fn message_success(message: String) -> Self {
        ApiResponse {
            success: true,
            message,
            payload: None,
        }
    }

    pub fn error(message: String) -> Self {
        ApiResponse {
            success: false,
            message,
            payload: None,
        }
    }
}

pub mod blog;
pub mod clipboard;
pub mod comment;
pub mod config;
pub mod coze;
pub mod email;
pub mod state;
pub mod wechat;
