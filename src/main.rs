use std::error::Error as StdError;


use roa::App;
use roa::preload::*;

mod routers;

const A: &[u8] = env!("WYCODE_ADMIN_PASS").as_bytes();
#[async_std::main]
async fn main() -> Result<(), Box<dyn StdError>> {
    println!("{:?}",A);
    let app = App::new()
        .end(routers::router().routes("/rust")?);
    app.listen("127.0.0.1:8080", |addr| {
        println!("Server is listening on {}", addr)
    })?.await?;
    Ok(())
}
