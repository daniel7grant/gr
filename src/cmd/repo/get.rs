use crate::cmd::{
    args::{Cli, Commands, RepoCommands},
    config::Configuration,
};
use eyre::{eyre, ContextCompat, Result};
use gr_bin::vcs::common::init_vcs;
use gr_bin::{git::git::LocalRepository, vcs::common::VersionControlSettings};
use tracing::instrument;

#[instrument(skip_all, fields(command = ?args.command))]
pub fn get(args: Cli, conf: Configuration) -> Result<()> {
    let Cli {
        command,
        branch,
        dir,
        auth,
        output,
        verbose: _,
    } = args;
    if let Commands::Repo(RepoCommands::Get { open }) = command {
        let repository = LocalRepository::init(dir)?;
        let (hostname, repo, ..) = repository.get_parsed_remote(branch)?;

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

        let repo = vcs.get_repository()?;
        repo.print(open, output.into());
        Ok(())
    } else {
        Err(eyre!("Invalid command!"))
    }
}
