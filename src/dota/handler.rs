use roa::{Context, Result};
use roa::body::PowerBody;
use serde::{Deserialize, Serialize};
use crate::state::{State, DotaNews};

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
}

pub async fn get_news(ctx: &mut Context<State>) -> Result {
    ctx.write_json(ctx.dota_news.lock().unwrap().as_slice())?;
    Ok(())
}

pub async fn put_news(ctx: &mut Context<State>) -> Result {
    let news: Vec<DotaNews> = ctx.read_json().await?;
    println!("{:?}", news);
    let mut state_news = ctx.dota_news.lock().unwrap();
    state_news.clear();
    state_news.clone_from_slice(news.as_slice());
    Ok(())
}

