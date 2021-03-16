use actix_web::{Responder, Scope, web};

async fn index() -> impl Responder {
    format!("Hello world!")
}


pub fn routes(scope: Scope) -> Scope {
    scope.route("/hello", web::get().to(index))
}