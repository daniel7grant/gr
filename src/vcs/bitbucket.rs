// Documentation: https://developer.atlassian.com/cloud/bitbucket/rest/intro/
use super::common::{
    CreatePullRequest, ListPullRequestFilters, PullRequest, PullRequestState,
    PullRequestStateFilter, User, VersionControl, VersionControlSettings,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use color_eyre::{eyre::eyre, eyre::ContextCompat, Result};
use reqwest::{Client, Method};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fmt::Debug;

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
pub struct BitbucketCreatePullRequest {
    pub title: String,
    pub description: String,
    pub source: BitbucketPullRequestRevision,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<BitbucketPullRequestRevision>,
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
            ..
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

pub struct Bitbucket {
    settings: VersionControlSettings,
    client: Client,
    repo: String,
}

impl Bitbucket {
    async fn call<T: DeserializeOwned, U: Serialize + Debug>(
        &self,
        method: Method,
        url: &str,
        body: Option<U>,
    ) -> Result<T> {
        let (username, password) = self
            .settings
            .auth
            .split_once(':')
            .wrap_err("Authentication has to contain a username and a token.")?;
        let mut request = self
            .client
            .request(
                method,
                format!(
                    "https://api.bitbucket.org/2.0/repositories/{}{}",
                    self.repo, url
                ),
            )
            .basic_auth(username, Some(password));
        if let Some(body) = &body {
            request = request.json(body);
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

    async fn call_paginated<T: DeserializeOwned>(&self, url: &str) -> Result<Vec<T>> {
        let mut collected_values: Vec<T> = vec![];
        let mut i = 1;
        loop {
            let mut page: BitbucketPaginated<T> = self
                .call(
                    Method::GET,
                    &format!("/{url}?page={i}"),
                    None as Option<i32>,
                )
                .await?;

            collected_values.append(&mut page.values);

            if page.next.is_none() {
                break;
            }

            i += 1;
        }
        Ok(collected_values)
    }
}

#[async_trait]
impl VersionControl for Bitbucket {
    fn init(_: String, repo: String, settings: VersionControlSettings) -> Self {
        let client = Client::new();
        Bitbucket {
            settings,
            client,
            repo,
        }
    }
    async fn get_user_by_name(&self, _: &str) -> Result<User> {
        // TODO: figure out a way to get an arbitrary user in BitBucket
        // maybe from the workspaces API?
        Err(eyre!("Reviewers feature is not implemented for Bitbucket."))
    }
    async fn create_pr(&self, pr: CreatePullRequest) -> Result<PullRequest> {
        let new_pr: BitbucketPullRequest = self
            .call(
                Method::POST,
                "/pullrequests",
                Some(BitbucketCreatePullRequest::from(pr)),
            )
            .await?;

        Ok(new_pr.into())
    }
    async fn get_pr_by_branch(&self, branch: &str) -> Result<PullRequest> {
        let prs: Vec<BitbucketPullRequest> = self.call_paginated("/pullrequests").await?;

        prs.into_iter()
            .find(|pr| pr.source.branch.name == branch)
            .map(|pr| pr.into())
            .wrap_err(eyre!("Pull request on branch {branch} not found."))
    }
    async fn list_prs(&self, filters: ListPullRequestFilters) -> Result<Vec<PullRequest>> {
        let state_param = match filters.state {
            PullRequestStateFilter::Open => "?state=OPEN",
            PullRequestStateFilter::Closed => "?state=DECLINED",
            PullRequestStateFilter::Merged => "?state=MERGED",
            PullRequestStateFilter::Locked | PullRequestStateFilter::All => "",
        };
        let prs: Vec<BitbucketPullRequest> = self
            .call_paginated(&format!("/pullrequests{state_param}"))
            .await?;

        Ok(prs.into_iter().map(|pr| pr.into()).collect())
    }
    async fn approve_pr(&self, id: u32) -> Result<()> {
        let _: BitbucketApproval = self
            .call(
                Method::POST,
                &format!("/pullrequests/{id}/approve"),
                None as Option<i32>,
            )
            .await?;

        Ok(())
    }
    async fn close_pr(&self, id: u32) -> Result<PullRequest> {
        let pr: BitbucketPullRequest = self
            .call(
                Method::POST,
                &format!("/pullrequests/{id}/decline"),
                None as Option<i32>,
            )
            .await?;

        Ok(pr.into())
    }
    async fn merge_pr(&self, id: u32) -> Result<()> {
        let _: BitbucketPullRequest = self
            .call(
                Method::POST,
                &format!("/pullrequests/{id}/merge"),
                None as Option<i32>,
            )
            .await?;

        Ok(())
    }
}
