use std::io::{stdin, BufRead, Error};

use crate::cmd::{
    args::{Cli, Commands, OutputType, PrCommands},
    config::{Configuration, RepositoryConfig},
    pr::merge::merge,
};
use eyre::{eyre, ContextCompat, Result};
use gr_bin::vcs::common::{init_vcs, CreatePullRequest};
use gr_bin::{git::git::LocalRepository, vcs::common::VersionControlSettings};
use tracing::{debug, info, instrument, trace};

#[instrument(skip_all, fields(command = ?args.command))]
pub fn create(mut args: Cli, mut conf: Configuration) -> Result<()> {
    let Cli {
        command,
        branch,
        dir,
        auth,
        output,
        ..
    } = args.clone();
    if let Commands::Pr(PrCommands::Create {
        message,
        description,
        target,
        delete,
        open,
        reviewers,
        should_merge,
        force_merge,
    }) = command
    {
        let repository = LocalRepository::init(dir)?;
        let (hostname, repo, remote_branch) = repository.get_parsed_remote(branch.clone())?;

        // If branch has no remote, we can push it first
        let remote_branch = remote_branch
            .wrap_err("You have to push this branch first before you can create a PR.")
            .or_else(|_| {
                let remotes = repository.get_remotes()?;
                let remote = remotes.first().wrap_err(
                    "There are no remotes, push to a repository before you can create a PR.",
                )?;

                let branch = if let Some(branch) = &branch {
                    branch.to_string()
                } else {
                    repository.get_branch()?
                };
                let message = format!("Branch {branch} is not pushed, pushing to {remote}.");
                match output {
                    OutputType::Json => info!("{}", message),
                    _ => println!("{}", message),
                };
                repository.push(remote, &branch)?;

                Ok(branch) as Result<String>
            })?;

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

        let default_branch = settings.default_branch.clone();

        let vcs = init_vcs(hostname.clone(), repo.clone(), settings)?;

        // Read the description from the STDIN or fallback to
        let description = description
            .or_else(|| {
                if atty::is(atty::Stream::Stdin) {
                    debug!("Stdin is closed, nothing to read from here.");
                    return None;
                };

                let input = stdin().lock();
                let str: Result<String, Error> = input.lines().collect();
                debug!("Received data from stdin: {:?}", str);
                str.ok()
            })
            .or_else(|| {
                let commits = repository
                    .get_branch_commits_from_target(
                        branch.clone(),
                        target
                            .clone()
                            .or(default_branch)
                            // TODO: fallback to query from the VCS
                            .unwrap_or("master".to_string()),
                    )
                    .map(|commits| {
                        commits
                            .into_iter()
                            .map(|s| format!("- {}\n", s))
                            .collect::<String>()
                    });

                match commits {
                    Ok(commits) => {
                        trace!("Description is {}.", commits);
                        Some(commits)
                    }
                    Err(err) => {
                        info!(
                            "Commit generation failed: {}, description will be empty.",
                            err
                        );
                        None
                    }
                }
            })
            .unwrap_or_default();
        let is_default_branch = target.is_none();

        let pr = vcs.create_pr(CreatePullRequest {
            title: message,
            description,
            source: remote_branch,
            target,
            close_source_branch: delete,
            reviewers: reviewers.unwrap_or_default(),
        })?;

        pr.print(open, output.into());

        // Merge the PR instantly if merge is passed
        if should_merge {
            info!("Merging pull request {} instantly.", pr.id);
            args.command = Commands::Pr(PrCommands::Merge {
                delete,
                force: force_merge,
            });
            merge(args, conf.clone())?;
        }

        // Save default branch to config for caching
        if is_default_branch {
            conf.vcs.entry(hostname).and_modify(|host| {
                host.repositories
                    .entry(repo)
                    .and_modify(|repo| repo.default_branch = Some(pr.target.clone()))
                    .or_insert(RepositoryConfig {
                        default_branch: Some(pr.target.clone()),
                        ..Default::default()
                    });
            });

            conf.save()?;
        }

        Ok(())
    } else {
        Err(eyre!("Invalid command!"))
    }
}
