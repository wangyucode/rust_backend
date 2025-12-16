use actix_web::{App, HttpServer, web};
use anyhow::Result;
use dotenv::dotenv;
use sqlx::SqlitePool;
use std::sync::Arc;

use crate::controller::app::get_apps;
use crate::controller::email::send_email;
use crate::controller::state::state;
use crate::dao::app::get_all_apps;
use crate::dao::database::init_database_pool;

mod controller;
mod dao;

/// 启动前业务逻辑
async fn start_business_logic(pool: &SqlitePool) -> Result<()> {
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
    Ok(())
}

#[actix_web::main]
async fn main() -> Result<()> {
    // 加载.env文件
    dotenv().ok();

    // 初始化数据库连接池
    let pool = init_database_pool().await?;

    // 启动业务逻辑
    start_business_logic(&pool).await?;

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(Arc::clone(&pool)))
            .service(
                web::scope("/api/v1")
                    .route("/", web::get().to(state))
                    .route("/email", web::post().to(send_email))
                    .route("/apps", web::get().to(get_apps))
                    .service(actix_files::Files::new("/doc", "swagger").index_file("index.html")),
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await?;

    Ok(())
}
