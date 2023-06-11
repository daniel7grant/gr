use crate::cmd::{
    args::{Cli, Commands, RepoCommands},
    config::{Configuration, RepositoryConfig},
};
use eyre::{eyre, ContextCompat, Result};
use gr_bin::vcs::common::init_vcs;
use gr_bin::{git::git::LocalRepository, vcs::common::VersionControlSettings};
use tracing::instrument;

#[instrument(skip_all, fields(command = ?args.command))]
pub fn get(args: Cli, mut conf: Configuration) -> Result<()> {
    let Cli {
        command,
        branch,
        dir,
        auth,
        output,
        ..
    } = args;
    if let Commands::Repo(RepoCommands::Get { open }) = command {
        let repository = LocalRepository::init(dir)?;
        let (hostname, repository_name, ..) = repository.get_parsed_remote(branch)?;

        // Find settings or use the auth command
        let settings = conf.find_settings(&hostname, &repository_name);
        let settings = if let Some(auth) = auth {
            VersionControlSettings {
                auth,
                ..settings.unwrap_or_default()
            }
        } else {
            settings.wrap_err(eyre!(
                "Authentication not found for {} in {}.",
                &hostname,
                &repository_name
            ))?
        };

        let vcs = init_vcs(hostname.clone(), repository_name.clone(), settings)?;

        let repo = vcs.get_repository()?;
        repo.print(open, output.into());

        // Update configuration with setting up if it is forked
        conf.vcs.entry(hostname).and_modify(|host| {
            host.repositories
                .entry(repository_name)
                .and_modify(|r| r.fork = repo.forked_from.is_some())
                .or_insert(RepositoryConfig {
                    fork: repo.forked_from.is_some(),
                    ..Default::default()
                });
        });
        conf.save()?;

        Ok(())
    } else {
        Err(eyre!("Invalid command!"))
    }
}
