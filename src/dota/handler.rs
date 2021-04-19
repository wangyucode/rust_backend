use roa::{Context, Result};
use roa::body::PowerBody;
use serde::{Deserialize, Serialize};
use crate::state::State;

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
}

pub async fn get_news(ctx: &mut Context<State>) -> Result {
    let user = User { id: 123, name: "name".to_string() };
    ctx.write_json(&user)?;
    Ok(())
}

pub async fn put_news(ctx: &mut Context<State>) -> Result {
    let user: User = ctx.read_json().await?;
    println!("{:?}", user);
    Ok(())
}

pub async fn print_path(ctx: &mut Context<State>) -> Result {
    let path = ctx.req.uri.path();
    ctx.resp.write(path.to_owned());
    Ok(())
}

