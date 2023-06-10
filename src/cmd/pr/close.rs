use crate::cmd::{
    args::{Cli, Commands, PrCommands},
    config::Configuration,
};
use eyre::{eyre, ContextCompat, Result};
use gr_bin::vcs::common::init_vcs;
use gr_bin::{git::git::LocalRepository, vcs::common::VersionControlSettings};
use tracing::instrument;

#[instrument(skip_all, fields(command = ?args.command))]
pub fn close(args: Cli, conf: Configuration) -> Result<()> {
    let Cli {
        command,
        branch,
        dir,
        auth,
        output,
        verbose: _,
    } = args;
    if let Commands::Pr(PrCommands::Close {}) = command {
        let repository = LocalRepository::init(dir)?;
        let (hostname, repo, remote_branch) = repository.get_parsed_remote(branch)?;
		let remote_branch = remote_branch.wrap_err(eyre!("You have to push this branch first before you can close a PR."))?;

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

        let vcs = init_vcs(hostname, repo, settings)?;
        let pr = vcs.get_pr_by_branch(&remote_branch)?;
        let pr = vcs.close_pr(pr.id)?;
        pr.print(false, output.into());
        Ok(())
    } else {
        Err(eyre!("Invalid command!"))
    }
}
