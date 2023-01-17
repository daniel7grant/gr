mod vcs;
use color_eyre::Result;
use vcs::{bitbucket::Bitbucket, common::VersionControl};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let bitbucket = Bitbucket::init(
        (
            "daniel7grant".to_string(),
            "ATBBjDd7qVpNLhzyxyKg85u3b3hk6327C40C".to_string(),
        ),
        ("thisisdone".to_string(), "bd-helm".to_string()),
    );

    let pr = bitbucket.get_pr("feature/nodeport-wpscan").await?;
    println!("{:#?}", pr);
    Ok(())
}
