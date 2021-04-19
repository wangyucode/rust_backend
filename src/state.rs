#[derive(Clone)]
pub struct DotaNews {
    pub url: String,
    pub title: String,
    pub image: String
}

#[derive(Clone)]
pub struct State{
    pub dota_news: Vec<DotaNews>
}