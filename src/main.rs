use actix_web::{App, HttpServer, web};

use routes::{admin, public};

mod routes;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(
                // prefixes all resources and routes attached to it...
                public::routes(web::scope("/rust"))
            )
            .service(
                admin::routes(web::scope("/rust/admin"))
            )
    })
        .bind("0.0.0.0:8080")?
        .run()
        .await
}
