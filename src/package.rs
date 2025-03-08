use std::path::PathBuf;
use std::str::FromStr;

use crate::config::ReleaseType;
use crate::{context, fs};
use anyhow::Result;

pub async fn get_package_name(
    ctx: &context::Context,
    path: &str,
    package_type: &ReleaseType,
) -> Result<String> {
    match package_type {
        ReleaseType::Cargo => get_cargo_package_name(ctx, path).await,
        ReleaseType::Node => todo!(),
    }
}

async fn get_cargo_package_name(ctx: &context::Context, path: &str) -> Result<String> {
    let cargo_file = PathBuf::from_str(path)
        .expect("Path isn't a path")
        .join("Cargo.toml");

    let content = fs::get_file_content(
        &octocrab::instance(),
        &ctx.owner,
        &ctx.repo,
        "main",
        cargo_file.as_os_str().to_str().unwrap(),
    )
    .await?
    .expect(&format!("Missing Cargo.toml at {}", path));

    let table = content
        .decoded_content()
        .expect("Missing content")
        .parse::<toml::Table>()?;

    Ok(table["package"]["name"].as_str().unwrap().to_string())
}
