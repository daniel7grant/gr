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

pub async fn list(command: Commands, conf: Configuration) -> Result<()> {
    if let Commands::Pr(PrCommands::List { dir }) = command {
        let repo = LocalRepository::init(dir)?;
        let (remote_url, _) = repo.get_remote_branch(None)?;
        let (hostname, repo) = parse_url(&remote_url)?;

        let settings = conf.find_settings(&hostname, &repo).wrap_err(eyre!(
            "Authentication not found for {} in {}.",
            &hostname,
            &repo
        ))?;

        let vcs = init_vcs(hostname, repo, settings);

        Err(eyre!("TODO: Call list for VCSs."))
    } else {
        Err(eyre!("Invalid command!"))
    }
}
