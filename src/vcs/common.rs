use async_trait::async_trait;
use chrono::{DateTime, Utc};
use color_eyre::Result;
use open::that as open_in_browser;
use serde::{Deserialize, Serialize};

use crate::formatters::formatter::{Formatter, FormatterType};
use crate::vcs::{bitbucket::Bitbucket, github::GitHub, gitlab::GitLab};

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct User {
    pub id: String,
    pub username: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum PullRequestState {
    Open,
    Closed,
    Merged,
    Locked,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PullRequest {
    pub id: u32,
    pub state: PullRequestState,
    pub title: String,
    pub description: String,
    pub source: String,
    pub target: String,
    pub url: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub author: User,
    pub closed_by: Option<User>,
    pub reviewers: Option<Vec<User>>,
}

impl PullRequest {
    pub fn print(&self, in_browser: bool, formatter_type: FormatterType) {
        // Open in browser if open is true
        if in_browser && open_in_browser(&self.url).is_ok() {
            return;
        }
        print!("{}", self.show(formatter_type));
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreatePullRequest {
    pub title: String,
    pub description: String,
    pub source: String,
    pub target: Option<String>,
    pub close_source_branch: bool,
    pub reviewers: Vec<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub enum PullRequestUserFilter {
    Me,
    #[default]
    All,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub enum PullRequestStateFilter {
    #[default]
    Open,
    Closed,
    Merged,
    Locked,
    All,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ListPullRequestFilters {
    pub author: PullRequestUserFilter,
    pub state: PullRequestStateFilter,
}

#[derive(Debug, Default)]
pub struct VersionControlSettings {
    pub auth: String,
    pub vcs_type: Option<String>,
    pub default_branch: Option<String>,
}
#[async_trait]
pub trait VersionControl {
    fn init(hostname: String, repo: String, settings: VersionControlSettings) -> Self
    where
        Self: Sized;
    fn login_url(&self) -> String;
    fn validate_token(&self, token: &str) -> Result<()>;
    async fn create_pr(&self, pr: CreatePullRequest) -> Result<PullRequest>;
    async fn get_pr_by_id(&self, id: u32) -> Result<PullRequest>;
    async fn get_pr_by_branch(&self, branch: &str) -> Result<PullRequest>;
    async fn list_prs(&self, filters: ListPullRequestFilters) -> Result<Vec<PullRequest>>;
    async fn approve_pr(&self, id: u32) -> Result<()>;
    async fn close_pr(&self, id: u32) -> Result<PullRequest>;
    async fn merge_pr(&self, id: u32, delete_source_branch: bool) -> Result<PullRequest>;
}

pub fn init_vcs(
    hostname: String,
    repo: String,
    settings: VersionControlSettings,
) -> Box<dyn VersionControl> {
    match (hostname.as_str(), &settings.vcs_type) {
        ("github.com", _) => Box::new(GitHub::init(hostname, repo, settings)),
        ("bitbucket.org", _) => Box::new(Bitbucket::init(hostname, repo, settings)),
        (_, _) => Box::new(GitLab::init(hostname, repo, settings)),
    }
}
