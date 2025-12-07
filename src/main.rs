use actix_web::{App, HttpServer, web};
use dotenv::dotenv;

use crate::controller::email::send_email;
use crate::controller::state::state;

mod controller;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // 加载.env文件
    dotenv().ok();

    HttpServer::new(|| {
        App::new().service(
            web::scope("/api/v1")
                .route("/", web::get().to(state))
                .route("/email", web::post().to(send_email))
                .service(actix_files::Files::new("/doc", "swagger").index_file("index.html")),
        )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
