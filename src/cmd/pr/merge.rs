use crate::cmd::{
    args::{Cli, Commands, OutputType, PrCommands},
    config::Configuration,
};
use eyre::{
    eyre, ContextCompat,
    Result,
};
use colored::Colorize;
use gr_bin::vcs::common::init_vcs;
use gr_bin::{
    git::{git::LocalRepository, url::parse_url},
    vcs::common::VersionControlSettings,
};
use tracing::{info, instrument};

#[instrument(skip_all, fields(command = ?args.command))]
pub fn merge(args: Cli, conf: Configuration) -> Result<()> {
    let Cli {
        command,
        branch,
        dir,
        auth,
        output,
        verbose: _,
    } = args;
    if let Commands::Pr(PrCommands::Merge { delete }) = command {
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

        // Merge the PR
        let vcs = init_vcs(hostname, repo, settings);
        let pr = vcs.get_pr_by_branch(&remote_branch)?;
        let pr = vcs.merge_pr(pr.id, delete)?;

        pr.print(false, output.into());

        // Checkout to the target branch
        let target_branch = pr.target;
        let message = format!("Checking out to {} and pulling after merge.", target_branch.blue());
        match output {
            OutputType::Json => info!("{}", message),
            _ => println!("{}", message),
        };
        repository.checkout_remote_branch(target_branch, output != OutputType::Json)?;

        // Delete local branch if delete was passed
        if delete {
            let source_branch = pr.source;
            repository.delete_branch(source_branch.clone())?;

            let message = format!("Deleted branch {}.", source_branch.blue());
            match output {
                OutputType::Json => info!("{}", message),
                _ => println!("{}", message),
            };
        }

        Ok(())
    } else {
        Err(eyre!("Invalid command!"))
    }
}
