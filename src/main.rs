mod vcs;
use color_eyre::Result;
use vcs::common::CreatePullRequest;
use vcs::{common::VersionControl, github::GitHub};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let vcs = GitHub::init(
        (
            "daniel7grant".to_string(),
            "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".to_string(),
        ),
        ("daniel7grant".to_string(), "dvali".to_string()),
    );

    let _ = vcs
        .create_pr(CreatePullRequest {
            title: "asd".to_string(),
            description: "asd".to_string(),
            source: "feature/rethink-types2".to_string(),
            target: "master".to_string(),
            close_source_branch: true,
        })
        .await?;

    let pr = vcs.get_pr("feature/rethink-types2").await?;
    println!("{:#?}", pr);
    Ok(())
}
