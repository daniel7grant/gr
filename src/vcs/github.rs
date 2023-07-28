// Documentation: https://docs.github.com/en/rest/quickstart
use super::common::{
    CreatePullRequest, CreateRepository, ForkRepository, ForkedFromRepository,
    ListPullRequestFilters, PullRequest, PullRequestState, PullRequestStateFilter, Repository,
    RepositoryVisibility, User, VersionControl, VersionControlSettings,
};
use eyre::{eyre, ContextCompat, Result};
use native_tls::TlsConnector;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{fmt::Debug, sync::Arc};
use time::OffsetDateTime;
use tracing::{info, instrument, trace};
use ureq::{Agent, AgentBuilder, Error};

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
    ssh_url: String,
    clone_url: String,
    description: Option<String>,
    #[serde(with = "time::serde::iso8601")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    updated_at: OffsetDateTime,
    stargazers_count: u32,
    forks_count: u32,
    archived: bool,
    default_branch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent: Option<Box<GitHubRepository>>,
}

#[derive(Debug, Deserialize, Serialize)]
struct GitHubCreateRepository {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    private: bool,
    auto_init: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    gitignore_template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    license_template: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct GitHubForkRepository {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    organization: Option<String>,
}

impl From<GitHubRepository> for Repository {
    fn from(repo: GitHubRepository) -> Repository {
        let GitHubRepository {
            name,
            full_name,
            private,
            owner,
            html_url,
            ssh_url,
            clone_url,
            description,
            created_at,
            updated_at,
            stargazers_count,
            forks_count,
            archived,
            default_branch,
            parent,
        } = repo;
        Repository {
            name,
            full_name,
            owner: Some(owner.into()),
            visibility: if private {
                RepositoryVisibility::Private
            } else {
                RepositoryVisibility::Public
            },
            html_url,
            ssh_url,
            https_url: clone_url,
            description: description.unwrap_or_default(),
            created_at,
            updated_at,
            archived,
            default_branch,
            stars_count: stargazers_count,
            forks_count,
            forked_from: parent.map(|r| ForkedFromRepository::from(*r)),
        }
    }
}

impl From<GitHubRepository> for ForkedFromRepository {
    fn from(repo: GitHubRepository) -> ForkedFromRepository {
        let GitHubRepository {
            name,
            full_name,
            html_url,
            ..
        } = repo;
        ForkedFromRepository {
            name,
            full_name,
            html_url,
        }
    }
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
    pub sha: String,
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
    #[serde(with = "time::serde::iso8601")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601::option")]
    pub merged_at: Option<OffsetDateTime>,
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
            source_sha: head.sha,
            target_sha: base.sha,
            url: html_url,
            created_at,
            updated_at,
            author: user.into(),
            closed_by: merged_by.map(|c| c.into()),
            reviewers: requested_reviewers.map(|rs| rs.into_iter().map(|r| r.into()).collect()),
            delete_source_branch: false,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GitHubCreatePullRequest {
    pub title: String,
    pub body: String,
    pub head: String,
    pub base: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head_repo: Option<String>,
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
            head_repo: None,
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
    client: Agent,
    repo: String,
    hostname: String,
}

impl GitHub {
    #[instrument(skip_all)]
    fn get_repository_url(&self, url: &str) -> String {
        format!("/repos/{}{}", self.repo, url)
    }
    #[instrument(skip_all)]
    fn call<T: DeserializeOwned, U: Serialize + Debug>(
        &self,
        method: &str,
        url: &str,
        body: Option<U>,
    ) -> Result<T> {
        // Base URL is api.github.com or /api/v3, see https://stackoverflow.com/a/50612869
        let hostname = match self.hostname.as_str() {
            "github.com" => "api.github.com".to_string(),
            hostname => format!("{hostname}/api/v3"),
        };
        let url = format!("https://{}{}", hostname, url);

        info!("Calling with {method} on {url}.");

        let token = &self.settings.auth;

        trace!("Authenticating with token '{token}'.");

        let request = self
            .client
            .request(method, &url)
            .set("User-Agent", "gr")
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
                let mut t = result.into_string()?;

                info!(
                    "Received response with response code {} with body size {}.",
                    status,
                    t.len()
                );
                trace!("Response body: {t}.");

                // Somewhat hacky, if the response is empty, return null
                if t.is_empty() {
                    t = "null".to_string();
                }

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
    fn get_repository_data(&self) -> Result<GitHubRepository> {
        self.call::<GitHubRepository, i32>("GET", &self.get_repository_url(""), None)
    }
}

impl VersionControl for GitHub {
    #[instrument(skip_all)]
    fn init(hostname: String, repo: String, settings: VersionControlSettings) -> Self {
        let client = AgentBuilder::new()
            .tls_connector(Arc::new(TlsConnector::new().unwrap()))
            .build();

        GitHub {
            settings,
            client,
            repo,
            hostname,
        }
    }
    #[instrument(skip_all)]
    fn login_url(&self) -> String {
        format!(
            "https://{}/settings/tokens/new?description=gr&scopes=repo,project",
            self.hostname
        )
    }

    #[instrument(skip_all)]
    fn validate_token(&self, token: &str) -> Result<()> {
        if !token.starts_with("ghp_") {
            Err(eyre!("Your GitHub token has to start with `ghp`."))
        } else if token.len() != 40 {
            Err(eyre!("Your GitHub token has to be 40 characters long."))
        } else {
            Ok(())
        }
    }

    #[instrument(skip(self))]
    fn create_pr(&self, mut pr: CreatePullRequest) -> Result<PullRequest> {
        let reviewers = pr.reviewers.clone();
        pr.target = pr.target.or(self.settings.default_branch.clone());
        if pr.target.is_none() {
            let GitHubRepository { default_branch, .. } = self.get_repository_data()?;
            pr.target = Some(default_branch);
        }

        let mut url = self.get_repository_url("/pulls");
        let mut github_pr = GitHubCreatePullRequest::from(pr);
        if self.settings.fork {
            let repo = self.get_repository()?;
            if let Some(forked) = repo.forked_from {
                url = format!("/repos/{}/pulls", forked.full_name);
                github_pr.head_repo = Some(repo.full_name);
            }
        }

        let new_pr: GitHubPullRequest = self.call("POST", &url, Some(github_pr))?;

        let _: GitHubPullRequest = self.call(
            "POST",
            &format!("{}/{}/requested_reviewers", url, new_pr.number),
            Some(GitHubCreatePullRequestReviewers { reviewers }),
        )?;

        Ok(new_pr.into())
    }

    #[instrument(skip(self))]
    fn get_pr_by_id(&self, id: u32) -> Result<PullRequest> {
        let pr: GitHubPullRequest = self.call(
            "GET",
            &self.get_repository_url(&format!("/pulls/{id}")),
            None as Option<i32>,
        )?;

        Ok(pr.into())
    }

    #[instrument(skip(self))]
    fn get_pr_by_branch(&self, branch: &str) -> Result<PullRequest> {
        // TODO: is this the correct head for a repo?
        let (head, _) = self
            .repo
            .split_once('/')
            .wrap_err(eyre!("Invalid repo format: {}.", self.repo))?;

        let prs: Vec<GitHubPullRequest> = self.call(
            "GET",
            &self.get_repository_url(&format!("/pulls?state=all&head={head}:{branch}")),
            None as Option<i32>,
        )?;

        match prs.into_iter().next() {
            Some(pr) => Ok(pr.into()),
            None => Err(eyre!("Pull request on branch {branch} not found.")),
        }
    }

    #[instrument(skip(self))]
    fn list_prs(&self, filters: ListPullRequestFilters) -> Result<Vec<PullRequest>> {
        let state = match filters.state {
            PullRequestStateFilter::Open => "open",
            PullRequestStateFilter::Closed
            | PullRequestStateFilter::Merged
            | PullRequestStateFilter::Locked => "closed",
            PullRequestStateFilter::All => "all",
        };
        let prs: Vec<GitHubPullRequest> = self.call(
            "GET",
            &self.get_repository_url(&format!("/pulls?state={state}")),
            None as Option<i32>,
        )?;

        Ok(prs.into_iter().map(|pr| pr.into()).collect())
    }

    #[instrument(skip(self))]
    fn approve_pr(&self, id: u32) -> Result<()> {
        self.call(
            "POST",
            &self.get_repository_url(&format!("/pulls/{id}/reviews")),
            Some(GitHubCreatePullRequestReview {
                event: GitHubCreatePullRequestReviewEvent::Approve,
                body: None,
            }),
        )?;

        Ok(())
    }

    #[instrument(skip(self))]
    fn close_pr(&self, id: u32) -> Result<PullRequest> {
        let closing = GitHubUpdatePullRequest {
            state: Some(GitHubPullRequestState::Closed),
            ..GitHubUpdatePullRequest::default()
        };
        let pr: GitHubPullRequest = self.call(
            "PATCH",
            &self.get_repository_url(&format!("/pulls/{id}")),
            Some(closing),
        )?;

        Ok(pr.into())
    }

    #[instrument(skip(self))]
    fn merge_pr(&self, id: u32, _: bool) -> Result<PullRequest> {
        let _: GitHubPullRequestMerged = self.call(
            "PUT",
            &self.get_repository_url(&format!("/pulls/{id}/merge")),
            None as Option<i32>,
        )?;

        self.get_pr_by_id(id)
    }

    #[instrument(skip_all)]
    fn get_repository(&self) -> Result<Repository> {
        let repo = self.get_repository_data()?;

        Ok(repo.into())
    }

    #[instrument(skip_all)]
    fn create_repository(&self, repo: CreateRepository) -> Result<Repository> {
        let CreateRepository {
            name,
            description,
            visibility,
            organization,
            init,
            default_branch: _,
            gitignore,
            license,
        } = repo;
        let create_repo: GitHubCreateRepository = GitHubCreateRepository {
            name,
            description,
            private: visibility != RepositoryVisibility::Public,
            auto_init: init,
            gitignore_template: gitignore,
            license_template: license,
        };
        let new_repo: GitHubRepository = if let Some(org) = organization {
            self.call("POST", &format!("/orgs/{org}/repos"), Some(create_repo))
        } else {
            self.call("POST", "/user/repos", Some(create_repo))
        }?;

        Ok(new_repo.into())
    }

    #[instrument(skip_all)]
    fn fork_repository(&self, repo: ForkRepository) -> Result<Repository> {
        let ForkRepository { name, organization } = repo;

        let new_repo: GitHubRepository = self.call(
            "POST",
            &self.get_repository_url("/forks"),
            Some(GitHubForkRepository { name, organization }),
        )?;

        Ok(new_repo.into())
    }

    #[instrument(skip_all)]
    fn delete_repository(&self) -> Result<()> {
        self.call("DELETE", &self.get_repository_url(""), None as Option<i32>)?;

        Ok(())
    }
}
