use actix_web::{App, HttpServer, web};

mod public_routes;
mod admin_routes;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(
                // prefixes all resources and routes attached to it...
                public_routes::routes(web::scope("/rust"))
            )
            .service(
                admin_routes::routes(web::scope("/rust/admin"))
            )
    })
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
