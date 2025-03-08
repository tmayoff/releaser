pub mod config;
pub mod context;
pub mod fs;
pub mod pr;

use std::collections::HashMap;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use config::get_config;
use log::info;
use octocrab::models::repos::{CommitAuthor, RepoCommit};
use pr::find_pr;

#[derive(Clone, Debug, Subcommand)]
enum Command {
    Check {},
    PR {},
}

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    repo_url: String,

    #[arg(long)]
    token: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::new()
        .filter_module("releaser", log::LevelFilter::Debug)
        .init();

    let args = Args::parse();

    let repo_url = args.repo_url;

    let (owner, repo) = get_owner_repo(&repo_url);
    let config = get_config(&owner, &repo)
        .await
        .context("Failed to parse config")?;

    let context = context::Context {
        config,
        owner,
        repo,
        token: args.token,
    };

    match args.command {
        Command::Check {} => {
            check(&context).await?;
        }
        Command::PR {} => {
            pr(&context).await?;
        }
    }

    Ok(())
}

async fn pr(ctx: &context::Context) -> Result<()> {
    info!("Checking repo: {}/{}", ctx.owner, ctx.repo);

    let token = ctx
        .token
        .as_ref()
        .expect("PR command requires a Github token");

    let commits = get_commits_since_last_release(&ctx.owner, &ctx.repo).await?;

    let octocrab = octocrab::instance().user_access_token(token.as_str())?;
    let head = octocrab
        .commits(&ctx.owner, &ctx.repo)
        .get("HEAD")
        .await?
        .sha;

    let branch_name = "releaser-main-release";

    let changelog = conventional_commits_to_string(&commits);
    if let None = find_pr(&ctx.owner, &ctx.repo, pr::PR_TITLE_PREFIX).await? {
        info!("Existing release PR not found, creating now...");

        let _ = octocrab
            .repos(&ctx.owner, &ctx.repo)
            .create_ref(
                &octocrab::params::repos::Reference::Branch(branch_name.to_string()),
                &head,
            )
            .await;

        update_or_create_file(&octocrab, &ctx.owner, &ctx.repo, "CHANGELOG.md", &changelog).await?;
    } else {
        info!("Existing release PR found");
        update_or_create_file(&octocrab, &ctx.owner, &ctx.repo, "CHANGELOG.md", &changelog).await?;
    }

    pr::update_or_create(
        &octocrab,
        &ctx.owner,
        &ctx.repo,
        &head,
        pr::PR_TITLE_PREFIX,
        &pr::format_body(&changelog),
    )
    .await?;

    Ok(())
}

async fn update_or_create_file(
    octocrab: &octocrab::Octocrab,
    owner: &str,
    repo: &str,
    path: &str,
    content: &str,
) -> Result<()> {
    let req = octocrab.repos(owner, repo);

    let current_content =
        fs::get_file_content(octocrab, owner, repo, "releaser-main-release", path).await?;

    let update_req;

    match current_content {
        Some(current_content) => {
            if let Some(c) = current_content.content {
                if content == c {
                    return Ok(());
                }
            }

            update_req = req.update_file(
                path,
                "update changelog.md",
                content,
                current_content.sha.to_string(),
            );
        }
        None => {
            update_req = req.create_file(path, "Update changelog", content);
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

async fn check(ctx: &context::Context) -> Result<()> {
    info!("Checking repo: {}/{}", ctx.owner, ctx.repo);

    let grouped = get_commits_since_last_release(&ctx.owner, &ctx.repo).await?;

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
    _breaking: bool,
    _commit: RepoCommit,
    title: String,
    _scope: String,
    _type: ConventionalCommitType,
}

fn conventional_commits_to_string(
    commits: &HashMap<ConventionalCommitType, Vec<ConventionalCommit>>,
) -> String {
    let features = commits.get(&ConventionalCommitType::Feature);
    let fixes = commits.get(&ConventionalCommitType::Fix);

    // let mut breaking = Vec::new();
    let mut string = String::new();

    if let Some(feats) = features {
        string += "### Features";
        for f in feats {
            string += &format!("\n- {}", f.title);
        }
        string += "\n";
    }

    if let Some(fixes) = fixes {
        string += "\n### Bug fixes";
        for f in fixes {
            string += &format!("\n- {}", f.title);
        }
    }

    string
}

fn parse_commit(commit: &RepoCommit) -> Option<ConventionalCommit> {
    let commit_line: &str = commit
        .commit
        .message
        .lines()
        .collect::<Vec<&str>>()
        .first()
        .expect("Commit requires a message");

    info!("Parsing commit: {}", commit_line);

    let title;
    let commit_type: ConventionalCommitType;
    match commit_line.find(':') {
        Some(index) => {
            let keyword = commit_line.get(..index).expect("requires a keyword");
            title = commit_line
                .get((index + 1)..)
                .expect("requires content")
                .trim()
                .to_string();
            match keyword {
                "fix" => commit_type = ConventionalCommitType::Fix,
                "feat" | "feature" => commit_type = ConventionalCommitType::Feature,
                other => commit_type = ConventionalCommitType::Other(other.to_string()),
            }
        }
        None => return None,
    }

    Some(ConventionalCommit {
        _breaking: false,
        _commit: commit.clone(),
        title,
        _scope: "".to_string(),
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
