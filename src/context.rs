use crate::config::Config;

pub struct Context {
    pub config: Config,
    pub owner: String,
    pub repo: String,
    pub token: Option<String>,
}
