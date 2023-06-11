use std::io::{self, Write};

use crate::cmd::{
    args::{Cli, Commands, RepoCommands},
    config::Configuration,
};
use colored::Colorize;
use eyre::{eyre, Context, ContextCompat, Result};
use gr_bin::vcs::common::init_vcs;
use gr_bin::{git::git::LocalRepository, vcs::common::VersionControlSettings};
use tracing::instrument;

#[instrument(skip_all, fields(command = ?args.command))]
pub fn delete(args: Cli, conf: Configuration) -> Result<()> {
    let Cli {
        command,
        branch,
        dir,
        auth,
        output,
        ..
    } = args;
    if let Commands::Repo(RepoCommands::Delete { force }) = command {
        let repository = LocalRepository::init(dir)?;
        let (hostname, repo, ..) = repository.get_parsed_remote(branch)?;

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

        let repo = vcs.get_repository()?;
        repo.print(false, output.into());

        if !force {
            println!(
                "You are about to {} this repository. {}",
                "delete".red().bold(),
                "This action cannot be undone!".bold()
            );
            let mut stdout = io::stdout();
            let stdin = io::stdin();
            write!(
                stdout,
                "Please enter the repository name if you are absolutely sure: "
            )?;
            stdout.flush()?;
            let mut entered_name = String::new();
            stdin
                .read_line(&mut entered_name)
                .wrap_err("Reading the repository name failed.")?;
            entered_name = entered_name.trim().to_string();
            if entered_name != repo.full_name {
                return Err(eyre!(
                    "You cannot delete {}! You entered: {}.",
                    repo.full_name,
                    entered_name
                ));
            }
        }

        vcs.delete_repository()?;

        println!("Repository {} deleted.", repo.full_name);

        Ok(())
    } else {
        Err(eyre!("Invalid command!"))
    }
}
