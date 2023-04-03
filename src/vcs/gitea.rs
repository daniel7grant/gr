// Documentation: https://codeberg.org/api/swagger
use super::common::{
    CreatePullRequest, ListPullRequestFilters, PullRequest, PullRequestState,
    PullRequestStateFilter, Repository, User, VersionControl, VersionControlSettings,
};
use eyre::{eyre, ContextCompat, Result};
use native_tls::TlsConnector;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{fmt::Debug, sync::Arc};
use time::OffsetDateTime;
use tracing::{info, instrument, trace};
use ureq::{Agent, AgentBuilder, Error};

#[derive(Debug, Deserialize, Serialize)]
pub struct GiteaUser {
    pub id: u32,
    pub login: String,
}

impl From<GiteaUser> for User {
    fn from(user: GiteaUser) -> User {
        let GiteaUser { id, login, .. } = user;
        User {
            id: id.to_string(),
            username: login,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GiteaPullRequestReview {
    pub id: u32,
    pub body: u32,
    pub user: GiteaUser,
}

#[derive(Debug, Deserialize, Serialize)]
struct GiteaRepository {
    name: String,
    full_name: String,
    private: bool,
    owner: GiteaUser,
    html_url: String,
    description: Option<String>,
    #[serde(with = "time::serde::iso8601")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    updated_at: OffsetDateTime,
    archived: bool,
    default_branch: String,
    stars_count: u32,
    forks_count: u32,
}

impl From<GiteaRepository> for Repository {
    fn from(repo: GiteaRepository) -> Repository {
        let GiteaRepository {
            name,
            full_name,
            private,
            owner,
            html_url,
            description,
            created_at,
            updated_at,
            archived,
            default_branch,
            stars_count,
            forks_count,
        } = repo;
        Repository {
            name,
            full_name,
            private,
            owner: Some(owner.into()),
            html_url,
            description: description.unwrap_or_default(),
            created_at,
            updated_at,
            archived,
            default_branch,
            stars_count,
            forks_count,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum GiteaPullRequestState {
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "closed")]
    Closed,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GiteaPullRequestBranch {
    #[serde(rename = "ref")]
    pub branch: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GiteaPullRequest {
    pub id: u32,
    pub number: u32,
    pub state: GiteaPullRequestState,
    pub title: String,
    pub body: Option<String>,
    pub head: GiteaPullRequestBranch,
    pub base: GiteaPullRequestBranch,
    pub html_url: String,
    #[serde(with = "time::serde::iso8601")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    pub updated_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601::option")]
    pub merged_at: Option<OffsetDateTime>,
    pub user: GiteaUser,
    pub merged_by: Option<GiteaUser>,
    pub requested_reviewers: Option<Vec<GiteaUser>>,
}

impl From<GiteaPullRequest> for PullRequest {
    fn from(pr: GiteaPullRequest) -> PullRequest {
        let GiteaPullRequest {
            number,
            state,
            title,
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
            state: match (state, merged_at) {
                (GiteaPullRequestState::Open, _) => PullRequestState::Open,
                (GiteaPullRequestState::Closed, Some(_)) => PullRequestState::Merged,
                (GiteaPullRequestState::Closed, None) => PullRequestState::Closed,
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
pub struct GiteaCreatePullRequest {
    pub title: String,
    pub body: String,
    pub head: String,
    pub base: String,
}

impl From<CreatePullRequest> for GiteaCreatePullRequest {
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
pub enum GiteaCreatePullRequestReviewEvent {
    #[serde(rename = "APPROVE")]
    Approve,
    #[serde(rename = "REQUEST_CHANGES")]
    RequestChanges,
    #[serde(rename = "COMMENT")]
    Comment,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GiteaCreatePullRequestReview {
    event: GiteaCreatePullRequestReviewEvent,
    #[serde(skip_serializing_if = "Option::is_none")]
    body: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GiteaCreatePullRequestReviewers {
    reviewers: Vec<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct GiteaUpdatePullRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<GiteaPullRequestState>,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum GiteaMergePullRequestDo {
    #[serde(rename = "merge")]
    Merge,
    #[serde(rename = "rebase")]
    Rebase,
    #[serde(rename = "rebase-merge")]
    RebaseMerge,
    #[serde(rename = "squash")]
    Squash,
    #[serde(rename = "manually-merged")]
    ManuallyMerged,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GiteaMergePullRequest {
    #[serde(rename = "Do")]
    pub do_this: GiteaMergePullRequestDo,
}

#[derive(Debug)]
pub struct Gitea {
    hostname: String,
    settings: VersionControlSettings,
    client: Agent,
    repo: String,
}

impl Gitea {
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
        let url = format!("https://{}/api/v1{}", self.hostname, url);

        info!("Calling with {method} on {url}.");

        let token = &self.settings.auth;

        trace!("Authenticating with token '{token}'.");

        let request = self
            .client
            .request(method, &url)
            .set("Authorization", &format!("token {token}"))
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
    fn get_repository_data(&self) -> Result<GiteaRepository> {
        self.call::<GiteaRepository, i32>("GET", &self.get_repository_url(""), None)
    }

    #[instrument(skip(self))]
    fn call_paginated<T: DeserializeOwned>(&self, url: &str, params: &str) -> Result<Vec<T>> {
        let mut collected_values: Vec<T> = vec![];
        let mut i = 1;
        loop {
            info!("Reading page {}.", i);

            let mut page: Vec<T> = self.call(
                "GET",
                &format!("{url}?page={i}{params}"),
                None as Option<i32>,
            )?;

            if page.is_empty() {
                break;
            }

            collected_values.append(&mut page);

            i += 1;
        }
        Ok(collected_values)
    }
}

impl VersionControl for Gitea {
    #[instrument(skip_all)]
    fn init(hostname: String, repo: String, settings: VersionControlSettings) -> Self {
        let client = AgentBuilder::new()
            .tls_connector(Arc::new(TlsConnector::new().unwrap()))
            .build();
        Gitea {
            hostname,
            settings,
            client,
            repo,
        }
    }

    #[instrument(skip_all)]
    fn login_url(&self) -> String {
        format!("https://{}/user/settings/applications", self.hostname)
    }

    #[instrument(skip_all)]
    fn validate_token(&self, token: &str) -> Result<()> {
        if token.len() != 40 {
            Err(eyre!("Your Gitea token has to be 40 characters long."))
        } else {
            Ok(())
        }
    }

    #[instrument(skip(self))]
    fn create_pr(&self, mut pr: CreatePullRequest) -> Result<PullRequest> {
        let reviewers = pr.reviewers.clone();
        pr.target = pr.target.or(self.settings.default_branch.clone());
        if pr.target.is_none() {
            let GiteaRepository { default_branch, .. } = self.get_repository_data()?;
            pr.target = Some(default_branch);
        }

        let new_pr: GiteaPullRequest = self.call(
            "POST",
            &self.get_repository_url("/pulls"),
            Some(GiteaCreatePullRequest::from(pr)),
        )?;

        let _: Vec<GiteaPullRequestReview> = self.call(
            "POST",
            &self.get_repository_url(&format!("/pulls/{}/requested_reviewers", new_pr.number)),
            Some(GiteaCreatePullRequestReviewers { reviewers }),
        )?;

        Ok(new_pr.into())
    }

    #[instrument(skip(self))]
    fn get_pr_by_id(&self, id: u32) -> Result<PullRequest> {
        let pr: GiteaPullRequest = self.call(
            "GET",
            &self.get_repository_url(&format!("/pulls/{id}")),
            None as Option<i32>,
        )?;

        Ok(pr.into())
    }

    #[instrument(skip(self))]
    fn get_pr_by_branch(&self, branch: &str) -> Result<PullRequest> {
        let prs: Vec<GiteaPullRequest> =
            self.call_paginated(&self.get_repository_url(&format!("/pulls")), "&state=all")?;

        prs.into_iter()
            .find(|pr| pr.head.branch == branch)
            .map(|pr| pr.into())
            .wrap_err(eyre!("Pull request on branch {branch} not found."))
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
        let prs: Vec<GiteaPullRequest> = self.call(
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
            Some(GiteaCreatePullRequestReview {
                event: GiteaCreatePullRequestReviewEvent::Approve,
                body: None,
            }),
        )?;

        Ok(())
    }

    #[instrument(skip(self))]
    fn close_pr(&self, id: u32) -> Result<PullRequest> {
        let closing = GiteaUpdatePullRequest {
            state: Some(GiteaPullRequestState::Closed),
            ..GiteaUpdatePullRequest::default()
        };
        let pr: GiteaPullRequest = self.call(
            "PATCH",
            &self.get_repository_url(&format!("/pulls/{id}")),
            Some(closing),
        )?;

        Ok(pr.into())
    }

    #[instrument(skip(self))]
    fn merge_pr(&self, id: u32, _: bool) -> Result<PullRequest> {
        let _: () = self.call(
            "POST",
            &self.get_repository_url(&format!("/pulls/{id}/merge")),
            Some(GiteaMergePullRequest {
                do_this: GiteaMergePullRequestDo::Merge,
            }),
        )?;

        self.get_pr_by_id(id)
    }

    #[instrument(skip_all)]
    fn get_repository(&self) -> Result<Repository> {
        let repo = self.get_repository_data()?;

        Ok(repo.into())
    }
}
