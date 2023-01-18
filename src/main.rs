mod cmd;
mod git;

use cmd::config::Configuration;
use color_eyre::eyre::{eyre, ContextCompat};
use color_eyre::Result;
use git::{git::{get_branch, get_remote_data, get_repository}, url::parse_url};
use gr::vcs::common::{init_vcs, CreatePullRequest};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let conf = Configuration::new()?;

    let repo = get_repository()?;
    let branch = get_branch(&repo)?;
    let (url, branch) = get_remote_data(&repo, &branch)?;
    
    let (hostname, repo) = parse_url(&url)?;
    let vcs_type = conf.find_type(&hostname);
    let auth = conf.find_auth(&hostname, &repo).wrap_err(eyre!(
        "Authentication not found for {} {}.",
        &hostname,
        &repo
    ))?;
    let vcs = init_vcs(hostname, repo, auth, vcs_type);

    // let _ = vcs
    //     .create_pr(CreatePullRequest {
    //         title: "asd".to_string(),
    //         description: "asd".to_string(),
    //         source: "feature/test".to_string(),
    //         target: "master".to_string(),
    //         close_source_branch: true,
    //     })
    //     .await?;

    let pr = vcs.get_pr(&branch).await?;
    println!("{:#?}", pr);
    Ok(())
}
