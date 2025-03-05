pub async fn get_file_content(
    octocrab: &octocrab::Octocrab,
    owner: &str,
    repo: &str,
    r#ref: &str,
    path: &str,
) -> Result<Option<octocrab::models::repos::Content>, octocrab::Error> {
    let content_list = octocrab
        .repos(owner, repo)
        .get_content()
        .path(path)
        .r#ref(r#ref)
        .send()
        .await;

    match content_list {
        Ok(content_list) => {
            if content_list.items.len() == 0 {
                return Ok(None);
            }

            assert!(
                content_list.items.len() == 1,
                "Get file content should return a single file"
            );

            let content = content_list
                .items
                .iter()
                .next()
                .expect("Requires at least 1 ");

            Ok(Some(content.to_owned()))
        }
        Err(octocrab::Error::GitHub { source, backtrace }) => {
            if source.status_code == 404 {
                Ok(None)
            } else {
                Err(octocrab::Error::GitHub { source, backtrace })
            }
        }
        Err(e) => Err(e),
    }
}
