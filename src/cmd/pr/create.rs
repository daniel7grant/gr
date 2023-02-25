use crate::cmd::{
    args::{Cli, Commands, PrCommands},
    config::{Configuration, RepositoryConfig},
};
use color_eyre::{
    eyre::{eyre, ContextCompat},
    Result,
};
use gr::vcs::common::{init_vcs, CreatePullRequest};
use gr::{
    git::{git::LocalRepository, url::parse_url},
    vcs::common::VersionControlSettings,
};
use tracing::{info, instrument};

#[instrument(skip_all, fields(command = ?args.command))]
pub async fn create(args: Cli, mut conf: Configuration) -> Result<()> {
    let Cli {
        command,
        branch,
        dir,
        auth,
        output,
        verbose: _,
    } = args;
    if let Commands::Pr(PrCommands::Create {
        message,
        description,
        target,
        delete,
        open,
        reviewers,
        merge,
    }) = command
    {
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

        let is_default_branch = target.is_none();

        let vcs = init_vcs(hostname.clone(), repo.clone(), settings);

        let reviewers = reviewers.unwrap_or_default();

        let mut pr = vcs
            .create_pr(CreatePullRequest {
                title: message,
                description: description.unwrap_or_default(),
                source: remote_branch,
                target,
                close_source_branch: delete,
                reviewers,
            })
            .await?;

        pr.print(open, output.into());

        // Merge the PR instantly if merge is passed
        if merge {
            info!("Merging pull request {} instantly.", pr.id);
            pr = vcs.merge_pr(pr.id, false).await?;

            let target_branch = pr.target.clone();
            info!("Checking out to {} after merge.", target_branch);
            repository.checkout_remote_branch(target_branch)?;

            // Delete local branch if delete was passed
            if delete {
                repository.delete_branch(pr.source)?;
            }
        }

        // Save default branch to config for caching
        if is_default_branch {
            conf.vcs.entry(hostname).and_modify(|host| {
                host.repositories
                    .entry(repo)
                    .and_modify(|repo| repo.default_branch = Some(pr.target.clone()))
                    .or_insert(RepositoryConfig {
                        auth: None,
                        default_branch: Some(pr.target.clone()),
                    });
            });

            conf.save()?;
        }

        Ok(())
    } else {
        Err(eyre!("Invalid command!"))
    }
}
