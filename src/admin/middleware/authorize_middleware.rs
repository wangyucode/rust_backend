use crate::utils::next_context;
use thruster::{middleware_fn, BasicContext, MiddlewareNext, MiddlewareResult};

#[middleware_fn]
pub async fn authorize(
    mut context: BasicContext,
    next: MiddlewareNext<BasicContext>,
) -> MiddlewareResult<BasicContext> {
    println!("{:?}",context.request.headers());
    let ctx = match context.request.headers().get("x-auth-token") {
        Some(token) => {
            if validate(&token[0]) {
                context = next_context(context, next).await;
            } else {
                context.status(401);
            }
            context
        }
        None => {
            println!("111");
            context.status(401);
            context
        }
    };
    Ok(ctx)
}

fn validate(token: &String) -> bool {
    println!("{}", token);
    token.len() > 0
}
