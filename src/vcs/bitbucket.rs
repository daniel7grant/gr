use super::common::{CreatePullRequest, PullRequest, PullRequestState, User, VersionControl};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use color_eyre::{eyre::eyre, eyre::ContextCompat, Result};
use reqwest::{Client, Method};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct BitbucketUser {
    pub account_id: String,
    pub uuid: String,
    pub nickname: String,
    pub display_name: String,
}

impl Into<User> for BitbucketUser {
    fn into(self) -> User {
        let Self { nickname, .. } = self;
        User { username: nickname }
    }
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
    pub commit: Option<BitbucketPullRequestCommit>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketPullRequest {
    pub id: u32,
    pub state: PullRequestState,
    pub title: String,
    pub description: String,
    pub created_on: DateTime<Utc>,
    pub updated_on: DateTime<Utc>,
    pub source: BitbucketPullRequestRevision,
    pub destination: BitbucketPullRequestRevision,
    pub author: BitbucketUser,
    pub closed_by: Option<BitbucketUser>,
    pub reviewers: Option<Vec<BitbucketUser>>,
}

impl Into<PullRequest> for BitbucketPullRequest {
    fn into(self) -> PullRequest {
        let Self {
            id,
            state,
            title,
            description,
            source,
            destination,
            created_on,
            updated_on,
            author,
            closed_by,
            reviewers,
        } = self;
        PullRequest {
            id,
            state,
            title,
            description,
            source: source.branch.name,
            destination: destination.branch.name,
            created_on,
            updated_on,
            author: author.into(),
            closed_by: closed_by.map(|c| c.into()),
            reviewers: reviewers.map(|rs| rs.into_iter().map(|r| r.into()).collect()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketCreatePullRequest {
    pub title: String,
    pub description: String,
    pub source: BitbucketPullRequestRevision,
    pub destination: BitbucketPullRequestRevision,
    pub close_source_branch: bool,
}

impl From<CreatePullRequest> for BitbucketCreatePullRequest {
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
            source: BitbucketPullRequestRevision {
                branch: BitbucketPullRequestBranch { name: source },
                commit: None,
            },
            destination: BitbucketPullRequestRevision {
                branch: BitbucketPullRequestBranch { name: destination },
                commit: None,
            },
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
    auth: (String, String),
    client: Client,
    project: String,
    repo: String,
}

impl Bitbucket {
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
                    "https://api.bitbucket.org/2.0/repositories/{}/{}{}",
                    self.project, self.repo, url
                ),
            )
            .basic_auth(&self.auth.0, Some(&self.auth.1))
            .header("Content-Type", "application/json");
        if let Some(body) = body {
            request = request.json(&body);
        }
        let result = request.send().await?;

        let t: T = result.json().await?;
        Ok(t)
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

            if let None = page.next {
                break;
            }

            i += 1;
        }
        Ok(collected_values)
    }
}

#[async_trait]
impl VersionControl for Bitbucket {
    fn init(auth: (String, String), (project, repo): (String, String)) -> Self {
        let client = Client::new();
        Bitbucket {
            auth,
            client,
            project,
            repo,
        }
    }
    async fn create_pr(self, pr: CreatePullRequest) -> Result<PullRequest> {
        let new_pr: BitbucketPullRequest = self
            .call(
                Method::POST,
                "/pullrequests",
                Some(BitbucketCreatePullRequest::from(pr)),
            )
            .await?;

        Ok(new_pr.into())
    }
    async fn get_pr(self, branch: &str) -> Result<PullRequest> {
        let prs: Vec<BitbucketPullRequest> = self.call_paginated(&format!("/pullrequests")).await?;

        prs.into_iter()
            .find(|pr| pr.source.branch.name == branch)
            .map(|pr| pr.into())
            .wrap_err(eyre!("Pull request on branch {branch} not found."))
    }
}
