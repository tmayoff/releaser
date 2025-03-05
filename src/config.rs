use std::collections::HashMap;

use serde::Deserialize;

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
