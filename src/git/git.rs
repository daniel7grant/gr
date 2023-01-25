use color_eyre::{
    eyre::{eyre, Context, ContextCompat},
    Result,
};
use git2::{BranchType, Repository, RepositoryOpenFlags};
use std::{env, path::PathBuf};

pub struct LocalRepository {
    repository: Repository,
}

impl LocalRepository {
    pub fn init(path: Option<String>) -> Result<LocalRepository> {
        let path = if let Some(path) = path {
            PathBuf::from(path)
        } else {
            env::current_dir()?
        };
        let repository =
            Repository::open_ext(path, RepositoryOpenFlags::empty(), vec![] as Vec<String>)
                .wrap_err("There is no git repository in the current directory.")?;

        Ok(LocalRepository { repository })
    }

    pub fn get_branch(self: &LocalRepository) -> Result<String> {
        let head = self
            .repository
            .head()
            .wrap_err("We are not on a branch currently.")?;
        let branch_shorthand = head
            .shorthand()
            .wrap_err("Branch name is not valid UTF-8.")?;
        Ok(branch_shorthand.to_string())
    }

    pub fn get_remote_branch(
        self: &LocalRepository,
        branch_name: Option<String>,
    ) -> Result<(String, String)> {
        let branch_name = if let Some(branch_name) = branch_name {
            branch_name
        } else {
            self.get_branch()?
        };

        let branch = self
            .repository
            .find_branch(&branch_name, BranchType::Local)
            .wrap_err(eyre!("Branch {} not found.", &branch_name))?;
        let upstream_branch = branch.upstream().wrap_err(eyre!(
            "Branch {} doesn't have an upstream branch.",
            &branch_name
        ))?;
        let upstream_branch_name = upstream_branch
            .name()?
            .wrap_err("Remote branch name is not valid UTF-8.")?;
        let (remote_name, remote_branch) = upstream_branch_name.split_once('/').wrap_err(eyre!(
            "Remote branch name {} is invalid.",
            upstream_branch_name
        ))?;
        let remote = self
            .repository
            .find_remote(remote_name)
            .wrap_err(eyre!("Remote URL with name {} not found.", remote_name))?;
        let remote_url = remote.url().wrap_err("Remote URL is not valid UTF-8.")?;
        Ok((remote_url.to_string(), remote_branch.to_string()))
    }
}
