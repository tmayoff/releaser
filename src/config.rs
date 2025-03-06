use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;

use crate::fs;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReleaseType {
    Cargo,
    Node,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Package {
    release_type: ReleaseType,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    packages: HashMap<String, Package>,
}

pub async fn get_config(owner: &str, repo: &str) -> Result<Config> {
    let content = fs::get_file_content(
        &octocrab::instance(),
        owner,
        repo,
        "main",
        "release-please-config.json",
    )
    .await?
    .expect("Couldn't find config file")
    .decoded_content()
    .expect("Config file shouldn't be empty");

    Ok(serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", content))?)
}

#[cfg(test)]
mod tests {

    use anyhow::Result;

    use crate::config::ReleaseType;

    use super::Config;

    #[test]
    fn json_config() -> Result<()> {
        let config = r#"
            {
                "packages": {
                    ".": {
                        "release-type": "node"
                    }
                }
            }
        "#;

        let config: Config = serde_json::from_str(config)?;

        assert_eq!(config.packages.len(), 1);

        let dot = config.packages.get(".").expect("Expected package at '.'");

        assert!(matches!(dot.release_type, ReleaseType::Node));

        Ok(())
    }
}
