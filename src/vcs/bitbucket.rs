// Documentation: https://developer.atlassian.com/cloud/bitbucket/rest/intro/
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

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct BitbucketUser {
    pub uuid: String,
    pub nickname: String,
    pub display_name: String,
}

impl From<BitbucketUser> for User {
    fn from(user: BitbucketUser) -> User {
        let BitbucketUser { uuid, nickname, .. } = user;
        User {
            id: uuid,
            username: nickname,
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct BitbucketTeam {
    pub uuid: String,
    pub username: String,
    pub display_name: String,
}

impl From<BitbucketTeam> for User {
    fn from(user: BitbucketTeam) -> User {
        let BitbucketTeam { uuid, username, .. } = user;
        User { id: uuid, username }
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

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketCloneLink {
    pub name: String,
    pub href: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketLink {
    pub href: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketForkedFromRepositoryLinks {
    pub html: BitbucketLink,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketRepositoryLinks {
    pub html: BitbucketLink,
    pub clone: Vec<BitbucketCloneLink>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketPullRequestLinks {
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
pub struct BitbucketRevisionRepository {
    pub full_name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketRevision {
    pub branch: BitbucketBranch,
    pub commit: BitbucketCommit,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<BitbucketRevisionRepository>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketCreateRevision {
    pub branch: BitbucketBranch,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<BitbucketRevisionRepository>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketRepositoryProject {
    uuid: String,
    key: String,
    name: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketForkedFromRepository {
    uuid: String,
    name: String,
    full_name: String,
    links: BitbucketForkedFromRepositoryLinks,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketRepository {
    uuid: String,
    name: String,
    full_name: String,
    links: BitbucketRepositoryLinks,
    owner: BitbucketTeam,
    description: String,
    #[serde(with = "time::serde::iso8601")]
    created_on: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    updated_on: OffsetDateTime,
    language: String,
    project: BitbucketRepositoryProject,
    mainbranch: BitbucketBranch,
    is_private: bool,
    parent: Option<BitbucketForkedFromRepository>,
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
            parent,
            ..
        } = repo;
        let ssh_url = links
            .clone
            .iter()
            .find(|BitbucketCloneLink { name, .. }| name == "ssh")
            .map(|BitbucketCloneLink { href, .. }| href);
        let https_url = links
            .clone
            .iter()
            .find(|BitbucketCloneLink { name, .. }| name == "https")
            .map(|BitbucketCloneLink { href, .. }| href);
        Repository {
            name,
            full_name,
            owner: Some(owner.into()),
            html_url: links.html.href,
            description,
            created_at: created_on,
            updated_at: updated_on,
            visibility: if is_private {
                RepositoryVisibility::Private
            } else {
                RepositoryVisibility::Public
            },
            archived: false,
            default_branch: mainbranch.name,
            forks_count: 0,
            stars_count: 0,
            ssh_url: ssh_url.unwrap().to_owned(),
            https_url: https_url.unwrap().to_owned(),
            forked_from: parent.map(|r| r.into()),
        }
    }
}

impl From<BitbucketForkedFromRepository> for ForkedFromRepository {
    fn from(repo: BitbucketForkedFromRepository) -> ForkedFromRepository {
        let BitbucketForkedFromRepository {
            name,
            links,
            full_name,
            ..
        } = repo;
        ForkedFromRepository {
            name,
            full_name,
            html_url: links.html.href,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct BitbucketCreateRepository {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    is_private: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct BitbucketForkRepositoryWorkspace {
    slug: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct BitbucketForkRepository {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace: Option<BitbucketForkRepositoryWorkspace>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BitbucketPullRequest {
    pub id: u32,
    pub state: BitbucketPullRequestState,
    pub title: String,
    pub description: String,
    pub links: BitbucketPullRequestLinks,
    #[serde(with = "time::serde::iso8601")]
    pub created_on: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    pub updated_on: OffsetDateTime,
    pub source: BitbucketRevision,
    pub destination: BitbucketRevision,
    pub author: BitbucketUser,
    pub closed_by: Option<BitbucketUser>,
    pub reviewers: Option<Vec<BitbucketUser>>,
    pub close_source_branch: bool,
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
            close_source_branch,
        } = pr;
        PullRequest {
            id,
            state: state.into(),
            title,
            description,
            source: source.branch.name,
            source_sha: source.commit.hash,
            target: destination.branch.name,
            target_sha: destination.commit.hash,
            url: links.html.href,
            created_at: created_on,
            updated_at: updated_on,
            author: author.into(),
            closed_by: closed_by.map(|u| u.into()),
            reviewers: reviewers.map(|rs| rs.into_iter().map(|r| r.into()).collect()),
            delete_source_branch: close_source_branch,
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
    pub source: BitbucketCreateRevision,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<BitbucketCreateRevision>,
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
            ..
        } = pr;
        Self {
            title,
            description,
            source: BitbucketCreateRevision {
                branch: BitbucketBranch { name: source },
                repository: None,
            },
            destination: destination.map(|name| BitbucketCreateRevision {
                branch: BitbucketBranch { name },
                repository: None,
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

        let mut url = self.get_repository_url("/pullrequests");
        let mut bitbucket_pr = BitbucketCreatePullRequest::from(pr);
        if self.settings.fork {
            let repo = self.get_repository()?;
            if let Some(forked) = repo.forked_from {
                url = format!("/repositories/{}/pullrequests", forked.full_name);
                bitbucket_pr.source = BitbucketCreateRevision {
                    repository: Some(BitbucketRevisionRepository {
                        full_name: repo.full_name,
                    }),
                    ..bitbucket_pr.source
                }
            }
        }

        let new_pr: BitbucketPullRequest = self.call("POST", &url, Some(bitbucket_pr))?;

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

    #[instrument(skip_all)]
    fn create_repository(&self, repo: CreateRepository) -> Result<Repository> {
        // TODO: make it work with user
        let CreateRepository {
            name,
            organization,
            visibility,
            description,
            ..
        } = repo;
        let (user, _) = self
            .settings
            .auth
            .split_once(':')
            .wrap_err("Authentication format is invalid")?;
        let workspace = organization.unwrap_or(user.to_string());
        let create_repo: BitbucketCreateRepository = BitbucketCreateRepository {
            name: name.clone(),
            description,
            is_private: visibility != RepositoryVisibility::Public,
        };
        let new_repo: BitbucketRepository = self.call(
            "POST",
            &format!("/repositories/{workspace}/{}", name),
            Some(create_repo),
        )?;

        Ok(new_repo.into())
    }

    #[instrument(skip_all)]
    fn fork_repository(&self, repo: ForkRepository) -> Result<Repository> {
        let ForkRepository { name, organization } = repo;
        let workspace = organization.map(|slug| BitbucketForkRepositoryWorkspace { slug });

        let new_repo: BitbucketRepository = self.call(
            "POST",
            &self.get_repository_url("/forks"),
            Some(BitbucketForkRepository { name, workspace }),
        )?;

        Ok(new_repo.into())
    }

    #[instrument(skip_all)]
    fn delete_repository(&self) -> Result<()> {
        self.call("DELETE", &self.get_repository_url(""), None as Option<i32>)?;

        Ok(())
    }
}
