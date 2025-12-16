use actix_web::{App, HttpServer, web};
use anyhow::Result;
use dotenv::dotenv;
use sqlx::SqlitePool;
use std::sync::Arc;

use crate::controller::app::get_apps;
use crate::controller::email::send_email_handler;
use crate::controller::state::state;
use crate::dao::app::get_all_apps;
use crate::dao::database::init_database_pool;

mod controller;
mod dao;
mod util;

/// 启动前业务逻辑
async fn after_startup(pool: &SqlitePool) -> Result<()> {
    // 查询apps数据
    let apps = get_all_apps(pool).await?;

    println!("\n📊 迁移后应用数据：");
    for app in apps {
        println!(
            "应用ID：{}，名称：{}，图片：{}，描述：{}",
            app.appid, app.name, app.image, app.description
        );
    }

    println!("\n🚀 业务服务启动成功");

    // 发送启动通知邮件
    let start_notification = format!(
        "Rust后端服务已成功启动！\n\n时间：{}\n版本：{}",
        chrono::Local::now().to_string(),
        env!("CARGO_PKG_VERSION")
    );
    let email_config = crate::util::email::EmailConfig::new(
        Some("【Rust】后端服务启动通知".to_string()),
        start_notification,
        None,
    );

    if let Err(e) = crate::util::email::send_email(email_config) {
        eprintln!("发送启动通知邮件失败：{}", e);
    } else {
        println!("已发送启动通知邮件");
    }

    Ok(())
}

#[actix_web::main]
async fn main() -> Result<()> {
    // 加载.env文件
    dotenv().ok();

    // 初始化数据库连接池
    let pool = init_database_pool().await?;
    let pool_for_after_startup = Arc::clone(&pool);

    // 创建HTTP服务器
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(Arc::clone(&pool)))
            .service(
                web::scope("/api/v1")
                    .route("/", web::get().to(state))
                    .route("/email", web::post().to(send_email_handler))
                    .route("/apps", web::get().to(get_apps))
                    .service(actix_files::Files::new("/doc", "swagger").index_file("index.html")),
            )
    })
    .bind(("127.0.0.1", 8080))?;

    // 绑定端口成功后，在服务器启动前创建异步任务执行业务逻辑
    println!("📡 服务器已绑定到127.0.0.1:8080，正在启动...");
    tokio::spawn(async move {
        if let Err(e) = after_startup(&pool_for_after_startup).await {
            eprintln!("❌ 业务逻辑启动失败: {}", e);
        }
    });

    // 启动服务器并等待其完成
    server.run().await?;

    Ok(())
}
