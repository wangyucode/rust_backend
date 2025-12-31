use crate::controller::blog;
use crate::controller::clipboard;
use crate::controller::comment;
use crate::controller::config;
use crate::controller::coze;
use crate::controller::email;
use crate::controller::state;
use crate::controller::wechat;
use crate::dao::database::init_database_pool;
use axum::{
    routing::{get, post},
    Router,
};
use tower::ServiceBuilder;
use tower::make::Shared;
use tower_http::normalize_path::NormalizePathLayer;
use dotenv::dotenv;
use sqlx::SqlitePool;
use std::env;
use std::sync::Arc;
use tower_http::{catch_panic::CatchPanicLayer, services::ServeDir};

mod after_startup;
mod controller;
mod dao;
mod util;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    println!("🚀 服务器启动中，v{}", env!("CARGO_PKG_VERSION"));
    // 加载.env文件
    dotenv().ok();

    // 初始化数据库连接池
    let pool = init_database_pool().await.expect("❌ 数据库初始化错误");

    // 检查 swagger 目录是否存在 (调试用途)
    if let Err(e) = tokio::fs::metadata("swagger").await {
        eprintln!("⚠️ 严重警告: 无法访问 'swagger' 目录: {}。访问 /doc 可能会导致错误。", e);
    } else {
        println!("✅ 'swagger' 目录检查通过");
    }

    let pool_for_after_startup = Arc::clone(&pool);
    match after_startup::after_startup(&pool_for_after_startup).await {
        Ok(_) => println!("✅ 业务逻辑启动成功"),
        Err(e) => {
            eprintln!("❌ 业务逻辑启动失败: {:?}", e);
        }
    };

    // 创建 API 路由
    let api_routes: Router<Arc<SqlitePool>> = Router::default()
        .route("/", get(state::state))
        .route("/email", post(email::send_email_handler))
        .route("/wechat/apps", get(wechat::get_apps))
        .route(
            "/comment",
            get(comment::get_comments).post(comment::post_comment),
        )
        .route("/clipboard/:id", get(clipboard::get_by_id))
        .route(
            "/clipboard/openid/:openid",
            get(clipboard::get_by_openid),
        )
        .route(
            "/clipboard/wx/:code",
            get(clipboard::get_by_wx_code),
        )
        .route("/clipboard", post(clipboard::save_by_id))
        .route("/coze/token", get(coze::get_token))
        .route("/config", get(config::get_config))
        .route("/blog-view", get(blog::record_blog_view))
        .route("/popular-posts", get(blog::get_popular_posts))
        .nest_service(
            "/doc",
            ServeDir::new("swagger").append_index_html_on_directories(true),
        );

    // 组装应用
    let app = Router::default()
        .nest("/api/v1", api_routes)
        .with_state(pool)
        .layer(CatchPanicLayer::new());

    let app = ServiceBuilder::new()
        .layer(NormalizePathLayer::trim_trailing_slash())
        .service(app);

    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);
    println!("尝试绑定端口: {}", port);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("✅ 端口 {} 绑定成功，服务器开始运行", port);

    axum::serve(listener, Shared::new(app)).await?;
    println!("🛑 服务器已停止");

    Ok(())
}

