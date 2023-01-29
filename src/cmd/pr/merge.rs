use crate::cmd::{
    args::{Commands, PrCommands},
    config::Configuration,
};
use color_eyre::{
    eyre::{eyre, ContextCompat},
    Result,
};
use gr::git::{git::LocalRepository, url::parse_url};
use gr::vcs::common::init_vcs;

pub async fn merge(command: Commands, conf: Configuration) -> Result<()> {
    if let Commands::Pr(PrCommands::Merge {
        branch,
        dir,
        delete,
    }) = command
    {
        let repo = LocalRepository::init(dir)?;
        let (remote_url, remote_branch) = repo.get_remote_branch(branch)?;
        let (hostname, repo) = parse_url(&remote_url)?;

        let settings = conf.find_settings(&hostname, &repo).wrap_err(eyre!(
            "Authentication not found for {} in {}.",
            &hostname,
            &repo
        ))?;

        let vcs = init_vcs(hostname, repo, settings);
        let pr = vcs.get_pr_by_branch(&remote_branch).await?;
        vcs.merge_pr(pr.id, delete).await?;
        pr.show(false);
        Ok(())
    } else {
        Err(eyre!("Invalid command!"))
    }
}
