use async_trait::async_trait;
use color_eyre::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct User {
    nickname: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub enum PullRequestState {
    #[default]
    OPEN,
    DECLINED,
    MERGED,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PullRequestBranch {
    name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PullRequestRevision {
    branch: PullRequestBranch,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PullRequest {
    id: u32,
    state: PullRequestState,
    title: String,
    description: String,
    source: PullRequestRevision,
    destination: PullRequestRevision,
    author: User,
    reviewers: Vec<User>,
}

#[derive(Default, Deserialize, Serialize)]
pub struct PullRequestPartial {
    id: u32,
    state: PullRequestState,
    title: String,
    description: String,
    source: String,
    target: String,
    author: User,
    reviewers: Vec<User>,
}

#[async_trait]
pub trait VersionControl {
    fn init(auth: (String, String), repo: (String, String)) -> Self;
    async fn create_pr(self) -> Result<PullRequest>;
    async fn get_pr(self, branch: &str) -> Result<PullRequest>;
}
