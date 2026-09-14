use reqwest::Client;
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;

/// AI 配置来源的 `.env` 文件（compose 以只读方式挂载，见 docker-compose.yml）。
const DOTENV_PATH: &str = ".env";
pub struct AppState {
    pub pool: Arc<SqlitePool>,
    pub client: Client,
    pub ai_config: AiConfig,
}

pub struct AiConfig {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub www_root: String,
    pub tools_enabled: bool,
}

impl AppState {
    pub fn new(pool: Arc<SqlitePool>) -> Self {
        let ai_config = AiConfig::load_live();
        if ai_config.api_key.is_empty() {
            eprintln!("⚠️  AI_API_KEY not configured, AI features may fail");
        }

        Self {
            pool,
            client: Client::new(),
            ai_config,
        }
    }
}

impl AiConfig {
    /// 实时加载 AI 配置：`.env` 文件优先，缺失时回退进程环境变量。
    ///
    /// 每次聊天请求都会调用一次，因此改 `.env`（模型 / 提供商 / 开关）后
    /// 无需重启，下一个请求自动生效。文件很小，逐请求读取开销可忽略。
    pub fn load_live() -> Self {
        let file_vars = read_dotenv_file(DOTENV_PATH);
        let get = |key: &str| -> String {
            if let Some(v) = file_vars.get(key) {
                v.clone()
            } else {
                std::env::var(key).unwrap_or_default()
            }
        };
        let www_root = get("WWW_ROOT");
        let tools_flag = get("AI_TOOLS_ENABLED");
        Self {
            api_key: get("AI_API_KEY"),
            model: get("AI_MODEL"),
            base_url: get("AI_BASE_URL"),
            www_root: if www_root.is_empty() {
                "/www".to_string()
            } else {
                www_root
            },
            tools_enabled: if tools_flag.is_empty() {
                true
            } else {
                tools_flag != "0" && tools_flag.to_lowercase() != "false"
            },
        }
    }
}

/// 解析 dotenv 文本为键值表（支持 `#` 注释、`export ` 前缀、单/双引号）。
fn parse_dotenv_text(text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some(eq) = line.find('=') else { continue };
        let key = line[..eq].trim();
        if key.is_empty() {
            continue;
        }
        let mut value = line[eq + 1..].trim().to_string();
        // 行尾注释：仅当值未被引号包裹时截断
        if !(value.starts_with('"') || value.starts_with('\'')) {
            if let Some(hash) = value.find(" #") {
                value.truncate(hash);
                value = value.trim_end().to_string();
            }
        }
        if value.len() >= 2
            && ((value.starts_with('"') && value.ends_with('"'))
                || (value.starts_with('\'') && value.ends_with('\'')))
        {
            value = value[1..value.len() - 1].to_string();
        }
        map.insert(key.to_string(), value);
    }
    map
}

/// 读取 `.env` 文件；文件不存在/不可读时返回空表（调用方回退环境变量）。
fn read_dotenv_file(path: &str) -> HashMap<String, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => parse_dotenv_text(&text),
        Err(_) => HashMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_dotenv_basics() {
        let map = parse_dotenv_text(
            "# comment\nAI_MODEL=doubao-x\nAI_API_KEY=\"sk-abc # not comment\"\nexport WWW_ROOT=/srv/www\nAI_TOOLS_ENABLED=0 # disable\nEMPTY=\nBADLINE\n",
        );
        assert_eq!(map.get("AI_MODEL").map(String::as_str), Some("doubao-x"));
        assert_eq!(
            map.get("AI_API_KEY").map(String::as_str),
            Some("sk-abc # not comment")
        );
        assert_eq!(map.get("WWW_ROOT").map(String::as_str), Some("/srv/www"));
        assert_eq!(map.get("AI_TOOLS_ENABLED").map(String::as_str), Some("0"));
        assert_eq!(map.get("EMPTY").map(String::as_str), Some(""));
        assert!(!map.contains_key("BADLINE"));
    }

    #[test]
    fn missing_file_falls_back_to_env() {
        let map = read_dotenv_file("/nonexistent-dir-xyz/.env");
        assert!(map.is_empty());
    }
}
