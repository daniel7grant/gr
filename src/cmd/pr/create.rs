use crate::cmd::{
    args::{Commands, PrCommands},
    config::{Configuration, RepositoryConfig},
};
use color_eyre::{
    eyre::{eyre, ContextCompat},
    Result,
};
use gr::git::{git::LocalRepository, url::parse_url};
use gr::vcs::common::{init_vcs, CreatePullRequest};

pub async fn create(command: Commands, mut conf: Configuration) -> Result<()> {
    if let Commands::Pr(PrCommands::Create {
        message,
        description,
        branch,
        dir,
        target,
        delete,
        open,
        reviewers,
    }) = command
    {
        let repo = LocalRepository::init(dir)?;
        let (remote_url, remote_branch) = repo.get_remote_branch(branch)?;
        let (hostname, repo) = parse_url(&remote_url)?;

        let settings = conf.find_settings(&hostname, &repo).wrap_err(eyre!(
            "Authentication not found for {} {}.",
            &hostname,
            &repo
        ))?;

        let is_default_branch = target.is_none();

        let vcs = init_vcs(hostname.clone(), repo.clone(), settings);

        let reviewers = reviewers.unwrap_or_default();

        let pr = vcs
            .create_pr(CreatePullRequest {
                title: message,
                description: description.unwrap_or_default(),
                source: remote_branch,
                target,
                close_source_branch: delete,
                reviewers,
            })
            .await?;
        pr.show(open);

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
