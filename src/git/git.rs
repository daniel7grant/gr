use color_eyre::{
    eyre::{eyre, Context, ContextCompat},
    Result,
};
use git2::{BranchType, Repository, RepositoryOpenFlags};
use std::env;

pub fn get_repository() -> Result<Repository> {
    let cwd = env::current_dir()?;
    Repository::open_ext(cwd, RepositoryOpenFlags::empty(), vec![] as Vec<String>)
        .wrap_err("There is no git repository in the current directory.")
}

pub fn get_branch(repo: &Repository) -> Result<String> {
    let head = repo.head().wrap_err("We are not on a branch currently.")?;
    let branch_shorthand = head
        .shorthand()
        .wrap_err("Branch name is not valid UTF-8.")?;
    Ok(branch_shorthand.to_string())
}
pub fn get_remote_data(repo: &Repository, branch_name: &str) -> Result<(String, String)> {
    let branch = repo
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
    let remote = repo
        .find_remote(remote_name)
        .wrap_err(eyre!("Remote URL with name {} not found.", remote_name))?;
    let remote_url = remote.url().wrap_err("Remote URL is not valid UTF-8.")?;
    Ok((remote_url.to_string(), remote_branch.to_string()))
}
