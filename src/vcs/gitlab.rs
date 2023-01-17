use super::common::{CreatePullRequest, PullRequest, PullRequestState, User, VersionControl};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use color_eyre::{eyre::eyre, Result};
use reqwest::{Client, Method};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct GitLabUser {
    pub id: u32,
    pub username: String,
    pub name: String,
}

impl Into<User> for GitLabUser {
    fn into(self) -> User {
        let Self { username, .. } = self;
        User { username }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum GitLabPullRequestState {
    #[serde(rename = "opened")]
    OPEN,
    #[serde(rename = "closed")]
    DECLINED,
    #[serde(rename = "merged")]
    MERGED,
    #[serde(rename = "locked")]
    LOCKED,
}

impl Into<PullRequestState> for GitLabPullRequestState {
    fn into(self) -> PullRequestState {
        match self {
            GitLabPullRequestState::OPEN => PullRequestState::OPEN,
            GitLabPullRequestState::DECLINED => PullRequestState::DECLINED,
            GitLabPullRequestState::MERGED => PullRequestState::MERGED,
            GitLabPullRequestState::LOCKED => PullRequestState::LOCKED,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GitLabPullRequest {
    pub id: u32,
    pub iid: u32,
    pub state: GitLabPullRequestState,
    pub title: String,
    pub description: String,
    pub source_branch: String,
    pub target_branch: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub author: GitLabUser,
    pub closed_by: Option<GitLabUser>,
    pub reviewers: Option<Vec<GitLabUser>>,
}

impl Into<PullRequest> for GitLabPullRequest {
    fn into(self) -> PullRequest {
        let Self {
            id,
            iid,
            state,
            title,
            description,
            source_branch,
            target_branch,
            created_at,
            updated_at,
            author,
            closed_by,
            reviewers,
        } = self;
        PullRequest {
            id: iid,
            state: state.into(),
            title,
            description,
            source: source_branch,
            destination: target_branch,
            created_on: created_at,
            updated_on: updated_at,
            author: author.into(),
            closed_by: closed_by.map(|c| c.into()),
            reviewers: reviewers.map(|rs| rs.into_iter().map(|r| r.into()).collect()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GitLabCreatePullRequest {
    pub title: String,
    pub description: String,
    pub source_branch: String,
    pub target_branch: String,
    pub remove_source_branch: bool,
}

impl From<CreatePullRequest> for GitLabCreatePullRequest {
    fn from(pr: CreatePullRequest) -> Self {
        let CreatePullRequest {
            title,
            description,
            source,
            destination,
            close_source_branch,
        } = pr;
        Self {
            title,
            description,
            source_branch: source,
            target_branch: destination,
            remove_source_branch: close_source_branch,
        }
    }
}

pub struct GitLab {
    auth: (String, String),
    client: Client,
    project: String,
    repo: String,
}

impl GitLab {
    async fn call<T: DeserializeOwned, U: Serialize>(
        &self,
        method: Method,
        url: &str,
        body: Option<U>,
    ) -> Result<T> {
        let mut request = self
            .client
            .request(
                method,
                format!(
                    "https://gitlab.danielgrants.com/api/v4/projects/{}%2F{}{}",
                    self.project, self.repo, url
                ),
            )
            .header("Authorization", format!("Bearer {}", &self.auth.1))
            .header("Content-Type", "application/json");
        if let Some(body) = body {
            request = request.json(&body);
        }
        let result = request.send().await?;

        let t: T = result.json().await?;
        Ok(t)
    }
}

#[async_trait]
impl VersionControl for GitLab {
    fn init(auth: (String, String), (project, repo): (String, String)) -> Self {
        let client = Client::new();
        GitLab {
            auth,
            client,
            project,
            repo,
        }
    }
    async fn create_pr(&self, pr: CreatePullRequest) -> Result<PullRequest> {
        let new_pr: GitLabPullRequest = self
            .call(
                Method::POST,
                "/merge_requests",
                Some(GitLabCreatePullRequest::from(pr)),
            )
            .await?;

        Ok(new_pr.into())
    }
    async fn get_pr(&self, branch: &str) -> Result<PullRequest> {
        let prs: Vec<GitLabPullRequest> = self
            .call(
                Method::GET,
                &format!("/merge_requests?state=opened&source_branch={}", branch),
                None as Option<i32>,
            )
            .await?;

        match prs.into_iter().next() {
            Some(pr) => Ok(pr.into()),
            None => Err(eyre!("Pull request on branch {branch} not found.")),
        }
    }
}
