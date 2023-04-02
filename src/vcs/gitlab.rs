// Documentation: https://docs.gitlab.com/ee/api/api_resources.html
use super::common::{
    CreatePullRequest, ListPullRequestFilters, PullRequest, PullRequestState,
    PullRequestStateFilter, PullRequestUserFilter, Repository, User, VersionControl,
    VersionControlSettings,
};
use eyre::{eyre, Result};
use native_tls::TlsConnector;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{fmt::Debug, sync::Arc};
use time::OffsetDateTime;
use tracing::{info, instrument, trace};
use ureq::{Agent, AgentBuilder, Error};
use urlencoding::encode;

#[derive(Debug, Deserialize, Serialize)]
pub struct GitLabUser {
    pub id: u32,
    pub username: String,
    pub name: String,
}

impl From<GitLabUser> for User {
    fn from(user: GitLabUser) -> User {
        let GitLabUser { id, username, .. } = user;
        User {
            id: id.to_string(),
            username,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GitLabApprovalUser {
    user: GitLabUser,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GitLabApproval {
    approved: bool,
    approved_by: Vec<GitLabApprovalUser>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub enum GitLabRepositoryVisibility {
    Private,
    Public,
}

#[derive(Debug, Deserialize, Serialize)]
struct GitLabRepository {
    id: u32,
    name: String,
    name_with_namespace: String,
    path: String,
    path_with_namespace: String,
    description: Option<String>,
    #[serde(with = "time::serde::iso8601")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    last_activity_at: OffsetDateTime,
    default_branch: String,
    web_url: String,
    forks_count: u32,
    star_count: u32,
    archived: bool,
    visibility: GitLabRepositoryVisibility,
    owner: Option<GitLabUser>,
}

impl From<GitLabRepository> for Repository {
    fn from(repo: GitLabRepository) -> Repository {
        let GitLabRepository {
            name,
            path_with_namespace,
            description,
            created_at,
            default_branch,
            web_url,
            forks_count,
            star_count,
            last_activity_at,
            archived,
            visibility,
            owner,
            ..
        } = repo;
        Repository {
            name,
            full_name: path_with_namespace,
            owner: owner.map(|o| o.into()),
            html_url: web_url,
            description: description.unwrap_or_default(),
            created_at,
            updated_at: last_activity_at,
            private: visibility == GitLabRepositoryVisibility::Private,
            archived,
            default_branch,
            forks_count,
            stars_count: star_count,
        }
    }
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
    #[serde(with = "time::serde::iso8601")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    pub updated_at: OffsetDateTime,
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
    pub reviewer_ids: Vec<String>,
}

impl From<CreatePullRequest> for GitLabCreatePullRequest {
    fn from(pr: CreatePullRequest) -> Self {
        let CreatePullRequest {
            title,
            description,
            source,
            target,
            close_source_branch,
            reviewers,
        } = pr;
        Self {
            title,
            description,
            source_branch: source,
            // We are never supposed to fallback to this, but handle it
            target_branch: target.unwrap_or("master".to_string()),
            remove_source_branch: close_source_branch,
            reviewer_ids: reviewers,
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub enum GitLabUpdatePullRequestStateEvent {
    #[serde(rename = "reopen")]
    Reopen,
    #[default]
    #[serde(rename = "close")]
    Close,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct GitLabUpdatePullRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remove_source_branch: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_event: Option<GitLabUpdatePullRequestStateEvent>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GitLabMergePullRequest {
    pub should_remove_source_branch: bool,
}

#[derive(Debug)]
pub struct GitLab {
    settings: VersionControlSettings,
    client: Agent,
    hostname: String,
    repo: String,
}

impl GitLab {
    #[instrument(skip_all)]
    fn get_repository_url(&self, url: &str) -> String {
        format!("/projects/{}{}", encode(&self.repo).into_owned(), url)
    }

    #[instrument(skip_all)]
    fn call<T: DeserializeOwned, U: Serialize + Debug>(
        &self,
        method: &str,
        url: &str,
        body: Option<U>,
    ) -> Result<T> {
        let url = format!("https://{}/api/v4{}", self.hostname, url);

        info!("Calling with {method} on {url}.");

        let token = &self.settings.auth;

        trace!("Authenticating with token '{token}'.");

        let request = self
            .client
            .request(method, &url)
            .set("Authorization", &format!("Bearer {}", token))
            .set("Content-Type", "application/json");
        let result = if let Some(body) = &body {
            trace!("Sending body: {}.", serde_json::to_string(&body)?);
            request.send_json(body)
        } else {
            request.call()
        };

        match result {
            Ok(result) => {
                let status = result.status();
                let t = result.into_string()?;

                info!(
                    "Received response with response code {} with body size {}.",
                    status,
                    t.len()
                );
                trace!("Response body: {t}.");
                let t: T = serde_json::from_str(&t)?;
                Ok(t)
            }
            Err(Error::Status(status, result)) => {
                let t = result.into_string()?;

                info!(
                    "Received response with response code {} with body size {}.",
                    status,
                    t.len()
                );
                Err(eyre!("Request failed (response: {}).", t))
            }
            Err(Error::Transport(_)) => Err(eyre!("Sending data failed.")),
        }
    }

    #[instrument(skip_all)]
    fn get_repository_data(&self) -> Result<GitLabRepository> {
        self.call::<GitLabRepository, i32>("GET", &self.get_repository_url(""), None)
    }

    #[instrument(skip(self))]
    fn get_user_by_name(&self, username: &str) -> Result<User> {
        let users: Vec<GitLabUser> = self.call(
            "GET",
            &format!("/users?username={username}"),
            None as Option<i32>,
        )?;

        match users.into_iter().next() {
            Some(user) => Ok(user.into()),
            None => Err(eyre!("User with name {username} not found.")),
        }
    }
}

impl VersionControl for GitLab {
    #[instrument(skip_all)]
    fn init(hostname: String, repo: String, settings: VersionControlSettings) -> Self {
        let client = AgentBuilder::new()
            .tls_connector(Arc::new(TlsConnector::new().unwrap()))
            .build();
        GitLab {
            settings,
            client,
            hostname,
            repo,
        }
    }
    #[instrument(skip_all)]
    fn login_url(&self) -> String {
        format!(
            "https://{}/-/profile/personal_access_tokens?name=gr&scopes=read_user,api",
            self.hostname
        )
    }
    #[instrument(skip_all)]
    fn validate_token(&self, token: &str) -> Result<()> {
        if token.len() != 20 {
            Err(eyre!("Your GitLab token has to be 20 characters long."))
        } else {
            Ok(())
        }
    }
    #[instrument(skip(self))]
    fn create_pr(&self, mut pr: CreatePullRequest) -> Result<PullRequest> {
        let reviewers = pr
            .reviewers
            .iter()
            .map(|reviewer| self.get_user_by_name(reviewer))
            .collect::<Result<Vec<User>>>()?;

        pr.reviewers = reviewers.into_iter().map(|r| r.id).collect();

        pr.target = pr.target.or(self.settings.default_branch.clone());
        if pr.target.is_none() {
            let GitLabRepository { default_branch, .. } = self.get_repository_data()?;
            info!("Using {default_branch} as target branch.");
            pr.target = Some(default_branch);
        }
        let new_pr: GitLabPullRequest = self.call(
            "POST",
            &self.get_repository_url("/merge_requests"),
            Some(GitLabCreatePullRequest::from(pr)),
        )?;

        Ok(new_pr.into())
    }
    #[instrument(skip(self))]
    fn get_pr_by_id(&self, id: u32) -> Result<PullRequest> {
        let pr: GitLabPullRequest = self.call(
            "GET",
            &self.get_repository_url(&format!("/merge_requests/{id}")),
            None as Option<i32>,
        )?;

        Ok(pr.into())
    }
    #[instrument(skip(self))]
    fn get_pr_by_branch(&self, branch: &str) -> Result<PullRequest> {
        let prs: Vec<GitLabPullRequest> = self.call(
            "GET",
            &self.get_repository_url(&format!("/merge_requests?source_branch={branch}")),
            None as Option<i32>,
        )?;

        match prs.into_iter().next() {
            Some(pr) => Ok(pr.into()),
            None => Err(eyre!("Pull request on branch {branch} not found.")),
        }
    }
    #[instrument(skip(self))]
    fn list_prs(&self, filters: ListPullRequestFilters) -> Result<Vec<PullRequest>> {
        let scope_param = match filters.author {
            PullRequestUserFilter::All => "?scope=all",
            PullRequestUserFilter::Me => "?scope=created_by_me",
        };
        let state_param = match filters.state {
            PullRequestStateFilter::Open => "&state=opened",
            PullRequestStateFilter::Closed => "&state=closed",
            PullRequestStateFilter::Merged => "&state=merged",
            PullRequestStateFilter::Locked => "&state=locked",
            PullRequestStateFilter::All => "",
        };
        let prs: Vec<GitLabPullRequest> = self.call(
            "GET",
            &self.get_repository_url(&format!("/merge_requests{scope_param}{state_param}")),
            None as Option<i32>,
        )?;

        Ok(prs.into_iter().map(|pr| pr.into()).collect())
    }
    #[instrument(skip(self))]
    fn approve_pr(&self, id: u32) -> Result<()> {
        let _: GitLabApproval = self.call(
            "POST",
            &self.get_repository_url(&format!("/merge_requests/{id}/approve")),
            None as Option<i32>,
        )?;

        Ok(())
    }
    #[instrument(skip(self))]
    fn close_pr(&self, id: u32) -> Result<PullRequest> {
        let closing = GitLabUpdatePullRequest {
            state_event: Some(GitLabUpdatePullRequestStateEvent::Close),
            ..GitLabUpdatePullRequest::default()
        };
        let pr: GitLabPullRequest = self.call(
            "PUT",
            &self.get_repository_url(&format!("/merge_requests/{id}")),
            Some(closing),
        )?;

        Ok(pr.into())
    }
    #[instrument(skip(self))]
    fn merge_pr(&self, id: u32, should_remove_source_branch: bool) -> Result<PullRequest> {
        let pr: GitLabPullRequest = self.call(
            "PUT",
            &self.get_repository_url(&format!("/merge_requests/{id}/merge")),
            Some(GitLabMergePullRequest {
                should_remove_source_branch,
            }),
        )?;

        Ok(pr.into())
    }

    #[instrument(skip_all)]
    fn get_repository(&self) -> Result<Repository> {
        let repo = self.get_repository_data()?;

        Ok(repo.into())
    }
}
