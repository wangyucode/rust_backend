use crate::plaintext;
use thruster::{async_middleware, App, BasicContext, Request};
use super::middleware::authorize_middleware;

pub fn get_app() -> App<Request, BasicContext, ()> {
    let mut admin = App::<Request, BasicContext, ()>::new_basic();
    admin
        .use_middleware("/", async_middleware!(BasicContext, [authorize_middleware::authorize]))
        .get("/login", async_middleware!(BasicContext, [plaintext]));
    return admin;
}
