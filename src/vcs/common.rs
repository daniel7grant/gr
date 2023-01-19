use async_trait::async_trait;
use chrono::{DateTime, Utc};
use color_eyre::Result;
use serde::{Deserialize, Serialize};

use super::{bitbucket::Bitbucket, github::GitHub, gitlab::GitLab};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct User {
    pub username: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum PullRequestState {
    Open,
    Closed,
    Merged,
    Locked,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PullRequest {
    pub id: u32,
    pub state: PullRequestState,
    pub title: String,
    pub description: String,
    pub source: String,
    pub target: String,
    pub url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub author: User,
    pub closed_by: Option<User>,
    pub reviewers: Option<Vec<User>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreatePullRequest {
    pub title: String,
    pub description: String,
    pub source: String,
    pub target: String,
    pub close_source_branch: bool,
}

#[async_trait]
pub trait VersionControl {
    fn init(hostname: String, repo: String, auth: String) -> Self
    where
        Self: Sized;
    async fn create_pr(&self, pr: CreatePullRequest) -> Result<PullRequest>;
    async fn get_pr(&self, branch: &str) -> Result<PullRequest>;
}

pub fn init_vcs(
    hostname: String,
    repo: String,
    auth: String,
    ty: Option<String>,
) -> Box<dyn VersionControl> {
    match (hostname.as_str(), ty) {
        ("github.com", _) => Box::new(GitHub::init(hostname, repo, auth)),
        ("bitbucket.org", _) => Box::new(Bitbucket::init(hostname, repo, auth)),
        (_, _) => Box::new(GitLab::init(hostname, repo, auth)),
    }
}
