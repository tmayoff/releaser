use anyhow::Result;
use releaser::fs;

#[tokio::test]
async fn get_file_content() -> Result<()> {
    let octocrab = octocrab::instance();

    let content =
        fs::get_file_content(&octocrab, "tmayoff", "releaser", "main", "README.md").await?;

    assert!(content.is_some());

    Ok(())
}

#[tokio::test]
async fn get_file_content_does_not_exist() -> Result<()> {
    let octocrab = octocrab::instance();

    let content =
        fs::get_file_content(&octocrab, "tmayoff", "releaser", "main", "DOES_NOT_EXIST").await?;

    assert!(content.is_none());

    Ok(())
}
