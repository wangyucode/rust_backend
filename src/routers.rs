use roa::query::query_parser;
use roa::router::{put, Router};

use crate::auth::{auth_guard, login};
use crate::dota::handler::{get_news, print_path, put_news};
use crate::state::State;

pub fn router() -> Router<State> {
    let admin: Router<State> = Router::new()
        .gate(auth_guard())
        .on("/dota/news", put(put_news));
    Router::new()
        .gate(query_parser)
        .on("/dota/news", get_news)
        .on("/admin/login", login)
        .include("/admin", admin)
}
