use crate::context::Context;

pub struct Content {
    pub text: String,
    pub sha: Option<String>,
}

pub async fn get_file_content(
    ctx: &Context,
    r#ref: &str,
    path: &str,
) -> Result<Option<Content>, octocrab::Error> {
    let content_list = ctx
        .octocrab
        .clone()
        .repos(&ctx.owner, &ctx.repo)
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

            Ok(Some(Content {
                text: content.decoded_content().unwrap(),
                sha: Some(content.sha.clone()),
            }))
        }
        Err(octocrab::Error::GitHub { source, backtrace }) => {
            if source.status_code == 404 {
                Ok(None)
            } else {
                Err(octocrab::Error::GitHub {
                    source: source.clone(),
                    backtrace,
                })
            }
        }
        Err(e) => Err(e),
    }
}
