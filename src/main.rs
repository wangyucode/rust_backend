use crate::after_startup::after_startup;
use crate::controller::blog;
use crate::controller::clipboard;
use crate::controller::comment;
use crate::controller::config;
use crate::controller::coze;
use crate::controller::email;
use crate::controller::state;
use crate::controller::wechat;
use crate::dao::database::init_database_pool;
use actix_web::{App, HttpServer, web};
use anyhow::Result;
use dotenv::dotenv;
use std::env;
use std::sync::Arc;

mod after_startup;
mod controller;
mod dao;
mod util;

#[actix_web::main]
async fn main() -> Result<()> {
    println!("🚀 服务器启动中，v{}", env!("CARGO_PKG_VERSION"));
    // 加载.env文件
    dotenv().ok();
    println!("🔧 环境变量APP_ENV: {:?}", env::var("APP_ENV"));

    // 初始化数据库连接池
    let pool = init_database_pool().await?;
    let pool_for_after_startup = Arc::clone(&pool);
    if let Err(e) = after_startup(&pool_for_after_startup).await {
        eprintln!("❌ 业务逻辑启动失败: {}", e);
    }
    println!("🟢 开始启动HTTP服务器");
    // 创建HTTP服务器
    HttpServer::new(move || {
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
                    .route("/coze/token", web::get().to(coze::get_token))
                    .route("/config", web::get().to(config::get_config))
                    .route("/blog-view", web::get().to(blog::record_blog_view))
                    .route("/popular-posts", web::get().to(blog::get_popular_posts))
                    .service(actix_files::Files::new("/doc", "swagger").index_file("index.html")),
            )
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await?;

    Ok(())
}
