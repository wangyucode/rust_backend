use blake2::{Blake2b, Digest};
use jsonwebtoken::{encode, EncodingKey, Header};
use roa::jwt::{guard, DecodingKey, JwtGuard};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use roa::query::Query;
use roa::{Context, Result};

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

pub async fn login(ctx: &mut Context) -> Result {
    let username = &*ctx.must_query("u")?;
    let password = &*ctx.must_query("p")?;
    // TODO check the username and password;
    println!("{},{}", username, password);

    let claims = Claims {
        exp: (SystemTime::now() + Duration::from_secs(60))
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
    ctx.resp.write(jwt);
    Ok(())
}
