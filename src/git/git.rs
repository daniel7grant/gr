use crate::git::url::parse_url;
use eyre::{eyre, Context, ContextCompat, Result};
use std::{
    env,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use tracing::{debug, info, instrument};

pub struct LocalRepository {
    path: String,
}

impl LocalRepository {
    #[instrument]
    pub fn init(path: Option<String>) -> Result<LocalRepository> {
        let path = if let Some(path) = path {
            PathBuf::from(path)
        } else {
            env::current_dir()?
        };
        info!("Repository directory is {}.", path.to_string_lossy());

        let path = path.into_os_string().into_string().unwrap();
        Ok(LocalRepository { path })
    }

    #[instrument(skip_all)]
    fn run(&self, args: Vec<&str>, inherit: bool) -> Result<Vec<String>> {
        let command = Command::new("git")
            .current_dir(&self.path)
            .args(args)
            .stdout(if inherit {
                Stdio::inherit()
            } else {
                Stdio::piped()
            })
            .stderr(if inherit {
                Stdio::inherit()
            } else {
                Stdio::piped()
            })
            .output()?;

        if command.status.success() {
            let output = String::from_utf8_lossy(&command.stdout).to_string();
            Ok(output
                .lines()
                .map(|s| s.to_string())
                .collect::<Vec<String>>())
        } else {
            Err(eyre!(String::from_utf8_lossy(&command.stderr).to_string()))
        }
    }

    #[instrument(skip_all)]
    pub fn has_git(self: &LocalRepository) -> bool {
        self.run(vec!["rev-parse"], false).is_ok()
    }

    #[instrument(skip(self))]
    pub fn get_remotes(self: &LocalRepository) -> Result<Vec<String>> {
        self.run(vec!["remote"], false)
    }

    #[instrument(skip(self))]
    pub fn set_remote(self: &LocalRepository, name: String, url: String) -> Result<()> {
        let existing_remotes = self.get_remotes()?;
        if existing_remotes.iter().any(|s| s == &name) {
            self.run(vec!["remote", "set-url", &name, &url], false)?;
        } else {
            self.run(vec!["remote", "add", &name, &url], false)?;
        }

        Ok(())
    }

    #[instrument(skip_all)]
    pub fn get_branch(self: &LocalRepository) -> Result<String> {
        let head = self
            .run(vec!["rev-parse", "--abbrev-ref", "HEAD"], false)
            .wrap_err("Cannot get current branch.")?
            .into_iter()
            .next()
            .filter(|h| h != "HEAD")
            .wrap_err(eyre!("We are not on a branch currently."))?;

        info!("Current branch is {head}.");
        Ok(head)
    }

    #[instrument(skip(self))]
    pub fn get_parsed_remote(
        self: &LocalRepository,
        branch_name: Option<String>,
    ) -> Result<(String, String, String)> {
        let branch_name = if let Some(branch_name) = branch_name {
            branch_name
        } else {
            self.get_branch()?
        };

        // Get remote name
        let remote_name = self
            .run(
                vec!["config", &format!("branch.{branch_name}.remote")],
                false,
            )
            .wrap_err(eyre!(
                "Branch {} doesn't have an upstream branch.",
                &branch_name
            ))?
            .into_iter()
            .next()
            .wrap_err(eyre!(
                "Branch {} doesn't have an upstream branch.",
                &branch_name
            ))?;

        // Find remote URL
        let remote_url = self
            .run(vec!["remote", "get-url", &remote_name], false)
            .wrap_err(eyre!("Cannot get remote for {remote_name}."))?
            .into_iter()
            .next()
            .wrap_err(eyre!("Cannot get URL for {remote_name}."))?;

        info!("Using remote {remote_name} with url {remote_url}.");

        parse_url(&remote_url).map(|(host, repo)| (host, repo, branch_name))
    }

    #[instrument(skip(self))]
    pub fn delete_branch(self: &LocalRepository, branch_name: String) -> Result<()> {
        self.run(vec!["branch", "-d", &branch_name], false)?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub fn get_branch_commits_from_target(
        self: &LocalRepository,
        branch_name: Option<String>,
        target_name: String,
    ) -> Result<Vec<String>> {
        let branch_name = if let Some(branch_name) = branch_name {
            branch_name
        } else {
            self.get_branch()?
        };

        // Find all commit summaries between the two branches
        let messages = self
            .run(
                vec![
                    "log",
                    &format!("{target_name}..{branch_name}"),
                    "--pretty=%s",
                ],
                false,
            )
            .wrap_err(eyre!("Branch {} not found.", &target_name))?;

        Ok(messages)
    }

    #[instrument(skip(self))]
    pub fn checkout_remote_branch(
        self: &LocalRepository,
        target_branch: String,
        output: bool,
    ) -> Result<()> {
        self.run(vec!["checkout", &target_branch], false)?;

        debug!("Git pulling in {}.", &self.path);
        self.run(vec!["pull"], output)?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub fn push(self: &LocalRepository, branch: &str) -> Result<()> {
        self.run(vec!["push", "-u", "origin", &branch], true)
            .wrap_err(eyre!("Could not push {branch} to remote"))?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub fn clone(self: &LocalRepository, url: String, dir: Option<String>) -> Result<()> {
        if let Some(dir) = dir {
            if Path::new(&dir).exists() {
                // If the path exists, we have to clone inside of it
                self.run(vec!["clone", &url], true)
            } else {
                // Otherwise we have to reinitialize the repository to allow cloning into empty repo
                LocalRepository::init(None)?.run(vec!["clone", &url, &dir], true)
            }
        } else {
            self.run(vec!["clone", &url], true)
        }
        .wrap_err(eyre!("Could not clone {url}."))?;

        Ok(())
    }
}
