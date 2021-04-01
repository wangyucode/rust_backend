use blake2::{Blake2b, Digest};
use roa::{Context, Result};
use roa::jwt::{DecodingKey, guard};
use roa::query::{Query, query_parser};
use roa::router::{post, Router};

const SECRET: &[u8] = b"123456";



pub fn router() -> Router<()> {
    let admin = Router::new()
        .gate(guard(DecodingKey::from_secret(SECRET)))
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

async fn login(ctx: &mut Context) -> Result {
    let username = &*ctx.must_query("u")?;
    let password = &*ctx.must_query("p")?;

    let hash = Blake2b::digest(b"1");
    println!("Result: {:x}", hash);

    ctx.resp.write(format!("{},{}", username, password));
    Ok(())
}
