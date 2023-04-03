use crate::cmd::{
    args::{Cli, Commands},
    config::{Configuration, RepositoryConfig, VcsConfig},
};
use eyre::{eyre, Context, Result};
use gr_bin::{
    git::git::LocalRepository,
    vcs::common::{init_vcs, VersionControlSettings},
};
use std::{collections::HashMap, io, io::Write, thread::sleep, time::Duration};
use tracing::instrument;

#[instrument(skip_all, fields(command = ?args.command))]
pub fn login(args: Cli, mut conf: Configuration) -> Result<()> {
    let Cli { command, dir, .. } = args;
    if let Commands::Login {
        hostname,
        vcs_type,
        repo: repo_name,
        token,
    } = command
    {
        // Get hostname and repo and initialize VCS
        let settings = VersionControlSettings {
            vcs_type: vcs_type.clone(),
            ..VersionControlSettings::default()
        };
        let (hostname, repo) = if let Some(hostname) = hostname {
            (hostname, repo_name.clone().unwrap_or_default())
        } else {
            let repo = LocalRepository::init(dir)?;
            let (hostname, repo, _) = repo.get_parsed_remote(None)?;

            (hostname, repo)
        };
        let vcs = init_vcs(hostname.clone(), repo, settings)?;

        // If the token arg is passed, validate and use that
        let token = if let Some(token) = token {
            vcs.validate_token(&token).and(Ok(token))?
        } else {
            // Otherwise open URL to have the user save the token
            let url = vcs.login_url();
            println!(
                "To login to {}, create a token and copy the token value.",
                hostname
            );
            // TODO: this is a dirty hack
            if hostname == "bitbucket.org" {
                println!("The token needs account, workspace and project read, pull request read and write permissions.");
                println!("You have to enter you username and the token separated with a colon (e.g. user:ATBB...).");
            }
            sleep(Duration::from_millis(500));
            let can_open = open::that(&url);
            if can_open.is_err() {
                println!("Open this page: {}", &url);
            }
            sleep(Duration::from_millis(500));

            // Read the token from the user
            let mut token;
            let mut stdout = io::stdout();
            let stdin = io::stdin();
            loop {
                write!(stdout, "Paste the token here: ")?;
                stdout.flush()?;
                token = String::new();
                stdin
                    .read_line(&mut token)
                    .wrap_err("Reading the token failed.")?;
                token = token.trim().to_string();
                match vcs.validate_token(&token) {
                    Ok(_) => break,
                    Err(err) => println!("{}", err),
                }
            }
            token
        };

        // Modify the token in the configuration
        let host_conf = conf.vcs.entry(hostname);
        match repo_name {
            Some(repo) => {
                host_conf.and_modify(|h| {
                    h.repositories
                        .entry(repo)
                        .and_modify(|r| {
                            r.auth = Some(token.clone());
                        })
                        .or_insert(RepositoryConfig {
                            auth: Some(token),
                            default_branch: None,
                        });
                });
            }
            None => {
                host_conf
                    .and_modify(|h| {
                        h.auth = token.clone();
                    })
                    .or_insert(VcsConfig {
                        auth: token,
                        repositories: HashMap::default(),
                        vcs_type,
                    });
            }
        };
        conf.save()?;
        println!("Authentication token saved.");

        Ok(())
    } else {
        Err(eyre!("Invalid command!"))
    }
}
