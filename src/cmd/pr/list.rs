use crate::cmd::{
    args::{Cli, Commands, OutputType, PrCommands, StateFilter, UserFilter},
    config::Configuration,
};
use eyre::{eyre, ContextCompat, Result};
use gr_bin::{
    formatters::formatter::Formatter,
    vcs::common::{
        init_vcs, PullRequestStateFilter, PullRequestUserFilter, VersionControlSettings,
    },
};
use gr_bin::{git::git::LocalRepository, vcs::common::ListPullRequestFilters};
use tracing::instrument;

#[instrument(skip_all, fields(command = ?args.command))]
pub fn list(args: Cli, conf: Configuration) -> Result<()> {
    let Cli {
        command,
        dir,
        auth,
        output,
        ..
    } = args;
    if let Commands::Pr(PrCommands::List { author, state }) = command {
        let repository = LocalRepository::init(dir)?;
        // Find remote from branch upstream, or fallback to origin or any remote
        let (hostname, repo, ..) = repository.get_parsed_remote(None)?;

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

        let vcs = init_vcs(hostname, repo, settings)?;

        let prs = vcs.list_prs(ListPullRequestFilters {
            state: match state {
                Some(StateFilter::Open) | None => PullRequestStateFilter::Open,
                Some(StateFilter::Closed) => PullRequestStateFilter::Closed,
                Some(StateFilter::Merged) => PullRequestStateFilter::Merged,
                Some(StateFilter::Locked) => PullRequestStateFilter::Locked,
                Some(StateFilter::All) => PullRequestStateFilter::All,
            },
            author: match author {
                Some(UserFilter::Me) => PullRequestUserFilter::Me,
                Some(UserFilter::All) | None => PullRequestUserFilter::All,
            },
        })?;

        for pr in prs {
            match output {
                OutputType::Json => print!("{}", pr.show_json()),
                _ => print!("{}", pr.show_short()),
            }
        }

        Ok(())
    } else {
        Err(eyre!("Invalid command!"))
    }
}
