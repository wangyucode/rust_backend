use sqlx::SqlitePool;
use std::sync::Arc;
use reqwest::Client;
pub struct AppState {
    pub pool: Arc<SqlitePool>,
    pub client: Client,
    pub ai_config: AiConfig,
}

pub struct AiConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

impl AppState {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        let api_key = std::env::var("AI_API_KEY").unwrap_or_default();
        if api_key.is_empty() {
            eprintln!("⚠️  AI_API_KEY not configured, AI features may fail");
        }
        let model = std::env::var("AI_MODEL").unwrap_or_default();
        let base_url = std::env::var("AI_BASE_URL").unwrap_or_default();

        Self {
            pool,
            client: Client::new(),
            ai_config: AiConfig {
                api_key,
                model,
                base_url,
            },
        }
    }
}
