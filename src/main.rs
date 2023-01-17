mod vcs;
use color_eyre::Result;
use vcs::{bitbucket::Bitbucket, common::VersionControl};

#[tokio::main]
async fn main() -> Result<()> {
    let bitbucket = Bitbucket::init(
        (
            "daniel7grant".to_string(),
            "ATBBjDd7qVpNLhzyxyKg85u3b3hk6327C40C".to_string(),
        ),
        ("thisisdone".to_string(), "bd-helm".to_string()),
    );

    let pr = bitbucket.get_pr("1").await?;
    println!("{:#?}", pr);
    Ok(())
}
