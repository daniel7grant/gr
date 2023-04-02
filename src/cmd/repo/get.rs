use crate::cmd::{
    args::{Cli, Commands, PrCommands},
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
        output: _,
        verbose: _,
    } = args;
    if let Commands::Pr(PrCommands::Get { .. }) = command {
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

        let _ = init_vcs(hostname, repo, settings);
        
        Err(eyre!("Getting information about current repo is not implemented"))
    } else {
        Err(eyre!("Invalid command!"))
    }
}
