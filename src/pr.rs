use anyhow::Result;
use octocrab::models::IssueState;

pub async fn update_or_create(
    octocrab: &octocrab::Octocrab,
    owner: &str,
    repo: &str,
    title_prefix: &str,
    body: &str,
) -> Result<()> {
    let pulls = octocrab.pulls(owner, repo).list().send().await?;

    let existing_pull= pulls
        .items
        .iter()
        .find(|p| match &p.title {
            Some(title) => title.starts_with(title_prefix),
            None => false,
   }&& &p.state.as_ref().expect("PR should have a state") != &&IssueState::Closed).cloned();

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
            let head = "";
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
