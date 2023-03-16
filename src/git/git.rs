use eyre::{eyre, Context, ContextCompat, Result};
use git2::{BranchType, ObjectType, Oid, Repository, RepositoryOpenFlags, Sort};
use std::{
    collections::HashSet,
    env,
    path::PathBuf,
    process::{Command, Stdio},
};
use tracing::{debug, info, instrument};

pub struct LocalRepository {
    repository: Repository,
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
        let repository =
            Repository::open_ext(&path, RepositoryOpenFlags::empty(), vec![] as Vec<String>)
                .wrap_err("There is no git repository in the current directory.")?;

        let path = path.into_os_string().into_string().unwrap();
        Ok(LocalRepository { repository, path })
    }

    #[instrument(skip_all)]
    pub fn get_branch(self: &LocalRepository) -> Result<String> {
        let head = self
            .repository
            .head()
            .wrap_err("We are not on a branch currently.")?;
        let branch_shorthand = head
            .shorthand()
            .wrap_err("Branch name is not valid UTF-8.")?;

        info!("Current branch is {branch_shorthand}.");

        Ok(branch_shorthand.to_string())
    }

    #[instrument(skip(self))]
    pub fn get_remote(self: &LocalRepository, remote_name: Option<String>) -> Result<String> {
        // Use given branch name if we can
        let remote_name = remote_name
            // Or fallback to origin, if exists
            .or_else(|| {
                self.repository
                    .find_remote("origin")
                    .ok()
                    .and(Some("origin".to_string()))
            })
            // Or fallback to the first remote
            .or_else(|| {
                self.repository
                    .remotes()
                    .into_iter()
                    .next()
                    .and_then(|r| r.get(0).map(|b| b.to_string()))
            })
            .wrap_err("There are no remotes in the current repository.")?;

        // Find remote URL
        let remote = self
            .repository
            .find_remote(&remote_name)
            .wrap_err(eyre!("Remote URL with name {} not found.", remote_name))?;
        let remote_url = remote.url().wrap_err("Remote URL is not valid UTF-8.")?;

        info!("Using remote {remote_name} with url {remote_url}.");

        Ok(remote_url.to_string())
    }

    #[instrument(skip(self))]
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

        let remote_url = self.get_remote(Some(remote_name.to_string()))?;

        debug!("Using remote {remote_name} with url {remote_url}.");

        Ok((remote_url.to_string(), remote_branch.to_string()))
    }

    #[instrument(skip(self))]
    pub fn pull(self: &LocalRepository, output: bool) -> Result<()> {
        debug!("Git pulling in {}.", &self.path);
        let mut child = Command::new("git")
            .args(["-C", &self.path, "pull"])
            .stdout(if output {
                Stdio::inherit()
            } else {
                Stdio::null()
            })
            .stderr(if output {
                Stdio::inherit()
            } else {
                Stdio::null()
            })
            .spawn()
            .wrap_err("Git pull failed to start.")?;

        child.wait().wrap_err("Git pull failed to end.")?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub fn delete_branch(self: &LocalRepository, branch_name: String) -> Result<()> {
        let mut branch = self
            .repository
            .find_branch(&branch_name, BranchType::Local)
            .wrap_err(eyre!("Branch {} not found.", &branch_name))?;

        branch
            .delete()
            .wrap_err(eyre!("Cannot delete local branch {}.", &branch_name))?;

        Ok(())
    }

    #[instrument(skip(self))]
    pub fn get_branch_commits_from_target(
        self: &LocalRepository,
        branch_name: Option<String>,
        target_name: String,
    ) -> Result<Vec<String>> {
        // Find all commit OIDs on target
        let target = self
            .repository
            .find_branch(&target_name, BranchType::Local)
            .wrap_err(eyre!("Branch {} not found.", &target_name))?;
        let target_commit = target.get().peel_to_commit()?;

        let mut revwalk_target = self.repository.revwalk()?;
        revwalk_target.set_sorting(Sort::TOPOLOGICAL)?;

        revwalk_target.push(target_commit.id())?;

        // Collect the OIDs of the commits on target for quick retrieval
        let target_oids: HashSet<Oid> = revwalk_target.filter_map(|oid| oid.ok()).collect();

        // Find all commits on branch but not on target
        let mut revwalk_branch = self.repository.revwalk()?;
        revwalk_branch.set_sorting(Sort::TOPOLOGICAL)?;

        if let Some(branch_name) = branch_name {
            let branch = self
                .repository
                .find_branch(&branch_name, BranchType::Local)
                .wrap_err(eyre!("Branch {} not found.", &branch_name))?;
            let branch_commit = branch.get().peel_to_commit()?;

            revwalk_branch.push(branch_commit.id())?;
        } else {
            revwalk_branch.push_head()?;
        }

        // Get messages from the commits on branch not on target
        let messages: Vec<String> = revwalk_branch
            .filter_map(|oid| oid.ok())
            .filter(|oid| !target_oids.contains(oid))
            .filter_map(|oid| self.repository.find_commit(oid).ok())
            .filter_map(|commit| commit.summary().map(|s| s.to_string()))
            .collect();

        Ok(messages)
    }

    #[instrument(skip(self))]
    pub fn checkout_remote_branch(
        self: &LocalRepository,
        target_branch: String,
        output: bool,
    ) -> Result<()> {
        let branch = self
            .repository
            .find_branch(&target_branch, BranchType::Local)
            .wrap_err(eyre!("Branch {} not found.", &target_branch))?;

        let reference = branch.get();
        let object = reference.peel(ObjectType::Any).wrap_err(eyre!(
            "Cannot unwrap reference for branch {}.",
            &target_branch
        ))?;

        debug!("Checking out the tree on branch {}", &target_branch);
        self.repository
            .checkout_tree(&object, None)
            .wrap_err(eyre!("Cannot checkout to branch {}.", &target_branch))?;

        let refname = reference
            .name()
            .wrap_err("Remote branch name is not valid UTF-8.")?;
        debug!("Setting the head on branch {}", &target_branch);
        self.repository
            .set_head(refname)
            .wrap_err(eyre!("Cannot checkout to branch {}.", &target_branch))?;

        self.pull(output)?;

        Ok(())
    }
}
