use std::ops::Deref;

use anyhow::Result;
use releaser::config;
use releaser::context;
use releaser::fs;

#[tokio::test]
async fn get_file_content() -> Result<()> {
    let context = context::Context {
        config: config::Config::default(),
        owner: "tmayoff".to_string(),
        repo: "releaser".to_string(),
        octocrab: octocrab::instance().deref().clone(),
        token: None,
        local: false,
    };

    let content = fs::get_file_content(&context, "main", "README.md").await?;

    assert!(content.is_some());

    Ok(())
}

#[tokio::test]
async fn get_file_content_does_not_exist() -> Result<()> {
    let context = context::Context {
        config: config::Config::default(),
        owner: "tmayoff".to_string(),
        repo: "releaser".to_string(),
        octocrab: octocrab::instance().deref().clone(),
        token: None,
        local: false,
    };

    let content = fs::get_file_content(&context, "main", "DOES_NOT_EXIST").await?;

    assert!(content.is_none());

    Ok(())
}
