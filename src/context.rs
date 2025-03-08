use crate::config::Config;

#[derive(Default)]
pub struct Context {
    pub config: Config,
    pub owner: String,
    pub repo: String,

    pub octocrab: octocrab::Octocrab,
    pub token: Option<String>,

    pub local: bool,
}
