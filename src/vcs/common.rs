use async_trait::async_trait;
use color_eyre::Result;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct User {
    pub nickname: String,
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
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PullRequestRevision {
    pub branch: PullRequestBranch,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PullRequest {
    pub id: u32,
    pub state: PullRequestState,
    pub title: String,
    pub description: String,
    pub source: PullRequestRevision,
    pub destination: PullRequestRevision,
    pub author: User,
    pub reviewers: Option<Vec<User>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Paginated<T> {
    pub next: Option<String>,
    pub page: u32,
    pub pagelen: u32,
    pub size: u32,
    pub values: Vec<T>,
}

#[async_trait]
pub trait VersionControl {
    fn init(auth: (String, String), repo: (String, String)) -> Self;
    async fn create_pr(self) -> Result<PullRequest>;
    async fn get_pr(self, branch: &str) -> Result<PullRequest>;
}
