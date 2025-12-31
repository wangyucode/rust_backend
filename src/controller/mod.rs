use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn data_success(data: T) -> Self {
        ApiResponse {
            success: true,
            message: "success".to_string(),
            data: Some(data),
        }
    }

    pub fn message_success(message: String) -> Self {
        ApiResponse {
            success: true,
            message,
            data: None,
        }
    }

    pub fn error(message: String) -> Self {
        ApiResponse {
            success: false,
            message,
            data: None,
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
