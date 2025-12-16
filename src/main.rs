use actix_web::{App, HttpServer, web};
use dotenv::dotenv;
use std::sync::Arc;

use crate::controller::app::get_apps;
use crate::controller::email::send_email;
use crate::controller::state::state;
use crate::dao::database::init_database_pool;

mod controller;
mod dao;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 加载.env文件
    dotenv().ok();

    // 初始化数据库连接池
    let pool = init_database_pool().await.unwrap();

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
    .await
}
