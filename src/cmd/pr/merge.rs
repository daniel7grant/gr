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
use tracing::{info, instrument};

#[instrument(skip_all, fields(command = ?args.command))]
pub async fn merge(args: Cli, conf: Configuration) -> Result<()> {
    let Cli {
        command,
        branch,
        dir,
        auth,
        output,
        verbose: _,
    } = args;
    if let Commands::Pr(PrCommands::Merge { delete }) = command {
        let repository = LocalRepository::init(dir)?;
        let (remote_url, remote_branch) = repository.get_remote_branch(branch)?;
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

        // Merge the PR
        let vcs = init_vcs(hostname, repo, settings);
        let pr = vcs.get_pr_by_branch(&remote_branch).await?;
        let pr = vcs.merge_pr(pr.id, delete).await?;

        pr.print(false, output.into());

        // Checkout to the target branch
        let target_branch = pr.target;
        info!("Checking out to {} after merge.", target_branch);
        repository.checkout_remote_branch(target_branch)?;

        // Delete local branch if delete was passed
        if delete {
            repository.delete_branch(pr.source)?;
        }

        Ok(())
    } else {
        Err(eyre!("Invalid command!"))
    }
}
