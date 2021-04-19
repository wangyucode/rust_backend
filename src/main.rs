use std::error::Error as StdError;

use roa::App;
use roa::preload::Listener;

mod routers;
mod dota;
mod state;
mod auth;

// const A: &[u8] = env!("WYCODE_ADMIN_PASS").as_bytes();
#[async_std::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    App::state(state::State { dota_news: Vec::new() })
        .end(routers::router().routes("/rust").unwrap())
        .listen("127.0.0.1:8080", |addr| {
            println!("Server is listening on {}", addr)
        })?.await?;
    Ok(())
}
