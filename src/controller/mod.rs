use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum ApiResponse<T> {
    DataSuccess { payload: T, success: bool },
    MessageSuccess { message: String, success: bool },
    Error { message: String, success: bool },
}

impl<T> ApiResponse<T> {
    pub fn data_success(payload: T) -> Self {
        ApiResponse::DataSuccess {
            payload,
            success: true,
        }
    }

    pub fn message_success(message: String) -> Self {
        ApiResponse::MessageSuccess {
            message,
            success: true,
        }
    }

    pub fn error(message: String) -> Self {
        ApiResponse::Error {
            message,
            success: false,
        }
    }
}

pub mod app;
pub mod email;
pub mod state;
