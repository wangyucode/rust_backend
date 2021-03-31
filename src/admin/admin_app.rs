use thruster::{App, async_middleware, BasicContext, Request};

use super::dota::dota_app;
use super::middleware::authorize_middleware;

pub fn get_app() -> App<Request, BasicContext, ()> {
    let mut admin = App::<Request, BasicContext, ()>::new_basic();
    admin
        .use_middleware("/", async_middleware!(BasicContext, [authorize_middleware::authorize]))
        .get("/login", async_middleware!(BasicContext, [plaintext]))
        .use_sub_app("/dota", dota_app::get_app());
    return admin;
}

#[middleware_fn]
pub async fn authorize(
    mut context: BasicContext,
    next: MiddlewareNext<BasicContext>,
) -> MiddlewareResult<BasicContext> {
    println!("{}", context.request.path());
    let ctx = if context.request.path() == "/rust/admin/login" {
        context
    } else {
        match context.request.headers().get("x-auth-token") {
            Some(token) => {
                if validate(&token[0]) {
                    context = next_context(context, next).await;
                } else {
                    context.status(401);
                }
                context
            }
            None => {
                context.status(401);
                context
            }
        }
    };

    Ok(ctx)
}
