use crate::cmd::{
    args::{Cli, Commands, RepoCommands},
    config::Configuration,
};
use eyre::{eyre, ContextCompat, Result};
use gr_bin::{
    git::url::parse_url,
    vcs::common::{init_vcs, ForkRepository, VersionControlSettings},
};
use tracing::instrument;

#[instrument(skip_all, fields(command = ?args.command))]
pub fn fork(args: Cli, conf: Configuration) -> Result<()> {
    let Cli {
        command,
        auth,
        output,
        ..
    } = args;
    if let Commands::Repo(RepoCommands::Fork {
        source,
        repository: repository_name,
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

        let vcs = init_vcs(hostname, original, settings)?;
        let repo = vcs.fork_repository(ForkRepository { organization, name })?;

        repo.print(false, output.into());

        Ok(())
    } else {
        Err(eyre!("Invalid command!"))
    }
}
