use anyhow::{Context, Result};
use file_id::get_file_id;
use glob::glob;
use serde::Deserialize;
use serde_json::Value;
use sqlx::SqlitePool;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time;

const LOG_DIR: &str = "data/caddy-access-logs";
const LOG_RETENTION_DAYS: f64 = 7.0;

#[derive(Debug, sqlx::FromRow)]
struct DbFileState {
    #[allow(dead_code)]
    file_name: String,
    file_id: Option<String>,
    offset: i64,
    #[allow(dead_code)]
    updated_at: i64,
}

#[derive(Debug, Deserialize)]
struct CaddyLog {
    ts: f64,
    level: Option<String>,
    logger: Option<String>,
    msg: Option<String>,
    request: Option<Request>,
    user_id: Option<String>,
    duration: Option<f64>,
    size: Option<i64>,
    status: Option<i64>,
    resp_headers: Option<Value>,
    bytes_read: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct Request {
    remote_ip: Option<String>,
    remote_port: Option<String>,
    client_ip: Option<String>,
    proto: Option<String>,
    method: Option<String>,
    host: Option<String>,
    uri: Option<String>,
    headers: Option<Value>,
    tls: Option<Value>,
}

pub fn start_caddy_log_task(pool: Arc<SqlitePool>) {
    let pool_clone = pool.clone();
    tokio::spawn(async move {
        // Wait a bit for startup to finish
        time::sleep(Duration::from_secs(5)).await;

        let mut interval = time::interval(Duration::from_secs(5));
        loop {
            interval.tick().await;
            if let Err(e) = process_logs(&pool_clone).await {
                eprintln!("❌ Caddy日志处理失败: {:?}", e);
            }
        }
    });
}

async fn process_logs(pool: &SqlitePool) -> Result<()> {
    let pattern = format!("{}/*.access.log", LOG_DIR);
    for entry in glob(&pattern).context("Failed to read glob pattern")? {
        match entry {
            Ok(path) => {
                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_string();
                let domain = file_name.trim_end_matches(".access.log").to_string();

                match process_single_file(pool, &path, &file_name, &domain).await {
                    Ok(logs) => {
                        if !logs.is_empty() {
                            for chunk in logs.chunks(20) {
                                insert_batch(pool, chunk).await?;
                            }
                        }
                    }
                    Err(e) => eprintln!("处理文件 {} 失败: {:?}", file_name, e),
                }
            }
            Err(e) => eprintln!("Glob error: {:?}", e),
        }
    }
    Ok(())
}

async fn process_single_file(
    pool: &SqlitePool,
    path: &Path,
    file_name: &str,
    domain: &str,
) -> Result<Vec<(String, CaddyLog)>> {
    // 1. 获取文件当前 ID (跨平台)
    let current_file_id = get_file_id(path)
        .map(|id| format!("{:?}", id))
        .unwrap_or_default();

    // 2. 从数据库查询状态
    let db_state: Option<DbFileState> = sqlx::query_as(
        "SELECT file_name, file_id, offset, updated_at FROM caddy_file_state WHERE file_name = ?",
    )
    .bind(file_name)
    .fetch_optional(pool)
    .await?;

    let mut current_offset = 0u64;
    let mut need_reset = false;

    if let Some(ref state) = db_state {
        // 检查 ID 是否变化 (Rotate)
        if let Some(ref saved_id) = state.file_id {
            if saved_id != &current_file_id {
                // ID 变了，发生了 Rotate
                need_reset = true;
            } else {
                // ID 没变，继续读取
                current_offset = state.offset as u64;
            }
        } else {
            // 如果数据库里没有 ID (可能是第一次运行新版代码)，
            // 且 offset 为 0，视为新文件。
            need_reset = true;
        }
    } else {
        // 数据库没记录，新文件
        need_reset = true;
    }

    if need_reset {
        current_offset = 0;
    }

    // 3. 读取文件
    let file = File::open(path)?;
    let file_size = file.metadata()?.len();

    if file_size <= current_offset {
        // 没有新内容，但也需要确保数据库中有记录（如果是新文件或Rotate后）
        if need_reset || db_state.is_none() {
            save_file_state(pool, file_name, &current_file_id, current_offset).await?;
        }
        return Ok(Vec::new());
    }

    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(current_offset))?;

    let mut new_offset = current_offset;
    let mut logs = Vec::new();
    let mut bytes_buffer = Vec::new();

    loop {
        bytes_buffer.clear();
        let bytes_read = reader.read_until(b'\n', &mut bytes_buffer)?;
        if bytes_read == 0 {
            break;
        }

        if bytes_buffer.ends_with(&[b'\n']) {
            let line_str = String::from_utf8_lossy(&bytes_buffer);
            if let Ok(log) = serde_json::from_str::<CaddyLog>(&line_str) {
                logs.push((domain.to_string(), log));
            }
            new_offset += bytes_read as u64;
        } else {
            break;
        }
    }

    // 4. 更新状态到数据库
    if new_offset > current_offset || need_reset {
        save_file_state(pool, file_name, &current_file_id, new_offset).await?;
    }

    Ok(logs)
}

async fn save_file_state(
    pool: &SqlitePool,
    file_name: &str,
    file_id: &str,
    offset: u64,
) -> Result<()> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;
    sqlx::query("INSERT OR REPLACE INTO caddy_file_state (file_name, file_id, offset, updated_at) VALUES (?, ?, ?, ?)")
        .bind(file_name)
        .bind(file_id)
        .bind(offset as i64)
        .bind(now)
        .execute(pool)
        .await?;
    Ok(())
}

async fn insert_batch(pool: &SqlitePool, logs: &[(String, CaddyLog)]) -> Result<()> {
    let mut query_builder = sqlx::QueryBuilder::new(
        "INSERT INTO caddy_access_log (
            ts, level, logger, msg, 
            remote_ip, remote_port, client_ip, proto, method, host, uri, req_headers, tls,
            bytes_read, user_id, duration, size, status, resp_headers, domain
        ) ",
    );

    query_builder.push_values(logs, |mut b, (domain, log)| {
        b.push_bind(log.ts);
        b.push_bind(&log.level);
        b.push_bind(&log.logger);
        b.push_bind(&log.msg);

        if let Some(req) = &log.request {
            b.push_bind(&req.remote_ip);
            let port = req.remote_port.as_ref().and_then(|p| p.parse::<i32>().ok());
            b.push_bind(port);
            b.push_bind(&req.client_ip);
            b.push_bind(&req.proto);
            b.push_bind(&req.method);
            b.push_bind(&req.host);
            b.push_bind(&req.uri);
            b.push_bind(req.headers.as_ref().map(|v| v.to_string()));
            b.push_bind(req.tls.as_ref().map(|v| v.to_string()));
        } else {
            b.push_bind(Option::<String>::None);
            b.push_bind(Option::<i32>::None);
            b.push_bind(Option::<String>::None);
            b.push_bind(Option::<String>::None);
            b.push_bind(Option::<String>::None);
            b.push_bind(Option::<String>::None);
            b.push_bind(Option::<String>::None);
            b.push_bind(Option::<String>::None);
            b.push_bind(Option::<String>::None);
        }

        b.push_bind(log.bytes_read);
        b.push_bind(&log.user_id);
        b.push_bind(log.duration);
        b.push_bind(log.size);
        b.push_bind(log.status);
        b.push_bind(log.resp_headers.as_ref().map(|v| v.to_string()));
        b.push_bind(domain);
    });

    let query = query_builder.build();
    query.execute(pool).await?;
    Ok(())
}

pub async fn clean_old_logs(pool: &SqlitePool) -> Result<()> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs_f64();
    let threshold = now - (LOG_RETENTION_DAYS * 24.0 * 60.0 * 60.0);

    sqlx::query("DELETE FROM caddy_access_log WHERE ts < ?")
        .bind(threshold)
        .execute(pool)
        .await?;
    Ok(())
}
