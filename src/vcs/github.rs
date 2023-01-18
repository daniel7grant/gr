use super::common::{CreatePullRequest, PullRequest, PullRequestState, User, VersionControl};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use color_eyre::{eyre::eyre, Result};
use reqwest::{Client, Method};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct GitHubUser {
    pub id: u32,
    pub login: String,
}

impl Into<User> for GitHubUser {
    fn into(self) -> User {
        let Self { login, .. } = self;
        User { username: login }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum GitHubPullRequestState {
    #[serde(rename = "open")]
    OPEN,
    #[serde(rename = "closed")]
    MERGED,
}

impl Into<PullRequestState> for GitHubPullRequestState {
    fn into(self) -> PullRequestState {
        match self {
            GitHubPullRequestState::OPEN => PullRequestState::OPEN,
            GitHubPullRequestState::MERGED => PullRequestState::MERGED,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GitHubPullRequestBranch {
    #[serde(rename = "ref")]
    pub branch: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GitHubPullRequest {
    pub id: u32,
    pub number: u32,
    pub state: GitHubPullRequestState,
    pub locked: bool,
    pub title: String,
    pub body: String,
    pub head: GitHubPullRequestBranch,
    pub base: GitHubPullRequestBranch,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub user: GitHubUser,
    pub merged_by: Option<GitHubUser>,
    pub requested_reviewers: Option<Vec<GitHubUser>>,
}

impl Into<PullRequest> for GitHubPullRequest {
    fn into(self) -> PullRequest {
        let Self {
            number,
            state,
            title,
            body,
            head,
            base,
            created_at,
            updated_at,
            user,
            merged_by,
            requested_reviewers,
            ..
        } = self;
        PullRequest {
            id: number,
            state: state.into(),
            title,
            description: body,
            source: head.branch,
            destination: base.branch,
            created_on: created_at,
            updated_on: updated_at,
            author: user.into(),
            closed_by: merged_by.map(|c| c.into()),
            reviewers: requested_reviewers.map(|rs| rs.into_iter().map(|r| r.into()).collect()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GitHubCreatePullRequest {
    pub title: String,
    pub body: String,
    pub head: String,
    pub base: String,
}

impl From<CreatePullRequest> for GitHubCreatePullRequest {
    fn from(pr: CreatePullRequest) -> Self {
        let CreatePullRequest {
            title,
            description,
            source,
            destination,
            ..
        } = pr;
        Self {
            title,
            body: description,
            head: source,
            base: destination,
        }
    }
}

pub struct GitHub {
    auth: (String, String),
    client: Client,
    project: String,
    repo: String,
}

impl GitHub {
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
                    "https://api.github.com/repos/{}/{}{}",
                    self.project, self.repo, url
                ),
            )
            .header("User-Agent", "gr")
            .header("Authorization", format!("Bearer {}", &self.auth.1))
            .header("Content-Type", "application/json");
        if let Some(body) = body {
            request = request.json(&body);
        }
        let result = request.send().await?;

        // let t = result.text().await?;
        // dbg!(t);
        // todo!();

        let t: T = result.json().await?;
        Ok(t)
    }
}

#[async_trait]
impl VersionControl for GitHub {
    fn init(auth: (String, String), (project, repo): (String, String)) -> Self {
        let client = Client::new();
        GitHub {
            auth,
            client,
            project,
            repo,
        }
    }
    async fn create_pr(&self, pr: CreatePullRequest) -> Result<PullRequest> {
        let new_pr: GitHubPullRequest = self
            .call(
                Method::POST,
                "/pulls",
                Some(GitHubCreatePullRequest::from(pr)),
            )
            .await?;

        Ok(new_pr.into())
    }
    async fn get_pr(&self, branch: &str) -> Result<PullRequest> {
        let prs: Vec<GitHubPullRequest> = self
            .call(
                Method::GET,
                &format!("/pulls?state=open&head={}", branch),
                None as Option<i32>,
            )
            .await?;

        match prs.into_iter().next() {
            Some(pr) => Ok(pr.into()),
            None => Err(eyre!("Pull request on branch {branch} not found.")),
        }
    }
}
