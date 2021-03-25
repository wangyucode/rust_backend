use actix_web::{web, App, HttpServer};
use routes::{admin, public};

mod routes {
    pub mod admin;
    pub mod public;
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().service(
            // prefixes all resources and routes attached to it...
            web::scope("/rust")
                .service(public::routes(web::scope("/public")))
                .service(admin::routes(web::scope("/admin")))
        )
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await
}
