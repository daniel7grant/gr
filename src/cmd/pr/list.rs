use crate::cmd::{
    args::{Cli, Commands, PrCommands, StateFilter, UserFilter},
    config::Configuration,
};
use color_eyre::{
    eyre::{eyre, ContextCompat},
    Result,
};
use colored::Colorize;
use gr::vcs::common::{
    init_vcs, PullRequestStateFilter, PullRequestUserFilter, VersionControlSettings,
};
use gr::{
    git::{git::LocalRepository, url::parse_url},
    vcs::common::{ListPullRequestFilters, PullRequestState},
};
use tracing::instrument;

#[instrument(skip(conf))]
pub async fn list(args: Cli, conf: Configuration) -> Result<()> {
    let Cli {
        command, dir, auth, ..
    } = args;
    if let Commands::Pr(PrCommands::List { author, state }) = command {
        let repo = LocalRepository::init(dir)?;
        // Find remote from branch upstream, or fallback to origin or any remote
        let remote_url = repo
            .get_remote_branch(None)
            .map(|(url, _)| url)
            .or_else(|_| repo.get_remote(None))?;
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

        let prs = vcs
            .list_prs(ListPullRequestFilters {
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
            })
            .await?;

        for pr in prs {
            let max_width_title = if pr.title.len() > 73 {
                format!("{}...", &pr.title[0..70])
            } else {
                pr.title.clone()
            };
            let colored_title = match pr.state {
                PullRequestState::Open => max_width_title.bold(),
                PullRequestState::Closed => max_width_title.bold().red(),
                PullRequestState::Merged => max_width_title.bold().green(),
                PullRequestState::Locked => max_width_title.bold().magenta(),
            };
            let colored_id = format!("#{}", pr.id).dimmed();
            println!("{:<73} {:>6}", colored_title, colored_id);
        }

        Ok(())
    } else {
        Err(eyre!("Invalid command!"))
    }
}
