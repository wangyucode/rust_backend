use roa::query::query_parser;
use roa::router::{post, Router};

use crate::auth::{auth_guard, login};
use crate::dota::handler::{get_news, print_path};

pub fn router() -> Router<()> {
    let admin = Router::new()
        .gate(auth_guard())
        .on("/dota/news", post(print_path));
    Router::new()
        .gate(query_parser)
        .on("/dota/news", get_news)
        .on("/admin/login", login)
        .include("/admin", admin)
}
