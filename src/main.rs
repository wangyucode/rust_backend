use crate::controller::blog;
use crate::controller::clipboard;
use crate::controller::comment;
use crate::controller::coze;
use crate::controller::email;
use crate::controller::roll;
use crate::controller::state;
use crate::controller::wechat;
use crate::controller::yml;
use crate::dao::database::{init_database_pool, init_log_database_pool};
use axum::{
    Router,
    routing::{get, post},
};
use dotenv::dotenv;
use sqlx::SqlitePool;
use std::env;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower::make::Shared;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::normalize_path::NormalizePathLayer;
use tower_http::trace::TraceLayer;

mod after_startup;
mod controller;
mod dao;
mod task;
mod util;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    println!("🚀 服务器启动中，v{}", env!("CARGO_PKG_VERSION"));
    // 加载.env文件
    dotenv().ok();

    // 初始化日志
    tracing_subscriber::fmt::init();

    // 初始化数据库连接池
    let pool = init_database_pool().await.expect("❌ 数据库初始化错误");
    let log_pool = init_log_database_pool().await.expect("❌ Caddy日志数据库初始化错误");

    let pool_for_after_startup = Arc::clone(&pool);
    let log_pool_for_after_startup = Arc::clone(&log_pool);
    match after_startup::after_startup(&pool_for_after_startup, &log_pool_for_after_startup).await {
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
        .route("/clipboard/openid/:openid", get(clipboard::get_by_openid))
        .route("/clipboard/wx/:code", get(clipboard::get_by_wx_code))
        .route("/clipboard", post(clipboard::save_by_id))
        .route("/coze/token", get(coze::get_token))
        .route("/blog-view", get(blog::record_blog_view))
        .route("/popular-posts", get(blog::get_popular_posts))
        .route("/roll/login", post(roll::login))
        .route("/roll/team", post(roll::set_team))
        .route("/roll/score", post(roll::report_score))
        .route(
            "/openapi.yml",
            get(|| async { include_str!("openapi.yml") }),
        )
        .route("/yml/*path", get(yml::get_yml));

    // 组装应用
    let app = Router::default()
        .nest("/api/v1", api_routes)
        .route("/yml/*path", get(yml::get_yml))
        .with_state(pool)
        .layer(TraceLayer::new_for_http())
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
