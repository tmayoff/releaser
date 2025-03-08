use std::path::PathBuf;
use std::str::FromStr;

use crate::config::ReleaseType;
use crate::{context, fs};
use anyhow::Result;

pub struct PackageInfo {
    pub name: String,
    pub version: String,
}

pub async fn bump_package_version(
    ctx: &context::Context,
    path: &str,
    package: &PackageInfo,
    release_type: &ReleaseType,
) -> Result<()> {
    match release_type {
        ReleaseType::Cargo => bump_cargo_package(ctx, path).await,
        ReleaseType::Node => todo!(),
    }
}

async fn bump_cargo_package(ctx: &context::Context, path: &str) -> Result<()> {
    Ok(())
}

pub async fn get_package_info(
    ctx: &context::Context,
    path: &str,
    package_type: &ReleaseType,
) -> Result<PackageInfo> {
    match package_type {
        ReleaseType::Cargo => get_cargo_package_name(ctx, path).await,
        ReleaseType::Node => todo!(),
    }
}

async fn get_cargo_package_name(ctx: &context::Context, path: &str) -> Result<PackageInfo> {
    let cargo_file = PathBuf::from_str(path)
        .expect("Path isn't a path")
        .join("Cargo.toml");

    let content = fs::get_file_content(&ctx, "main", cargo_file.as_os_str().to_str().unwrap())
        .await?
        .expect(&format!("Missing Cargo.toml at {}", path));

    let table = content.text.parse::<toml::Table>()?;

    Ok(PackageInfo {
        name: table["package"]["name"].as_str().unwrap().to_string(),
        version: table["package"]["version"]
            .as_str()
            .unwrap_or("")
            .to_string(),
    })
}
