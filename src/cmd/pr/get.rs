use crate::cmd::{
    args::{Commands, PrCommands},
    config::Configuration,
};
use color_eyre::{
    eyre::{eyre, ContextCompat},
    Result,
};
use gr::{git::{git::LocalRepository, url::parse_url}, vcs::common::VersionControlSettings};
use gr::vcs::common::init_vcs;

pub async fn get(command: Commands, conf: Configuration) -> Result<()> {
    if let Commands::Pr(PrCommands::Get { branch, dir, open , auth}) = command {
        let repo = LocalRepository::init(dir)?;
        let (remote_url, remote_branch) = repo.get_remote_branch(branch)?;
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

        let vcs = init_vcs(hostname, repo, settings);

        let pr = vcs.get_pr_by_branch(&remote_branch).await?;
        pr.show(open);
        Ok(())
    } else {
        Err(eyre!("Invalid command!"))
    }
}
