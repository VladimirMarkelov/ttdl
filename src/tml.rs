// use serde_derive::{Deserialize};
use toml;

#[derive(Deserialize)]
pub struct Colors {
    pub color_term: Option<String>,
    pub overdue: Option<String>,
    pub threshold: Option<String>,
    pub top: Option<String>,
    pub important: Option<String>,
    pub done: Option<String>,
    pub today: Option<String>,
    pub soon: Option<String>,
}

#[derive(Deserialize)]
pub struct Ranges {
    pub soon: Option<i32>,
    pub important: Option<String>,
}

#[derive(Deserialize)]
pub struct Global {
    pub filename: Option<String>,
    pub creation_date_auto: Option<bool>,
    pub fields: Option<String>,
}

#[derive(Deserialize)]
pub struct Conf {
    pub colors: Colors,
    pub ranges: Ranges,
    pub global: Global,
}
