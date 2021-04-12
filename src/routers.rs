use crate::auth::{auth_guard, login};
use roa::{Context, Result};

use roa::query::query_parser;
use roa::router::{post, Router};

pub fn router() -> Router<()> {
    let admin = Router::new()
        .gate(auth_guard())
        .on("/dota/news", post(print_path));
    Router::new()
        .gate(query_parser)
        .on("/dota/news", print_path)
        .on("/admin/login", login)
        .include("/admin", admin)
}

async fn print_path(ctx: &mut Context) -> Result {
    let path = ctx.req.uri.path();
    ctx.resp.write(path.to_owned());
    Ok(())
}
