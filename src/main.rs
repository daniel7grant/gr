mod cmd;
mod git;

use cmd::{
    args::{Cli, Commands, PrCommands},
    config::Configuration,
};
use color_eyre::eyre::{eyre, ContextCompat};
use color_eyre::Result;
use git::{git::LocalRepository, url::parse_url};
use gr::vcs::common::{init_vcs, CreatePullRequest};
use open::that as open_in_browser;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Cli::parse_args();
    let conf = Configuration::new()?;

    let repo = LocalRepository::init()?;
    let branch = repo.get_branch()?;
    let (remote_url, remote_branch) = repo.get_remote_data(&branch)?;
    let (hostname, repo) = parse_url(&remote_url)?;

    let vcs_type = conf.find_type(&hostname);
    let auth = conf.find_auth(&hostname, &repo).wrap_err(eyre!(
        "Authentication not found for {} {}.",
        &hostname,
        &repo
    ))?;

    let vcs = init_vcs(hostname, repo, auth, vcs_type);

    match args.command {
        Commands::Pr(PrCommands::Get { branch, open }) => {
            let branch = branch.unwrap_or(remote_branch);
            let pr = vcs.get_pr(&branch).await?;
            if open {
                open_in_browser(pr.url)?;
            } else {
                println!("{:#?}", pr);
            }
            Ok(())
        }
        Commands::Pr(PrCommands::Create {
            message,
            description,
            branch,
            target,
            close,
            open,
        }) => {
            let source = branch.unwrap_or(remote_branch);
            let pr = vcs
                .create_pr(CreatePullRequest {
                    title: message,
                    description: description.unwrap_or_default(),
                    source,
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
        }
        _ => Err(eyre!("Invalid command.")),
    }
}
