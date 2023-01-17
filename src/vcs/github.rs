use super::common::{PullRequest, VersionControl};
use async_trait::async_trait;
use color_eyre::Result;
use reqwest::Client;

pub struct GitHub {
    auth: (String, String),
    client: Client,
    project: String,
    repo: String,
}

#[async_trait]
impl VersionControl for GitHub {
    fn init(auth: (String, String), (project, repo): (String, String)) -> Self {
        let client = Client::new();
        GitHub { auth, client, project, repo }
    }
    async fn create_pr(self) -> Result<PullRequest> {
        unimplemented!();
    }
    async fn get_pr(self, branch: &str) -> Result<PullRequest> {
        unimplemented!();
    }
}
