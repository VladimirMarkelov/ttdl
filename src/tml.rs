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
    pub old: Option<String>,
    pub default_fg: Option<String>,
}

#[derive(Deserialize)]
pub struct Ranges {
    pub soon: Option<i32>,
    pub important: Option<String>,
    pub old: Option<String>,
}

#[derive(Deserialize)]
pub struct Global {
    pub filename: Option<String>,
    pub creation_date_auto: Option<bool>,
    pub fields: Option<String>,
    pub sort: Option<String>,
    pub shell: Option<Vec<String>>,
    pub script_ext: Option<String>,
    pub script_prefix: Option<String>,
    pub first_sunday: Option<bool>,
    pub strict_mode: Option<bool>,
    pub clean_subject: Option<String>,
}

#[derive(Deserialize)]
pub struct Syntax {
    pub enabled: Option<bool>,
    pub tag_color: Option<String>,
    pub hashtag_color: Option<String>,
    pub project_color: Option<String>,
    pub context_color: Option<String>,
}

#[derive(Deserialize)]
pub struct CustomRule {
    pub range: String,
    pub color: String,
}

#[derive(Deserialize)]
pub struct CustomField {
    pub name: String, // tag name, and internal name for `--fields` option
    pub title: String,
    pub kind: String, // string, integer, float, duration, date
    pub width: u16,   // maximum width
    pub rules: Option<Vec<CustomRule>>,
}

#[derive(Deserialize)]
pub struct Conf {
    pub colors: Colors,
    pub ranges: Ranges,
    pub global: Global,
    pub syntax: Option<Syntax>,
    pub fields: Option<Vec<CustomField>>,
}
