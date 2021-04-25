use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DotaNews {
    pub url: String,
    pub title: String,
    pub image: String,
}

#[derive(Clone)]
pub struct State {
    pub dota_news: Mutex<Vec<DotaNews>>
}