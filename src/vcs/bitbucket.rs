// Documentation: https://developer.atlassian.com/cloud/bitbucket/rest/intro/
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
pub enum BitbucketPullRequestState {
    #[serde(rename = "OPEN")]
    Open,
    #[serde(rename = "DECLINED")]
    Declined,
    #[serde(rename = "MERGED")]
    Merged,
    #[serde(rename = "LOCKED")]
    Locked,
}

impl From<BitbucketPullRequestState> for PullRequestState {
    fn from(pr: BitbucketPullRequestState) -> PullRequestState {
        match pr {
            BitbucketPullRequestState::Open => PullRequestState::Open,
            BitbucketPullRequestState::Declined => PullRequestState::Closed,
            BitbucketPullRequestState::Merged => PullRequestState::Merged,
            BitbucketPullRequestState::Locked => PullRequestState::Locked,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketApproval {
    approved: bool,
    user: BitbucketUser,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketMembership {
    user: BitbucketUser,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct BitbucketUser {
    pub account_id: String,
    pub uuid: String,
    pub nickname: String,
    pub display_name: String,
}

impl From<BitbucketUser> for User {
    fn from(user: BitbucketUser) -> User {
        let BitbucketUser {
            account_id,
            nickname,
            ..
        } = user;
        User {
            id: account_id,
            username: nickname,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketLink {
    pub href: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketLinks {
    pub html: BitbucketLink,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketCommit {
    pub hash: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketBranch {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketRevision {
    pub branch: BitbucketBranch,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<BitbucketCommit>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketRepositoryProject {
    uuid: String,
    key: String,
    owner: User,
    name: String,
    description: String,
    is_private: bool,
    #[serde(with = "time::serde::iso8601")]
    created_on: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    updated_on: OffsetDateTime,
    has_publicly_visible_repos: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketRepository {
    uuid: String,
    name: String,
    full_name: String,
    links: BitbucketLinks,
    owner: BitbucketUser,
    description: String,
    #[serde(with = "time::serde::iso8601")]
    created_on: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    updated_on: OffsetDateTime,
    language: String,
    project: BitbucketRepositoryProject,
    mainbranch: BitbucketBranch,
    is_private: bool,
}

impl From<BitbucketRepository> for Repository {
    fn from(repo: BitbucketRepository) -> Repository {
        let BitbucketRepository {
            name,
            links,
            full_name,
            owner,
            description,
            created_on,
            updated_on,
            mainbranch,
            is_private,
            ..
        } = repo;
        Repository {
            name,
            full_name,
            owner: Some(owner.into()),
            html_url: links.html.href,
            description,
            created_at: created_on,
            updated_at: updated_on,
            private: is_private,
            archived: false,
            default_branch: mainbranch.name,
            forks_count: 0,
            stars_count: 0,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketPullRequest {
    pub id: u32,
    pub state: BitbucketPullRequestState,
    pub title: String,
    pub description: String,
    pub links: BitbucketLinks,
    #[serde(with = "time::serde::iso8601")]
    pub created_on: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    pub updated_on: OffsetDateTime,
    pub source: BitbucketRevision,
    pub destination: BitbucketRevision,
    pub author: BitbucketUser,
    pub closed_by: Option<BitbucketUser>,
    pub reviewers: Option<Vec<BitbucketUser>>,
}

impl From<BitbucketPullRequest> for PullRequest {
    fn from(pr: BitbucketPullRequest) -> PullRequest {
        let BitbucketPullRequest {
            id,
            state,
            title,
            description,
            source,
            destination,
            links,
            created_on,
            updated_on,
            author,
            closed_by,
            reviewers,
        } = pr;
        PullRequest {
            id,
            state: state.into(),
            title,
            description,
            source: source.branch.name,
            target: destination.branch.name,
            url: links.html.href,
            created_at: created_on,
            updated_at: updated_on,
            author: author.into(),
            closed_by: closed_by.map(|u| u.into()),
            reviewers: reviewers.map(|rs| rs.into_iter().map(|r| r.into()).collect()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketReviewer {
    pub uuid: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketCreatePullRequest {
    pub title: String,
    pub description: String,
    pub source: BitbucketRevision,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<BitbucketRevision>,
    pub close_source_branch: bool,
    pub reviewers: Vec<BitbucketReviewer>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketMergePullRequest {
    pub close_source_branch: bool,
}

impl From<CreatePullRequest> for BitbucketCreatePullRequest {
    fn from(pr: CreatePullRequest) -> Self {
        let CreatePullRequest {
            title,
            description,
            source,
            target: destination,
            close_source_branch,
            reviewers,
        } = pr;
        Self {
            title,
            description,
            source: BitbucketRevision {
                branch: BitbucketBranch { name: source },
                commit: None,
            },
            destination: destination.map(|name| BitbucketRevision {
                branch: BitbucketBranch { name },
                commit: None,
            }),
            close_source_branch,
            reviewers: reviewers
                .into_iter()
                .map(|uuid| BitbucketReviewer { uuid })
                .collect(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketPaginated<T> {
    pub next: Option<String>,
    pub page: u32,
    pub pagelen: u32,
    pub size: u32,
    pub values: Vec<T>,
}

#[derive(Debug)]
pub struct Bitbucket {
    settings: VersionControlSettings,
    client: Agent,
    repo: String,
}

impl Bitbucket {
    #[instrument(skip_all)]
    fn get_repository_url(&self, url: &str) -> String {
        format!("/repositories/{}{}", self.repo, url)
    }

    #[instrument(skip_all)]
    fn call<T: DeserializeOwned, U: Serialize + Debug>(
        &self,
        method: &str,
        url: &str,
        body: Option<U>,
    ) -> Result<T> {
        let url = format!("https://api.bitbucket.org/2.0{url}");

        info!("Calling with {method} {url}.");

        let (username, password) = self
            .settings
            .auth
            .split_once(':')
            .wrap_err("Authentication has to contain a username and a token.")?;

        trace!("Authenticating with username '{username}' and token '{password}'.");

        let request = self.client.request(method, &url).set(
            "Authorization",
            &format!("Basic {}", base64::encode(format!("{username}:{password}"))),
        );
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

    #[instrument(skip(self))]
    fn call_paginated<T: DeserializeOwned>(&self, url: &str, params: &str) -> Result<Vec<T>> {
        let mut collected_values: Vec<T> = vec![];
        let mut i = 1;
        loop {
            info!("Reading page {}.", i);

            let mut page: BitbucketPaginated<T> = self.call(
                "GET",
                &format!("{url}?page={i}{params}"),
                None as Option<i32>,
            )?;

            collected_values.append(&mut page.values);

            if page.next.is_none() {
                break;
            }

            i += 1;
        }
        Ok(collected_values)
    }

    #[instrument(skip(self))]
    fn get_workspace_users(&self, usernames: Vec<String>) -> Result<Vec<BitbucketUser>> {
        let (workspace, _) = self
            .repo
            .split_once('/')
            .wrap_err(eyre!("Repo URL is malformed: {}", &self.repo))?;
        let members: Vec<BitbucketMembership> =
            self.call_paginated(&format!("/workspaces/{workspace}/members"), "")?;

        Ok(members
            .into_iter()
            .map(|m| m.user)
            .filter(|u| usernames.contains(&u.nickname))
            .collect())
    }
}

impl VersionControl for Bitbucket {
    #[instrument(skip_all)]
    fn init(_: String, repo: String, settings: VersionControlSettings) -> Self {
        let client = AgentBuilder::new()
            .tls_connector(Arc::new(TlsConnector::new().unwrap()))
            .build();
        Bitbucket {
            settings,
            client,
            repo,
        }
    }
    #[instrument(skip_all)]
    fn login_url(&self) -> String {
        "https://bitbucket.org/account/settings/app-passwords/new".to_string()
    }
    #[instrument(skip_all)]
    fn validate_token(&self, token: &str) -> Result<()> {
        if !token.contains(':') {
            Err(eyre!("Enter your Bitbucket username and the token, separated with a colon (user:ABBT...)."))
        } else {
            Ok(())
        }
    }
    #[instrument(skip(self))]
    fn create_pr(&self, mut pr: CreatePullRequest) -> Result<PullRequest> {
        let reviewers = self.get_workspace_users(pr.reviewers.clone())?;
        pr.reviewers = reviewers.into_iter().map(|r| r.uuid).collect();
        let new_pr: BitbucketPullRequest = self.call(
            "POST",
            &self.get_repository_url("/pullrequests"),
            Some(BitbucketCreatePullRequest::from(pr)),
        )?;

        Ok(new_pr.into())
    }
    #[instrument(skip(self))]
    fn get_pr_by_id(&self, id: u32) -> Result<PullRequest> {
        let pr: BitbucketPullRequest = self.call(
            "GET",
            &self.get_repository_url(&format!("/pullrequests/{id}")),
            None as Option<u32>,
        )?;

        Ok(pr.into())
    }
    #[instrument(skip(self))]
    fn get_pr_by_branch(&self, branch: &str) -> Result<PullRequest> {
        let prs: Vec<BitbucketPullRequest> =
            self.call_paginated(&self.get_repository_url("/pullrequests"), "")?;

        prs.into_iter()
            .find(|pr| pr.source.branch.name == branch)
            .map(|pr| pr.into())
            .wrap_err(eyre!("Pull request on branch {branch} not found."))
    }
    #[instrument(skip(self))]
    fn list_prs(&self, filters: ListPullRequestFilters) -> Result<Vec<PullRequest>> {
        let state_param = match filters.state {
            PullRequestStateFilter::Open => "&state=OPEN",
            PullRequestStateFilter::Closed => "&state=DECLINED",
            PullRequestStateFilter::Merged => "&state=MERGED",
            PullRequestStateFilter::Locked | PullRequestStateFilter::All => "",
        };
        let prs: Vec<BitbucketPullRequest> =
            self.call_paginated(&self.get_repository_url("/pullrequests"), state_param)?;

        Ok(prs.into_iter().map(|pr| pr.into()).collect())
    }
    #[instrument(skip(self))]
    fn approve_pr(&self, id: u32) -> Result<()> {
        let _: BitbucketApproval = self.call(
            "POST",
            &self.get_repository_url(&format!("/pullrequests/{id}/approve")),
            None as Option<i32>,
        )?;

        Ok(())
    }
    #[instrument(skip(self))]
    fn close_pr(&self, id: u32) -> Result<PullRequest> {
        let pr: BitbucketPullRequest = self.call(
            "POST",
            &self.get_repository_url(&format!("/pullrequests/{id}/decline")),
            None as Option<i32>,
        )?;

        Ok(pr.into())
    }
    #[instrument(skip(self))]
    fn merge_pr(&self, id: u32, close_source_branch: bool) -> Result<PullRequest> {
        let pr: BitbucketPullRequest = self.call(
            "POST",
            &self.get_repository_url(&format!("/pullrequests/{id}/merge")),
            Some(BitbucketMergePullRequest {
                close_source_branch,
            }),
        )?;

        Ok(pr.into())
    }

    #[instrument(skip_all)]
    fn get_repository(&self) -> Result<Repository> {
        let repo =
            self.call::<BitbucketRepository, i32>("GET", &self.get_repository_url(""), None)?;

        Ok(repo.into())
    }
}
