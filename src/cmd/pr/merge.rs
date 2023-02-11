use crate::cmd::{
    args::{Cli, Commands, PrCommands},
    config::Configuration,
};
use color_eyre::{
    eyre::{eyre, ContextCompat},
    Result,
};
use gr::vcs::common::init_vcs;
use gr::{
    git::{git::LocalRepository, url::parse_url},
    vcs::common::VersionControlSettings,
};
use tracing::instrument;

#[instrument]
pub async fn merge(args: Cli, conf: Configuration) -> Result<()> {
    let Cli {
        command,
        branch,
        dir,
        auth,
        ..
    } = args;
    if let Commands::Pr(PrCommands::Merge { delete }) = command {
        let repo = LocalRepository::init(dir)?;
        let (remote_url, remote_branch) = repo.get_remote_branch(branch)?;
        let (hostname, repo) = parse_url(&remote_url)?;

        // Find settings or use the auth command
        let settings = conf.find_settings(&hostname, &repo);
        let settings = if let Some(auth) = auth {
            VersionControlSettings {
                auth,
                ..settings.unwrap_or_default()
            }
        } else {
            settings.wrap_err(eyre!(
                "Authentication not found for {} in {}.",
                &hostname,
                &repo
            ))?
        };

        let vcs = init_vcs(hostname, repo, settings);
        let pr = vcs.get_pr_by_branch(&remote_branch).await?;
        let pr = vcs.merge_pr(pr.id, delete).await?;
        pr.show(false);
        Ok(())
    } else {
        Err(eyre!("Invalid command!"))
    }
}
