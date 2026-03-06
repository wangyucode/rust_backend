use crate::dao::blog;
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;

pub fn start_clean_visit_task(pool: Arc<SqlitePool>) {
    let pool_clone = pool.clone();
    tokio::spawn(async move {
        // Run cleanup every 24 hours
        let mut interval = time::interval(Duration::from_secs(24 * 60 * 60));
        loop {
            interval.tick().await;
            if let Err(e) = blog::clean_old_visits(&pool_clone).await {
                eprintln!("❌ 清理旧访问记录失败: {:?}", e);
            }
        }
    });
}
