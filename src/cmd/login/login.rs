use crate::cmd::{
    args::{Cli, Commands},
    config::{Configuration, RepositoryConfig},
};
use color_eyre::{
    eyre::{eyre, Context},
    Result,
};
use gr_bin::{
    git::{git::LocalRepository, url::parse_url},
    vcs::common::{init_vcs, VersionControlSettings},
};
use inquire::Text;
use std::{thread::sleep, time::Duration};
use tracing::instrument;

#[instrument(skip_all, fields(command = ?args.command))]
pub fn login(args: Cli, mut conf: Configuration) -> Result<()> {
    let Cli { command, dir, .. } = args;
    if let Commands::Login {
        hostname,
        repo: repo_name,
        token,
    } = command
    {
        // Get hostname and repo and initialize VCS
        let settings = VersionControlSettings::default();
        let (hostname, repo) = if let Some(hostname) = hostname {
            (hostname, repo_name.clone().unwrap_or_default())
        } else {
            let repo = LocalRepository::init(dir)?;
            let (remote_url, _) = repo.get_remote_branch(None)?;
            let (hostname, repo) = parse_url(&remote_url)?;

            (hostname, repo)
        };
        let vcs = init_vcs(hostname.clone(), repo, settings);

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
            if !url.contains("scopes=") {
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
            loop {
                token = Text::new("Paste the token here: ")
                    .prompt()
                    .wrap_err("Reading the token failed.")?;
                match vcs.validate_token(&token) {
                    Ok(_) => break,
                    Err(err) => println!("{}", err.to_string()),
                }
            }
            token
        };

        // Modify the token in the configuration
        let host_conf = conf.vcs.entry(hostname);
        match repo_name {
            Some(repo) => host_conf.and_modify(|h| {
                h.repositories
                    .entry(repo)
                    .and_modify(|r| {
                        r.auth = Some(token.clone());
                    })
                    .or_insert(RepositoryConfig {
                        auth: Some(token),
                        default_branch: None,
                    });
            }),
            None => host_conf.and_modify(|h| {
                h.auth = token;
            }),
        };
        conf.save()?;
        println!("Authentication token saved.");

        Ok(())
    } else {
        Err(eyre!("Invalid command!"))
    }
}
