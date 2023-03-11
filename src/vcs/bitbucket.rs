// Documentation: https://developer.atlassian.com/cloud/bitbucket/rest/intro/
use super::common::{
    CreatePullRequest, ListPullRequestFilters, PullRequest, PullRequestState,
    PullRequestStateFilter, User, VersionControl, VersionControlSettings,
};
use chrono::{DateTime, Utc};
use color_eyre::{eyre::eyre, eyre::ContextCompat, Result};
use reqwest::{blocking::Client, Method};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::Debug;
use tracing::{info, instrument, trace};

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
pub struct BitbucketPullRequestLink {
    pub href: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketPullRequestLinks {
    pub html: BitbucketPullRequestLink,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketPullRequestCommit {
    pub hash: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketPullRequestBranch {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketPullRequestRepository {
    pub name: String,
    pub full_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketPullRequestRevision {
    pub branch: BitbucketPullRequestBranch,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<BitbucketPullRequestCommit>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketPullRequest {
    pub id: u32,
    pub state: BitbucketPullRequestState,
    pub title: String,
    pub description: String,
    pub links: BitbucketPullRequestLinks,
    pub created_on: DateTime<Utc>,
    pub updated_on: DateTime<Utc>,
    pub source: BitbucketPullRequestRevision,
    pub destination: BitbucketPullRequestRevision,
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
    pub source: BitbucketPullRequestRevision,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<BitbucketPullRequestRevision>,
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
            source: BitbucketPullRequestRevision {
                branch: BitbucketPullRequestBranch { name: source },
                commit: None,
            },
            destination: destination.map(|name| BitbucketPullRequestRevision {
                branch: BitbucketPullRequestBranch { name },
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
    client: Client,
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
        method: Method,
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

        let mut request = self
            .client
            .request(method, url)
            .basic_auth(username, Some(password));
        if let Some(body) = &body {
            request = request.json(body);

            trace!("Sending body: {}.", serde_json::to_string(&body)?);
        }

        let result = request.send()?;
        let status = result.status();
        let t = result.text()?;

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

    #[instrument(skip(self))]
    fn call_paginated<T: DeserializeOwned>(&self, url: &str) -> Result<Vec<T>> {
        let mut collected_values: Vec<T> = vec![];
        let mut i = 1;
        loop {
            info!("Reading page {}.", i);

            let mut page: BitbucketPaginated<T> =
                self.call(Method::GET, &format!("{url}?page={i}"), None as Option<i32>)?;

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
            .split_once("/")
            .wrap_err(eyre!("Repo URL is malformed: {}", &self.repo))?;
        let members: Vec<BitbucketMembership> =
            self.call_paginated(&format!("/workspaces/{workspace}/members"))?;

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
        let client = Client::new();
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
        if !token.contains(":") {
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
            Method::POST,
            &self.get_repository_url("/pullrequests"),
            Some(BitbucketCreatePullRequest::from(pr)),
        )?;

        Ok(new_pr.into())
    }
    #[instrument(skip(self))]
    fn get_pr_by_id(&self, id: u32) -> Result<PullRequest> {
        let pr: BitbucketPullRequest = self.call(
            Method::GET,
            &self.get_repository_url(&format!("/pullrequests/{id}")),
            None as Option<u32>,
        )?;

        Ok(pr.into())
    }
    #[instrument(skip(self))]
    fn get_pr_by_branch(&self, branch: &str) -> Result<PullRequest> {
        let prs: Vec<BitbucketPullRequest> =
            self.call_paginated(&self.get_repository_url("/pullrequests"))?;

        prs.into_iter()
            .find(|pr| pr.source.branch.name == branch)
            .map(|pr| pr.into())
            .wrap_err(eyre!("Pull request on branch {branch} not found."))
    }
    #[instrument(skip(self))]
    fn list_prs(&self, filters: ListPullRequestFilters) -> Result<Vec<PullRequest>> {
        let state_param = match filters.state {
            PullRequestStateFilter::Open => "?state=OPEN",
            PullRequestStateFilter::Closed => "?state=DECLINED",
            PullRequestStateFilter::Merged => "?state=MERGED",
            PullRequestStateFilter::Locked | PullRequestStateFilter::All => "",
        };
        let prs: Vec<BitbucketPullRequest> =
            self.call_paginated(&self.get_repository_url(&format!("/pullrequests{state_param}")))?;

        Ok(prs.into_iter().map(|pr| pr.into()).collect())
    }
    #[instrument(skip(self))]
    fn approve_pr(&self, id: u32) -> Result<()> {
        let _: BitbucketApproval = self.call(
            Method::POST,
            &self.get_repository_url(&format!("/pullrequests/{id}/approve")),
            None as Option<i32>,
        )?;

        Ok(())
    }
    #[instrument(skip(self))]
    fn close_pr(&self, id: u32) -> Result<PullRequest> {
        let pr: BitbucketPullRequest = self.call(
            Method::POST,
            &self.get_repository_url(&format!("/pullrequests/{id}/decline")),
            None as Option<i32>,
        )?;

        Ok(pr.into())
    }
    #[instrument(skip(self))]
    fn merge_pr(&self, id: u32, close_source_branch: bool) -> Result<PullRequest> {
        let pr: BitbucketPullRequest = self.call(
            Method::POST,
            &self.get_repository_url(&format!("/pullrequests/{id}/merge")),
            Some(BitbucketMergePullRequest {
                close_source_branch,
            }),
        )?;

        Ok(pr.into())
    }
}
