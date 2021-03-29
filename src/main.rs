use std::time::Instant;
use thruster::{async_middleware, middleware_fn};
use thruster::{App, BasicContext as Ctx, Request, Server, ThrusterServer};
use thruster::{MiddlewareNext, MiddlewareResult};

mod public;

#[middleware_fn]
async fn profile(context: Ctx, next: MiddlewareNext<Ctx>) -> MiddlewareResult<Ctx> {
    let start_time = Instant::now();

    let res = next(context).await;
    let ctx = match res {
        Ok(val) => val,
        Err(e) => {
            let mut context = e.context;
            context.body(&format!(
                "{{\"message\": \"{}\",\"success\":false}}",
                e.message
            ));
            context.status(e.status);
            context
        }
    };

    let elapsed_time = start_time.elapsed();
    println!(
        "[{}ms] {} -- {}",
        elapsed_time.as_micros(),
        ctx.request.method(),
        ctx.request.path()
    );

    Ok(ctx)
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
        .use_sub_app("/rust/public", public::public_app::get_app());

    let server = Server::new(app);
    server.start("127.0.0.1", 8080);
}
