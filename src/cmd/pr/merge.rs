use crate::cmd::{
    args::{Cli, Commands, OutputType, PrCommands},
    config::Configuration,
};
use colored::Colorize;
use eyre::{eyre, ContextCompat, Result};
use gr_bin::vcs::common::init_vcs;
use gr_bin::{git::git::LocalRepository, vcs::common::VersionControlSettings};
use tracing::{info, instrument};

#[instrument(skip_all, fields(command = ?args.command))]
pub fn merge(args: Cli, conf: Configuration) -> Result<()> {
    let Cli {
        command,
        branch,
        dir,
        auth,
        output,
        ..
    } = args;
    if let Commands::Pr(PrCommands::Merge { delete, force }) = command {
        let repository = LocalRepository::init(dir)?;
        let (hostname, repo, remote_branch) = repository.get_parsed_remote(branch.clone())?;
        let remote_branch = remote_branch.wrap_err(eyre!(
            "You have to push this branch first, before you can merge it."
        ))?;

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

        // Check if there are local changes if we are on this branch
        if branch.is_none() && repository.has_modifications()? && !force {
            return Err(eyre!("You can't merge until there are local modifications. If you are sure, pass the --force argument."));
        }

        // Get the PR on the current branch
        let vcs = init_vcs(hostname, repo, settings)?;
        let pr = vcs.get_pr_by_branch(&remote_branch)?;

        // Check if there are unpushed changes
        let branch_sha = repository.get_branch_sha(branch.clone())?;
        if !branch_sha.starts_with(&pr.source_sha) && !force {
            return Err(eyre!("You can't merge until there are unpushed changes. If you are sure, pass the --force argument."));
        };

        // Merge the PR
        let pr = vcs.merge_pr(pr.id, delete)?;

        pr.print(false, output.into());

        // Checkout to the target branch if we are on this branch
        if branch.is_none() {
            let target_branch = pr.target;
            let message = format!(
                "Checking out to {} and pulling after merge.",
                target_branch.blue()
            );
            match output {
                OutputType::Json => info!("{}", message),
                _ => println!("{}", message),
            };
            repository.checkout_remote_branch(target_branch, output != OutputType::Json)?;
        }

        // Delete local branch if remote branch is deleted
        if pr.delete_source_branch || delete {
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
