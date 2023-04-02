use eyre::Result;
use open::that as open_in_browser;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

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
    #[serde(with = "time::serde::iso8601")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    pub updated_at: OffsetDateTime,
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

#[derive(Debug, Deserialize, Serialize)]
pub struct Repository {
    pub name: String,
    pub full_name: String,
    pub owner: Option<User>,
    pub html_url: String,
    pub description: String,
    #[serde(with = "time::serde::iso8601")]
    pub created_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    pub updated_at: OffsetDateTime,
    pub private: bool,
    pub archived: bool,
    pub default_branch: String,
    pub forks_count: u32,
    pub stars_count: u32,
}

impl Repository {
    pub fn print(&self, in_browser: bool, formatter_type: FormatterType) {
        // Open in browser if open is true
        if in_browser && open_in_browser(&self.html_url).is_ok() {
            return;
        }
        print!("{}", self.show(formatter_type));
    }
}

#[derive(Debug, Default)]
pub struct VersionControlSettings {
    pub auth: String,
    pub vcs_type: Option<String>,
    pub default_branch: Option<String>,
}
pub trait VersionControl {
    fn init(hostname: String, repo: String, settings: VersionControlSettings) -> Self
    where
        Self: Sized;

    // Login
    fn login_url(&self) -> String;
    fn validate_token(&self, token: &str) -> Result<()>;

    // Pull requests
    fn create_pr(&self, pr: CreatePullRequest) -> Result<PullRequest>;
    fn get_pr_by_id(&self, id: u32) -> Result<PullRequest>;
    fn get_pr_by_branch(&self, branch: &str) -> Result<PullRequest>;
    fn list_prs(&self, filters: ListPullRequestFilters) -> Result<Vec<PullRequest>>;
    fn approve_pr(&self, id: u32) -> Result<()>;
    fn close_pr(&self, id: u32) -> Result<PullRequest>;
    fn merge_pr(&self, id: u32, delete_source_branch: bool) -> Result<PullRequest>;

    // Repositories
    fn get_repository(&self) -> Result<Repository>;
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
