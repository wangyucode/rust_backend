use crate::after_startup::after_startup;
use crate::controller::{clipboard, comment, email, state, wechat};
use crate::dao::database::init_database_pool;
use actix_web::{App, HttpServer, web};
use anyhow::Result;
use dotenv::dotenv;
use std::sync::Arc;

mod after_startup;
mod controller;
mod dao;
mod util;

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
                    .route("/", web::get().to(state::state))
                    .route("/email", web::post().to(email::send_email_handler))
                    .route("/wechat/apps", web::get().to(wechat::get_apps))
                    .route("/comment", web::get().to(comment::get_comments))
                    .route("/comment", web::post().to(comment::post_comment))
                    .route("/clipboard/{id}", web::get().to(clipboard::get_by_id))
                    .route(
                        "/clipboard/openid/{openid}",
                        web::post().to(clipboard::get_by_openid),
                    )
                    .route(
                        "/clipboard/wx/{code}",
                        web::get().to(clipboard::get_by_wx_code),
                    )
                    .route("/clipboard", web::post().to(clipboard::save_by_id))
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
