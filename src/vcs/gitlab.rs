// Documentation: https://docs.gitlab.com/ee/api/api_resources.html
use super::common::{CreatePullRequest, PullRequest, PullRequestState, User, VersionControl, VersionControlSettings};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use color_eyre::{eyre::eyre, Result};
use reqwest::{Client, Method};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use urlencoding::encode;

#[derive(Debug, Deserialize, Serialize)]
pub struct GitLabUser {
    pub id: u32,
    pub username: String,
    pub name: String,
}

impl From<GitLabUser> for User {
    fn from(user: GitLabUser) -> User {
        let GitLabUser { username, .. } = user;
        User { username }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct GitLabRepository {
    id: u32,
    name: String,
    name_with_namespace: String,
    path: String,
    path_with_namespace: String,
    description: String,
    created_at: DateTime<Utc>,
    default_branch: String,
    web_url: String,
    forks_count: u32,
    star_count: u32,
    last_activity_at: String,
    archived: bool,
    visibility: String,
    owner: GitLabUser,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum GitLabPullRequestState {
    #[serde(rename = "opened")]
    Open,
    #[serde(rename = "closed")]
    Closed,
    #[serde(rename = "merged")]
    Merged,
    #[serde(rename = "locked")]
    Locked,
}

impl From<GitLabPullRequestState> for PullRequestState {
    fn from(state: GitLabPullRequestState) -> PullRequestState {
        match state {
            GitLabPullRequestState::Open => PullRequestState::Open,
            GitLabPullRequestState::Closed => PullRequestState::Closed,
            GitLabPullRequestState::Merged => PullRequestState::Merged,
            GitLabPullRequestState::Locked => PullRequestState::Locked,
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
    pub web_url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub author: GitLabUser,
    pub closed_by: Option<GitLabUser>,
    pub reviewers: Option<Vec<GitLabUser>>,
}

impl From<GitLabPullRequest> for PullRequest {
    fn from(pr: GitLabPullRequest) -> PullRequest {
        let GitLabPullRequest {
            iid,
            state,
            title,
            description,
            source_branch,
            target_branch,
            web_url,
            created_at,
            updated_at,
            author,
            closed_by,
            reviewers,
            ..
        } = pr;
        PullRequest {
            id: iid,
            state: state.into(),
            title,
            description,
            source: source_branch,
            target: target_branch,
            url: web_url,
            created_at,
            updated_at,
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
            target,
            close_source_branch,
        } = pr;
        Self {
            title,
            description,
            source_branch: source,
            // We are never supposed to fallback to this, but handle it
            target_branch: target.unwrap_or("master".to_string()),
            remove_source_branch: close_source_branch,
        }
    }
}

pub struct GitLab {
    settings: VersionControlSettings,
    client: Client,
    hostname: String,
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
                    "https://{}/api/v4/projects/{}{}",
                    self.hostname,
                    encode(&self.repo).into_owned(),
                    url
                ),
            )
            .header("Authorization", format!("Bearer {}", &self.settings.auth))
            .header("Content-Type", "application/json");
        if let Some(body) = body {
            request = request.json(&body);
        }
        
        let result = request.send().await?;
        let status = result.status();
        if status.is_client_error() || status.is_server_error() {
            let t = result.text().await?;
            Err(eyre!("Request failed (response: {}).", t))
        } else {
            let t: T = result.json().await?;
            Ok(t)
        }
    }

    async fn get_repository_data(&self) -> Result<GitLabRepository> {
        self.call::<GitLabRepository, i32>(Method::GET, "", None)
            .await
    }
}

#[async_trait]
impl VersionControl for GitLab {
    fn init(hostname: String, repo: String, settings: VersionControlSettings) -> Self {
        let client = Client::new();
        GitLab {
            settings,
            client,
            hostname,
            repo,
        }
    }
    async fn create_pr(&self, mut pr: CreatePullRequest) -> Result<PullRequest> {
        pr.target = pr.target.or(self.settings.default_branch.clone());
        if pr.target.is_none() {
            let GitLabRepository { default_branch, .. } = self.get_repository_data().await?;
            pr.target = Some(default_branch);
        }
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
                &format!("/merge_requests?source_branch={branch}"),
                None as Option<i32>,
            )
            .await?;

        match prs.into_iter().next() {
            Some(pr) => Ok(pr.into()),
            None => Err(eyre!("Pull request on branch {branch} not found.")),
        }
    }
    async fn list_prs(&self, filters: ListPullRequestFilters) -> Result<Vec<PullRequest>> {
        todo!();
    }
    async fn approve_pr(&self, branch: &str) -> Result<PullRequest> {
        todo!();
    }
    async fn decline_pr(&self, branch: &str) -> Result<PullRequest> {
        todo!();
    }
    async fn merge_pr(&self, branch: &str) -> Result<PullRequest> {
        todo!();
    }
}
