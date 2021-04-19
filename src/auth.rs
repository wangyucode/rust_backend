use std::time::{Duration, SystemTime, UNIX_EPOCH};

use blake2::{Blake2b, Digest};
use jsonwebtoken::{encode, EncodingKey, Header};
use roa::{Context, Result};
use roa::http::header::CONTENT_TYPE;
use roa::jwt::{DecodingKey, guard, JwtGuard};
use roa::query::Query;
use serde::{Deserialize, Serialize};
use crate::state::State;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp: u64,
    uid: u64,
    uname: String,
    name: String,
    icon: String,
}

const JWT_SECRET: &[u8] = b"123456";

pub fn auth_guard() -> JwtGuard {
    guard(DecodingKey::from_secret(&Blake2b::digest(JWT_SECRET)))
}

pub async fn login(ctx: &mut Context<State>) -> Result {
    let username = &*ctx.must_query("u")?;
    let password = &*ctx.must_query("p")?;
    // TODO check the username and password;
    println!("{},{}", username, password);

    let claims = Claims {
        exp: (SystemTime::now() + Duration::from_secs(86400))
            .duration_since(UNIX_EPOCH)?
            .as_secs(),
        uid: 1,
        uname: username.to_owned(),
        name: "wayne".to_owned(),
        icon: "icon.png".to_owned(),
    };
    let jwt = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&Blake2b::digest(JWT_SECRET)),
    )?;
    ctx.resp.headers.insert(CONTENT_TYPE, "text/plain".parse().unwrap());
    ctx.resp.write(jwt);
    Ok(())
}
