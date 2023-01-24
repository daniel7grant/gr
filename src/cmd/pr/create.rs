use crate::{
    cmd::{
        args::{Commands, PrCommands},
        config::Configuration,
    },
    git::{git::LocalRepository, url::parse_url},
};
use color_eyre::{
    eyre::{eyre, ContextCompat},
    Result,
};
use gr::vcs::common::{init_vcs, CreatePullRequest};
use open::that as open_in_browser;

pub async fn create(command: Commands, conf: Configuration) -> Result<()> {
    if let Commands::Pr(PrCommands::Create {
        message,
        description,
        branch,
        target,
        close,
        open,
    }) = command
    {
        let repo = LocalRepository::init()?;
        let branch = if let Some(branch) = branch {
            branch
        } else {
            repo.get_branch()?
        };
        let (remote_url, remote_branch) = repo.get_remote_data(&branch)?;
        let (hostname, repo) = parse_url(&remote_url)?;

        let vcs_type = conf.find_type(&hostname);
        let auth = conf.find_auth(&hostname, &repo).wrap_err(eyre!(
            "Authentication not found for {} {}.",
            &hostname,
            &repo
        ))?;

        let vcs = init_vcs(hostname, repo, auth, vcs_type);

        let pr = vcs
            .create_pr(CreatePullRequest {
                title: message,
                description: description.unwrap_or_default(),
                source: remote_branch,
                target: target.unwrap_or("master".to_string()),
                close_source_branch: close,
            })
            .await?;
        if open {
            open_in_browser(pr.url)?;
        } else {
            println!("{:#?}", pr);
        }
        Ok(())
    } else {
        Err(eyre!("Invalid command!"))
    }
}
