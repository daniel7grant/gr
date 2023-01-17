use async_trait::async_trait;
use chrono::{Utc, DateTime};
use color_eyre::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct User {
    pub username: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub enum PullRequestState {
    #[default]
    OPEN,
    DECLINED,
    MERGED,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PullRequest {
    pub id: u32,
    pub state: PullRequestState,
    pub title: String,
    pub description: String,
    pub source: String,
    pub destination: String,
    pub created_on: DateTime<Utc>,
    pub updated_on: DateTime<Utc>,
    pub author: User,
    pub closed_by: Option<User>,
    pub reviewers: Option<Vec<User>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreatePullRequest {
    pub title: String,
    pub description: String,
    pub source: String,
    pub destination: String,
    pub close_source_branch: bool,
}

#[async_trait]
pub trait VersionControl {
    fn init(auth: (String, String), repo: (String, String)) -> Self;
    async fn create_pr(self, pr: CreatePullRequest) -> Result<PullRequest>;
    async fn get_pr(self, branch: &str) -> Result<PullRequest>;
}
