use crate::cmd::{
    args::{Cli, Commands, RepoCommands},
    config::{Configuration, RepositoryConfig},
};
use eyre::{eyre, ContextCompat, Result};
use gr_bin::{
    git::{git::LocalRepository, url::parse_url},
    vcs::common::{init_vcs, ForkRepository, VersionControlSettings},
};
use std::{thread::sleep, time::Duration};
use tracing::instrument;

#[instrument(skip_all, fields(command = ?args.command))]
pub fn fork(args: Cli, mut conf: Configuration) -> Result<()> {
    let Cli {
        command,
        auth,
        output,
        dir,
        ..
    } = args;
    if let Commands::Repo(RepoCommands::Fork {
        source,
        repository: repository_name,
        clone,
    }) = command
    {
        let (hostname, original) = parse_url(&source)?;

        let settings = conf.find_settings(&hostname, &original);
        let settings = if let Some(auth) = auth {
            VersionControlSettings {
                auth,
                ..settings.unwrap_or_default()
            }
        } else {
            settings.wrap_err(eyre!(
                "Authentication not found for {} in {}.",
                &hostname,
                &original
            ))?
        };

        let (organization, name) = repository_name
            .map(|name| {
                if let Some((org, name)) = name.split_once('/') {
                    (Some(org.to_string()), Some(name.to_string()))
                } else {
                    (None, Some(name))
                }
            })
            .unwrap_or((None, None));

        let vcs = init_vcs(hostname.clone(), original.clone(), settings.clone())?;
        let repo = vcs.fork_repository(ForkRepository { organization, name })?;

        repo.print(false, output.into());

        let vcs = init_vcs(hostname.clone(), repo.full_name, settings)?;

        if clone {
            // Wait until we have our repository (and a little more), to let the server finish the fork
            while vcs.get_repository().is_err() {
                sleep(Duration::from_millis(200));
            }
            sleep(Duration::from_millis(1000));

            // If clone is given, clone it to the directory (or here)
            let repository = LocalRepository::init(dir.clone())?;
            repository
                .clone(repo.ssh_url, dir.clone())
                .or_else(|_| repository.clone(repo.https_url, dir))?;
        }

        // Update configuration with setting up if it is forked
        conf.vcs.entry(hostname).and_modify(|host| {
            host.repositories
                .entry(original)
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
