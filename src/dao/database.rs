use anyhow::Result;
use sqlx::{SqlitePool, migrate::Migrator, sqlite::SqlitePoolOptions};
use std::path::Path;
use std::sync::Arc;

/// 初始化数据库连接池 + 执行迁移
pub async fn init_database_pool() -> Result<Arc<SqlitePool>> {
    // 从环境变量读取数据库URL
    let db_file = "./db/sqlite.db";
    // 文件不存在时，创建文件
    if !Path::new(&db_file).exists() {
        std::fs::create_dir_all(Path::new(&db_file).parent().unwrap())?;
        std::fs::File::create(&db_file)?;
    }

    let db_url = format!("sqlite://{}", db_file);

    // 创建连接池
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect(&db_url)
        .await?;

    // 执行迁移：加载migrations目录下的所有未执行脚本
    let migrations_dir = Path::new("./db/migrations");
    if migrations_dir.exists() {
        let migrator = Migrator::new(migrations_dir).await?;
        migrator.run(&pool).await?;
    } else {
        eprintln!("⚠️  未找到迁移目录: {}", migrations_dir.display());
    }

    Ok(Arc::new(pool))
}
