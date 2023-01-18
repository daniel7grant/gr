mod cmd;
mod git;

use cmd::{
    args::{Cli, Commands, PrCommands},
    config::Configuration,
};
use color_eyre::eyre::{eyre, ContextCompat};
use color_eyre::Result;
use git::{
    git::{get_branch, get_remote_data, get_repository},
    url::parse_url,
};
use gr::vcs::common::{init_vcs, CreatePullRequest};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Cli::parse_args();
    println!("{:?}", args);

    let conf = Configuration::new()?;

    let repo = get_repository()?;
    let branch = get_branch(&repo)?;
    let (remote_url, remote_branch) = get_remote_data(&repo, &branch)?;
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
            println!("{:#?}", pr);
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
            println!("{:#?}", pr);
            Ok(())
        }
        _ => Err(eyre!("Invalid command.")),
    }
}
