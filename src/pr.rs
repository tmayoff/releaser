use anyhow::Result;
use octocrab::models::pulls::PullRequest;

pub const PR_TITLE_PREFIX: &str = "chore(main): release";

pub async fn update_or_create(
    octocrab: &octocrab::Octocrab,
    owner: &str,
    repo: &str,
    head: &str,
    title_prefix: &str,
    body: &str,
) -> Result<()> {
    let existing_pull = find_pr(owner, repo, title_prefix).await?;

    match existing_pull {
        Some(pr) => {
            octocrab
                .pulls(owner, repo)
                .update(pr.number)
                .body(body)
                .send()
                .await?;
        }
        None => {
            let title = title_prefix;

            octocrab
                .pulls(owner, repo)
                .create(title, head, "main")
                .body(body)
                .send()
                .await?;
        }
    }

    Ok(())
}

pub async fn find_pr(owner: &str, repo: &str, title_prefix: &str) -> Result<Option<PullRequest>> {
    let pulls = octocrab::instance()
        .pulls(owner, repo)
        .list()
        .state(octocrab::params::State::Open)
        .send()
        .await?;

    let existing_pull = pulls
        .items
        .iter()
        .find(|p| match &p.title {
            Some(title) => title.starts_with(title_prefix),
            None => false,
        })
        .cloned();

    Ok(existing_pull)
}

pub fn format_body(content: &str) -> String {
    format!(
        r#"
# 🤖 I have created a release beep boop

{}

---

Release created by [releaser](https://github.com/tmayoff/releaser)
"#,
        content
    )
}
