use crate::cmd::{
    args::{Cli, Commands, RepoCommands},
    config::{Configuration, VcsConfig},
};
use eyre::{eyre, ContextCompat, Result};
use gr_bin::{
    git::{git::LocalRepository, url::parse_url},
    vcs::common::{init_vcs, CreateRepository, VersionControlSettings},
};
use tracing::instrument;

#[instrument(skip_all, fields(command = ?args.command))]
pub fn new(args: Cli, mut conf: Configuration) -> Result<()> {
    let Cli {
        command,
        output,
        dir,
        ..
    } = args;
    if let Commands::Repo(RepoCommands::New {
        repository: repository_name,
        host,
        description,
        visibility,
        clone,
        open,
        init,
        default_branch,
        gitignore,
        license,
    }) = command
    {
        // Check if the host if full URL
        let (parsed_host, path) = parse_url(&repository_name)
            .map(|(host, path)| (Some(host), path))
            .unwrap_or((None, repository_name));

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
            ..Default::default()
        };

        // Create the new repository
        let vcs = init_vcs(hostname, "".to_string(), settings)?;
        let repo = vcs.create_repository(CreateRepository {
            name,
            organization,
            description,
            visibility: visibility.into(),
            init,
            default_branch,
            gitignore,
            license,
        })?;

        repo.print(open, output.into());

        let repository = LocalRepository::init(dir.clone())?;
        if clone {
            // If clone is given, clone it to the directory (or here)
            repository
                .clone(repo.ssh_url, dir.clone())
                .or_else(|_| repository.clone(repo.https_url, dir))?;
        } else {
            // If repository is git repo and has no remote, set remote and push to the new repo
            if repository.has_git() && repository.get_remotes()?.is_empty() {
                let branch = repository.get_branch()?;

                repository
                    .set_remote("origin".to_string(), repo.ssh_url)
                    .and_then(|_| repository.push(&branch))
                    .or_else(|_| {
                        repository
                            .set_remote("origin".to_string(), repo.https_url)
                            .and_then(|_| repository.push(&branch))
                    })?;
            }
        }

        Ok(())
    } else {
        Err(eyre!("Invalid command!"))
    }
}
