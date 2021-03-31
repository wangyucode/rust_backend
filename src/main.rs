use std::time::Instant;
use thruster::{async_middleware, middleware_fn};
use thruster::{App, BasicContext as Ctx, Request, Server, ThrusterServer};
use thruster::{MiddlewareNext, MiddlewareResult};

mod public;
mod admin;
mod utils;

#[middleware_fn]
async fn profile(mut context: Ctx, next: MiddlewareNext<Ctx>) -> MiddlewareResult<Ctx> {
    let start_time = Instant::now();
    println!(
        "[before][{:?}] {} -- {}",
        start_time,
        context.request.method(),
        context.request.path()
    );
    context = utils::next_context(context, next).await;
    let elapsed_time = start_time.elapsed();
    println!(
        "[after][{}ms] {} -- {}",
        elapsed_time.as_micros(),
        context.request.method(),
        context.request.path()
    );

    Ok(context)
}

#[middleware_fn]
pub async fn plaintext(mut context: Ctx, _next: MiddlewareNext<Ctx>) -> MiddlewareResult<Ctx> {
    let val = "Hello, World!";
    context.body(val);
    Ok(context)
}

#[middleware_fn]
async fn four_oh_four(mut context: Ctx, _next: MiddlewareNext<Ctx>) -> MiddlewareResult<Ctx> {
    context.status(404);
    context.body("Whoops! That route doesn't exist!");
    Ok(context)
}

fn main() {
    let mut app = App::<Request, Ctx, ()>::new_basic();
    app.set404(async_middleware!(Ctx, [four_oh_four]))
        .use_middleware("/", async_middleware!(Ctx, [profile]))
        .use_sub_app("/rust/public", public::public_app::get_app())
        .use_sub_app("/rust/admin", admin::admin_app::get_app());

    let server = Server::new(app);
    server.start("127.0.0.1", 8080);
}
