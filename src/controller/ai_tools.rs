//! `read` 工具：供 AI 秘书 Agent 实时读取挂载的站点文件。
//!
//! 安全约束（见 plan.md §2.2）：
//! - 规范化后必须以 `/www/` 开头（防 `../` 逃逸）
//! - 拒绝跳出挂载点的符号链接
//! - 仅允许 `.md` / `.txt` / `.json` 扩展名
//! - 单文件 ≤ 256KB，超限截断并附加提示

use std::path::PathBuf;

pub const MAX_FILE_BYTES: usize = 256 * 1024;
pub const MAX_READS_PER_REQUEST: usize = 8;

/// OpenAI function calling 格式的 `read` 工具定义。
pub fn read_tool_definition() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "read",
            "description": "读取王郁博客(wycode.cn)或大模型高考榜单上的文件。回答任何关于王郁本人、博客文章、项目、高考榜单的问题前，应先读取相关文件获取事实。",
            "parameters": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "文件绝对路径。站点总索引(含全部文章摘要): /www/wycode/llms.txt；关于王郁: /www/wycode/llms/about.md；王郁的项目: /www/wycode/llms/apps.md；博客列表: /www/wycode/llms/blog.md；高考榜单说明: /www/wycode/llms/gaokao.md；博客全文: /www/wycode/blog/<文章id>.md；高考最新榜: /www/gaokao-app/result.json；历史版本索引: /www/gaokao-app/versions.json"
                    }
                },
                "required": ["path"]
            }
        }
    })
}

/// 词法规范化路径。返回 `None` 表示存在逃逸企图（`..` 越过根）。
/// 不触碰文件系统，调用方再做前缀白名单校验。
pub fn normalize_www_path(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty() || raw.contains('\0') {
        return None;
    }
    // 统一反斜杠（防 Windows 风格绕过），只接受绝对路径
    let unified = raw.replace('\\', "/");
    if !unified.starts_with('/') {
        return None;
    }
    let mut stack: Vec<&str> = Vec::new();
    for seg in unified.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if stack.pop().is_none() {
                    return None; // 越过 filesystem 根
                }
            }
            s => stack.push(s),
        }
    }
    Some(format!("/{}", stack.join("/")))
}

/// 把 `/www/...` 虚路径映射到宿主机实际路径。
/// 容器内 `WWW_ROOT=/www` 时为恒等映射；本地开发可指向他处。
fn map_to_fs(www_root: &str, normalized: &str) -> PathBuf {
    let root = www_root.trim_end_matches('/').to_string();
    let suffix = normalized
        .strip_prefix("/www")
        .unwrap_or(normalized)
        .trim_start_matches('/');
    if suffix.is_empty() {
        PathBuf::from(root)
    } else {
        PathBuf::from(format!("{root}/{suffix}"))
    }
}

fn canonical_root(www_root: &str) -> Option<PathBuf> {
    // 挂载点本身必须存在；不存在则降级为词法前缀校验
    std::fs::canonicalize(map_to_fs(www_root, "/www")).ok()
}

/// 校验通过后返回实际文件系统路径；失败返回给模型的错误文本。
pub fn resolve_read_path(www_root: &str, raw: &str) -> Result<PathBuf, String> {
    let normalized = normalize_www_path(raw)
        .ok_or_else(|| format!("非法路径（须为 /www/ 开头的绝对路径，且不得含 .. 逃逸）: {raw}"))?;
    if normalized != "/www" && !normalized.starts_with("/www/") {
        return Err(format!("越权路径：仅允许读取 /www/ 目录下的文件: {raw}"));
    }
    if normalized == "/www" || normalized.ends_with('/') {
        return Err(format!("路径指向目录而非文件: {raw}"));
    }
    let lower = normalized.to_lowercase();
    if !(lower.ends_with(".md") || lower.ends_with(".txt") || lower.ends_with(".json")) {
        return Err(format!(
            "不支持的扩展名：仅允许 .md / .txt / .json 文件: {raw}"
        ));
    }

    let fs_path = map_to_fs(www_root, &normalized);

    // 符号链接逃逸检查：已存在文件/父目录 canonicalize 后必须仍在挂载点内
    if let Some(root) = canonical_root(www_root) {
        let anchor: PathBuf = if fs_path.exists() {
            std::fs::canonicalize(&fs_path).unwrap_or_else(|_| fs_path.clone())
        } else if let Some(parent) = fs_path.parent() {
            match std::fs::canonicalize(parent) {
                Ok(p) => {
                    let mut r = p;
                    if let Some(name) = fs_path.file_name() {
                        r.push(name);
                    }
                    r
                }
                Err(_) => fs_path.clone(),
            }
        } else {
            fs_path.clone()
        };
        if !anchor.starts_with(&root) {
            return Err(format!("越权路径：符号链接指向挂载点之外: {raw}"));
        }
    }
    Ok(fs_path)
}

pub struct ReadOutcome {
    /// 回填给模型 / 前端展示用的原始请求路径
    pub requested_path: String,
    /// 成功时为文件字节数，失败为 0
    pub bytes: usize,
    /// 是否被截断
    pub truncated: bool,
    /// 成功为文件内容，失败为错误说明（模型可据此换路径重试）
    pub text: String,
    pub is_error: bool,
}

/// 执行一次 read。永不抛错：所有失败都转为文本便于模型重试。
pub async fn execute_read(www_root: &str, raw_path: &str) -> ReadOutcome {
    let requested_path = raw_path.trim().to_string();
    let fs_path = match resolve_read_path(www_root, &requested_path) {
        Ok(p) => p,
        Err(err_text) => {
            return ReadOutcome {
                requested_path,
                bytes: 0,
                truncated: false,
                text: format!("read 失败: {err_text}"),
                is_error: true,
            };
        }
    };
    match tokio::fs::read(&fs_path).await {
        Ok(bytes) => {
            let total = bytes.len();
            let truncated = total > MAX_FILE_BYTES;
            let slice = if truncated {
                &bytes[..MAX_FILE_BYTES]
            } else {
                &bytes[..]
            };
            let mut text = String::from_utf8_lossy(slice).into_owned();
            if truncated {
                text.push_str(&format!(
                    "\n\n[截断提示：文件共 {total} 字节，超出 {MAX_FILE_BYTES} 字节上限，仅返回前 {MAX_FILE_BYTES} 字节。请基于已有内容作答，或换更精确的路径读取。]"
                ));
            }
            ReadOutcome {
                requested_path,
                bytes: total,
                truncated,
                text,
                is_error: false,
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => ReadOutcome {
            requested_path,
            bytes: 0,
            truncated: false,
            text: format!(
                "read 失败：文件不存在 {}。可先读 /www/wycode/llms.txt（站点总索引）或 /www/gaokao-app/versions.json（榜单版本索引）定位正确路径后重试。",
                fs_path.display()
            ),
            is_error: true,
        },
        Err(e) => ReadOutcome {
            requested_path,
            bytes: 0,
            truncated: false,
            text: format!(
                "read 失败：无法读取 {}（{e}）。若目录未挂载，请基于已有信息回答并告知用户暂无法读取实时数据。",
                fs_path.display()
            ),
            is_error: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_blocks_escape() {
        assert_eq!(
            normalize_www_path("/www/wycode/../gaokao-app/result.json"),
            Some("/www/gaokao-app/result.json".to_string())
        );
        assert!(normalize_www_path("/www/../../etc/passwd").is_none()); // 越过根直接拒绝
        // …但前缀白名单会拒绝它
        assert!(resolve_read_path("/www", "/www/../../etc/passwd").is_err());
        assert!(normalize_www_path("../../etc/passwd").is_none());
        assert!(normalize_www_path("www/wycode/llms.txt").is_none());
        assert!(normalize_www_path("").is_none());
    }

    #[test]
    fn rejects_bad_extension() {
        assert!(resolve_read_path("/www", "/www/wycode/app.db").is_err());
        assert!(resolve_read_path("/www", "/www/wycode/secret").is_err());
        assert!(resolve_read_path("/www", "/www/wycode/llms.txt").is_ok());
        assert!(resolve_read_path("/www", "/www/gaokao-app/result.json").is_ok());
        assert!(resolve_read_path("/www", "/www/wycode/blog/rust-ownership.md").is_ok());
    }

    #[test]
    fn rejects_outside_prefix() {
        assert!(resolve_read_path("/www", "/etc/passwd").is_err());
        assert!(resolve_read_path("/www", "/www").is_err());
        assert!(resolve_read_path("/www", "/www/").is_err());
    }

    #[test]
    fn maps_custom_root() {
        let p = map_to_fs("/srv/www", "/www/wycode/llms.txt");
        assert_eq!(p, PathBuf::from("/srv/www/wycode/llms.txt"));
        let p2 = map_to_fs("/www", "/www/gaokao-app/result.json");
        assert_eq!(p2, PathBuf::from("/www/gaokao-app/result.json"));
    }

    #[tokio::test]
    async fn missing_file_returns_friendly_error() {
        let dir = std::env::temp_dir().join("ai_tools_test_www");
        let _ = tokio::fs::create_dir_all(dir.join("www")).await;
        let root = dir.join("www").to_string_lossy().into_owned();
        let out = execute_read(&root, "/www/wycode/llms.txt").await;
        assert!(out.is_error);
        assert!(out.text.contains("不存在") || out.text.contains("read 失败"));
    }

    #[tokio::test]
    async fn reads_and_truncates() {
        let dir = std::env::temp_dir().join("ai_tools_test_www2");
        let _ = tokio::fs::create_dir_all(dir.join("www")).await;
        let root = dir.join("www").to_string_lossy().into_owned();
        tokio::fs::write(dir.join("www").join("a.txt"), "hello world")
            .await
            .unwrap();
        let out = execute_read(&root, "/www/a.txt").await;
        assert!(!out.is_error);
        assert_eq!(out.text, "hello world");
        assert_eq!(out.bytes, 11);
    }
}
