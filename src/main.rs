mod vcs;
use color_eyre::Result;
use vcs::common::CreatePullRequest;
use vcs::{common::VersionControl, gitlab::GitLab};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let vcs = GitLab::init(
        ("grant".to_string(), "4AuLXZqc-Ps2cLAxcwcD".to_string()),
        ("grant".to_string(), "gr".to_string()),
    );

    // let new_pr = vcs
    //     .create_pr(CreatePullRequest {
    //         title: "asd".to_string(),
    //         description: "asd".to_string(),
    //         source: "feature/test".to_string(),
    //         destination: "master".to_string(),
    //         close_source_branch: true,
    //     })
    //     .await?;

    let pr = vcs.get_pr("feature/test").await?;
    println!("{:#?}", pr);
    Ok(())
}
