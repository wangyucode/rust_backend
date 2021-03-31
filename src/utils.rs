use thruster::{BasicContext as Ctx, MiddlewareNext};

pub async fn next_context(context: Ctx, next: MiddlewareNext<Ctx>) -> Ctx {
    let res = next(context).await;
    match res {
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
    }
}
