// Documentation: https://docs.github.com/en/rest/quickstart
use super::common::{
    CreatePullRequest, ListPullRequestFilters, PullRequest, PullRequestState,
    PullRequestStateFilter, User, VersionControl, VersionControlSettings,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use color_eyre::{eyre::eyre, Result};
use reqwest::{Client, Method};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::Debug;
use tracing::{info, instrument, trace};

#[derive(Debug, Deserialize, Serialize)]
pub struct GitHubUser {
    pub id: u32,
    pub login: String,
}

impl From<GitHubUser> for User {
    fn from(user: GitHubUser) -> User {
        let GitHubUser { id, login, .. } = user;
        User {
            id: id.to_string(),
            username: login,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct GitHubRepository {
    name: String,
    full_name: String,
    private: bool,
    owner: GitHubUser,
    html_url: String,
    description: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    stargazers_count: u32,
    archived: bool,
    disabled: bool,
    default_branch: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum GitHubPullRequestState {
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "closed")]
    Closed,
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
    pub body: Option<String>,
    pub head: GitHubPullRequestBranch,
    pub base: GitHubPullRequestBranch,
    pub html_url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub merged_at: Option<DateTime<Utc>>,
    pub user: GitHubUser,
    pub merged_by: Option<GitHubUser>,
    pub requested_reviewers: Option<Vec<GitHubUser>>,
}

impl From<GitHubPullRequest> for PullRequest {
    fn from(pr: GitHubPullRequest) -> PullRequest {
        let GitHubPullRequest {
            number,
            state,
            title,
            locked,
            body,
            head,
            base,
            html_url,
            created_at,
            updated_at,
            merged_at,
            user,
            merged_by,
            requested_reviewers,
            ..
        } = pr;
        PullRequest {
            id: number,
            state: match (state, merged_at, locked) {
                (_, _, true) => PullRequestState::Locked,
                (GitHubPullRequestState::Open, _, _) => PullRequestState::Open,
                (GitHubPullRequestState::Closed, Some(_), _) => PullRequestState::Merged,
                (GitHubPullRequestState::Closed, None, _) => PullRequestState::Closed,
            },
            title,
            description: body.unwrap_or_default(),
            source: head.branch,
            target: base.branch,
            url: html_url,
            created_at,
            updated_at,
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
            target: destination,
            ..
        } = pr;
        Self {
            title,
            body: description,
            head: source,
            // We are never supposed to fallback to this, but handle it
            base: destination.unwrap_or("master".to_string()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum GitHubCreatePullRequestReviewEvent {
    #[serde(rename = "APPROVE")]
    Approve,
    #[serde(rename = "REQUEST_CHANGES")]
    RequestChanges,
    #[serde(rename = "COMMENT")]
    Comment,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GitHubCreatePullRequestReview {
    event: GitHubCreatePullRequestReviewEvent,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GitHubCreatePullRequestReviewers {
    reviewers: Vec<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct GitHubUpdatePullRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<GitHubPullRequestState>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GitHubPullRequestMerged {
    merged: bool,
    message: String,
}

#[derive(Debug)]
pub struct GitHub {
    settings: VersionControlSettings,
    client: Client,
    repo: String,
}

impl GitHub {
    #[instrument(skip_all)]
    fn get_repository_url(&self, url: &str) -> String {
        format!("/repos/{}{}", self.repo, url)
    }
    #[instrument(skip_all)]
    async fn call<T: DeserializeOwned, U: Serialize + Debug>(
        &self,
        method: Method,
        url: &str,
        body: Option<U>,
    ) -> Result<T> {
        let url = format!("https://api.github.com{}", url);

        info!("Calling with {method} on {url}.");

        let token = &self.settings.auth;

        trace!("Authenticating with token '{token}'.");

        let mut request = self
            .client
            .request(method, url)
            .header("User-Agent", "gr")
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json");
        if let Some(body) = body {
            request = request.json(&body);

            trace!("Sending body: {}.", serde_json::to_string(&body)?);
        }

        let result = request.send().await?;
        let status = result.status();
        let t = result.text().await?;

        info!(
            "Received response with response code {} with body size {}.",
            status,
            t.len()
        );
        trace!("Response body: {t}.");

        if status.is_client_error() || status.is_server_error() {
            Err(eyre!("Request failed (response: {}).", t))
        } else {
            let t: T = serde_json::from_str(&t)?;
            Ok(t)
        }
    }

    #[instrument(skip_all)]
    async fn get_repository_data(&self) -> Result<GitHubRepository> {
        self.call::<GitHubRepository, i32>(Method::GET, &self.get_repository_url(""), None)
            .await
    }
}

#[async_trait]
impl VersionControl for GitHub {
    #[instrument(skip_all)]
    fn init(_: String, repo: String, settings: VersionControlSettings) -> Self {
        let client = Client::new();
        GitHub {
            settings,
            client,
            repo,
        }
    }
    #[instrument(skip_all)]
    fn login_url(&self) -> String {
        "https://github.com/settings/tokens/new?description=gr&scopes=repo,project".to_string()
    }

    #[instrument(skip_all)]
    fn validate_token(&self, token: &str) -> Result<()> {
        if token.starts_with("ghp_") {
            Err(eyre!("Your GitHub token has to start with `ghp`."))
        } else if token.len() != 40 {
            Err(eyre!("Your GitHub token has to be 40 characters long."))
        } else {
            Ok(())
        }
    }

    #[instrument(skip(self))]
    async fn create_pr(&self, mut pr: CreatePullRequest) -> Result<PullRequest> {
        let reviewers = pr.reviewers.clone();
        pr.target = pr.target.or(self.settings.default_branch.clone());
        if pr.target.is_none() {
            let GitHubRepository { default_branch, .. } = self.get_repository_data().await?;
            pr.target = Some(default_branch);
        }
        let new_pr: GitHubPullRequest = self
            .call(
                Method::POST,
                &self.get_repository_url("/pulls"),
                Some(GitHubCreatePullRequest::from(pr)),
            )
            .await?;

        let _: GitHubPullRequest = self
            .call(
                Method::POST,
                &self.get_repository_url(&format!("/pulls/{}/requested_reviewers", new_pr.number)),
                Some(GitHubCreatePullRequestReviewers { reviewers }),
            )
            .await?;

        Ok(new_pr.into())
    }

    #[instrument(skip(self))]
    async fn get_pr_by_id(&self, id: u32) -> Result<PullRequest> {
        let pr: GitHubPullRequest = self
            .call(
                Method::GET,
                &self.get_repository_url(&format!("/pulls/{id}")),
                None as Option<i32>,
            )
            .await?;

        Ok(pr.into())
    }

    #[instrument(skip(self))]
    async fn get_pr_by_branch(&self, branch: &str) -> Result<PullRequest> {
        let prs: Vec<GitHubPullRequest> = self
            .call(
                Method::GET,
                &self.get_repository_url(&format!("/pulls?state=all&head={}", branch)),
                None as Option<i32>,
            )
            .await?;

        match prs.into_iter().next() {
            Some(pr) => Ok(pr.into()),
            None => Err(eyre!("Pull request on branch {branch} not found.")),
        }
    }

    #[instrument(skip(self))]
    async fn list_prs(&self, filters: ListPullRequestFilters) -> Result<Vec<PullRequest>> {
        let state = match filters.state {
            PullRequestStateFilter::Open => "open",
            PullRequestStateFilter::Closed
            | PullRequestStateFilter::Merged
            | PullRequestStateFilter::Locked => "closed",
            PullRequestStateFilter::All => "all",
        };
        let prs: Vec<GitHubPullRequest> = self
            .call(
                Method::GET,
                &self.get_repository_url(&format!("/pulls?state={state}")),
                None as Option<i32>,
            )
            .await?;

        Ok(prs.into_iter().map(|pr| pr.into()).collect())
    }

    #[instrument(skip(self))]
    async fn approve_pr(&self, id: u32) -> Result<()> {
        self.call(
            Method::POST,
            &self.get_repository_url(&format!("/pulls/{id}/reviews")),
            Some(GitHubCreatePullRequestReview {
                event: GitHubCreatePullRequestReviewEvent::Approve,
                body: None,
            }),
        )
        .await?;

        Ok(())
    }

    #[instrument(skip(self))]
    async fn close_pr(&self, id: u32) -> Result<PullRequest> {
        let closing = GitHubUpdatePullRequest {
            state: Some(GitHubPullRequestState::Closed),
            ..GitHubUpdatePullRequest::default()
        };
        let pr: GitHubPullRequest = self
            .call(
                Method::PATCH,
                &self.get_repository_url(&format!("/pulls/{id}")),
                Some(closing),
            )
            .await?;

        Ok(pr.into())
    }

    #[instrument(skip(self))]
    async fn merge_pr(&self, id: u32, _: bool) -> Result<PullRequest> {
        let _: GitHubPullRequestMerged = self
            .call(
                Method::PUT,
                &self.get_repository_url(&format!("/pulls/{id}/merge")),
                None as Option<i32>,
            )
            .await?;

        self.get_pr_by_id(id).await
    }
}
