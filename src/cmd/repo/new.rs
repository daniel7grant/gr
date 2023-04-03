use crate::cmd::{
    args::{Cli, Commands, RepoCommands},
    config::{Configuration, VcsConfig},
};
use eyre::{eyre, ContextCompat, Result};
use gr_bin::{
    git::url::parse_url,
    vcs::common::{init_vcs, CreateRepository, VersionControlSettings},
};
use tracing::instrument;

#[instrument(skip_all, fields(command = ?args.command))]
pub fn new(args: Cli, mut conf: Configuration) -> Result<()> {
    let Cli {
        command, output, ..
    } = args;
    if let Commands::Repo(RepoCommands::New {
        repository,
        host,
        description,
        visibility,
    }) = command
    {
        // Check if the host if full URL
        let (parsed_host, path) = parse_url(&repository)
            .map(|(host, path)| (Some(host), path))
            .unwrap_or((None, repository));

        // Check if the path can be split into parts
        let (organization, name) = match path.split_once('/') {
            Some((organization, name)) => (Some(organization.to_string()), name.to_string()),
            None => (None, path),
        };

        // Figure out the final hostname to use
        let (hostname, VcsConfig { auth, vcs_type, .. }) = host
            .clone()
            .or(parsed_host)
            .and_then(|host| conf.vcs.remove_entry(&host))
            .or_else(|| {
                // If only one VCS is used, fallback to that one
                if conf.vcs.len() == 1 {
                    conf.vcs.into_iter().next()
                } else {
                    None
                }
            })
            .wrap_err(if let Some(host) = host {
                eyre!("There is no host {host} in the configuration file.")
            } else {
                eyre!("You have to pass the server name (like github.com) in the --host flag.")
            })?;

        let settings = VersionControlSettings {
            auth,
            vcs_type,
            default_branch: None,
        };

        // Create the new repository
        let vcs = init_vcs(hostname, "".to_string(), settings)?;
        let repo = vcs.create_repository(CreateRepository {
            name,
            organization,
            description,
            visibility: visibility.into(),
        })?;

        repo.print(false, output.into());

        // TODO: clone it

        Ok(())
    } else {
        Err(eyre!("Invalid command!"))
    }
}
