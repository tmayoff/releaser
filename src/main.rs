use std::{any::Any, collections::HashMap};

use anyhow::Result;
use clap::{Parser, Subcommand};
use log::info;
use octocrab::models::{
    IssueState,
    repos::{CommitAuthor, RepoCommit},
};

#[derive(Clone, Debug, Subcommand)]
enum Command {
    Check {
        #[arg(long)]
        repo_url: String,
    },
    PR {
        #[arg(long)]
        repo_url: String,
        #[arg(long)]
        token: String,
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
async fn main() -> Result<()> {
    env_logger::Builder::new()
        .filter_module("releaser", log::LevelFilter::Debug)
        .init();

    let args = Args::parse();
    println!("{:?}", args);

    match args.command {
        Command::Check { repo_url } => {
            check(&repo_url).await?;
        }
        Command::PR { repo_url, token } => {
            pr(&repo_url, &token).await?;
        }
    }

    Ok(())
}

async fn pr(repo_url: &str, token: &str) -> Result<()> {
    info!("Checking repo: {}", repo_url);
    let (owner, repo) = get_owner_repo(repo_url);

    let _commits = get_commits_since_last_release(&owner, &repo).await?;

    let octocrab = octocrab::instance().user_access_token(token)?;
    let head = octocrab.commits(&owner, &repo).get("HEAD").await?.sha;

    let pulls = octocrab.pulls(&owner, &repo).list().send().await?;

    let last_release_pr =  pulls.items.iter().find(|p| match &p.title {
        Some(title) => title.starts_with("chore(main): release") ,
        None => false,
    } && match &p.body_text {
        Some(body) => body.contains("created by releaser"),
        None => false,
    }&& &p.state.as_ref().expect("PR should have a state") == &&IssueState::Closed).cloned();

    match last_release_pr {
        Some(_) => {
            info!("Found existing release PR");
            update_or_create_file(&octocrab, &owner, &repo, "CHANGELOG.md").await?;
        }
        None => {
            info!("Release PR not found, creating a new one");

            let _ = octocrab
                .repos(&owner, &repo)
                .create_ref(
                    &octocrab::params::repos::Reference::Branch(
                        "releaser-main-release".to_string(),
                    ),
                    head,
                )
                .await;

            update_or_create_file(&octocrab, &owner, &repo, "CHANGELOG.md").await?;

            Some(
                octocrab
                    .pulls(&owner, &repo)
                    .create("chore(main): release", "releaser-main-release", "main")
                    .send()
                    .await?,
            );
        }
    }

    Ok(())
}

async fn update_or_create_file(
    octocrab: &octocrab::Octocrab,
    owner: &str,
    repo: &str,
    path: &str,
) -> Result<()> {
    // View if file exists

    let req = octocrab.repos(owner, repo);

    let content = req
        .get_content()
        .path(path)
        .r#ref("releaser-main-release")
        .send()
        .await;

    let update_req;

    match content {
        Ok(content) => {
            let content = content
                .items
                .iter()
                .next()
                .expect("Expected the file to exist");

            update_req = req.update_file(path, "update changelog.md", "", content.sha.to_string());
        }
        Err(_) => {
            update_req = req.create_file(path, "Update changelog", "UPDATE CHANGELOG");
        }
    }

    update_req
        .branch("releaser-main-release")
        .author(CommitAuthor {
            name: "releaser".to_owned(),
            email: "releaser@releaser.com".to_owned(),
            date: None,
        })
        .commiter(CommitAuthor {
            name: "releaser".to_owned(),
            email: "releaser@releaser.com".to_owned(),
            date: None,
        })
        .send()
        .await?;

    Ok(())
}

async fn check(repo_url: &str) -> Result<()> {
    info!("Checking repo: {}", repo_url);
    let (owner, repo) = get_owner_repo(repo_url);

    let grouped = get_commits_since_last_release(&owner, &repo).await?;

    info!("Commits since last release");
    for (_type, commits) in grouped {
        info!("{:?}", _type);
        for commit in commits {
            info!("\t{}", commit.title);
        }
    }
    Ok(())
}

async fn get_commits_since_last_release(
    owner: &str,
    repo: &str,
) -> Result<HashMap<ConventionalCommitType, Vec<ConventionalCommit>>> {
    let octocrab = octocrab::instance();

    let latest_release = octocrab.repos(owner, repo).releases().get_latest().await;

    match latest_release {
        Ok(release) => {
            let commits = octocrab
                .repos(owner, repo)
                .list_commits()
                .since(release.published_at.unwrap())
                .send()
                .await?;

            let commits = commits
                .items
                .iter()
                .filter_map(|c| parse_commit(c))
                .collect::<Vec<ConventionalCommit>>();

            Ok(group_by_category(&commits))
        }
        Err(_) => {
            let commits = octocrab.repos(owner, repo).list_commits().send().await?;

            let commits = commits
                .items
                .iter()
                .filter_map(|c| parse_commit(c))
                .collect::<Vec<ConventionalCommit>>();

            Ok(group_by_category(&commits))
        }
    }
}

fn group_by_category(
    commits: &Vec<ConventionalCommit>,
) -> HashMap<ConventionalCommitType, Vec<ConventionalCommit>> {
    let mut grouped_data = HashMap::new();

    for commit in commits.iter() {
        grouped_data
            .entry(commit._type.clone())
            .or_insert(vec![])
            .push(commit.clone());
    }

    grouped_data
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
enum ConventionalCommitType {
    Fix,
    Feature,
    Other(String),
}

#[derive(Clone, Debug)]
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
        None => return None,
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
