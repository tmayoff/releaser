use std::collections::HashMap;

use anyhow::Result;
use clap::{Parser, Subcommand};
use log::info;
use octocrab::models::repos::RepoCommit;

#[derive(Clone, Debug, Subcommand)]
enum Command {
    Check {
        #[arg(long)]
        repo_url: String,
    },
}

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    repo_url: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::new()
        .filter_module("releaser", log::LevelFilter::Debug)
        .init();

    let args = Args::parse();
    println!("{:?}", args);

    match args.command {
        Command::Check { repo_url } => {
            let _ = check(&repo_url).await;
        }
    }
}

async fn check(repo_url: &str) -> Result<()> {
    info!("Checking repo: {}", repo_url);
    let (owner, repo) = get_owner_repo(repo_url);

    let octocrab = octocrab::instance();

    let latest_release = octocrab.repos(&owner, &repo).releases().get_latest().await;

    match latest_release {
        Ok(release) => {
            let commits = octocrab
                .repos(&owner, &repo)
                .list_commits()
                .since(release.published_at.unwrap())
                .send()
                .await?;

            let commits = commits
                .items
                .iter()
                .filter_map(|c| parse_commit(c))
                .collect::<Vec<ConventionalCommit>>();

            let grouped = group_by_category(&commits);

            for (_type, commits) in grouped {
                info!("{:?}", _type);
                for commit in commits {
                    info!("\t{}", commit.title);
                }
            }
        }
        Err(_) => todo!("no previous releases exist"),
    }

    Ok(())
}

fn group_by_category(
    commits: &Vec<ConventionalCommit>,
) -> HashMap<ConventionalCommitType, Vec<&ConventionalCommit>> {
    let mut grouped_data = HashMap::new();

    for commit in commits.iter() {
        grouped_data
            .entry(commit._type.clone())
            .or_insert(vec![])
            .push(commit);
    }

    grouped_data
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
enum ConventionalCommitType {
    Fix,
    Feature,
    Other(String),
}

#[derive(Debug)]
struct ConventionalCommit {
    breaking: bool,
    commit: RepoCommit,
    title: String,
    scope: String,
    _type: ConventionalCommitType,
}

fn parse_commit(commit: &RepoCommit) -> Option<ConventionalCommit> {
    let commit_line: &str = commit
        .commit
        .message
        .lines()
        .collect::<Vec<&str>>()
        .first()
        .expect("Commit requires a message");

    let commit_type: ConventionalCommitType;
    match commit_line.find(':') {
        Some(index) => {
            let keyword = commit_line.get(..index).expect("requires a keyword");

            match keyword {
                "fix" => commit_type = ConventionalCommitType::Fix,
                "feat" | "feature" => commit_type = ConventionalCommitType::Feature,
                other => commit_type = ConventionalCommitType::Other(other.to_string()),
            }
        }
        None => todo!(),
    }

    Some(ConventionalCommit {
        breaking: false,
        commit: commit.clone(),
        title: commit_line.to_string(),
        scope: "".to_string(),
        _type: commit_type,
    })
}

fn get_owner_repo(repo: &str) -> (String, String) {
    let parts: Vec<&str> = repo.split("/").collect::<Vec<&str>>();
    assert_eq!(parts.len(), 2, "Repo url should be in format owner/repo");

    (
        parts.get(0).unwrap().to_string(),
        parts.get(1).unwrap().to_string(),
    )
}
