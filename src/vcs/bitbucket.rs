use super::common::{Paginated, PullRequest, VersionControl};
use async_trait::async_trait;
use color_eyre::{
    eyre::{eyre, ContextCompat},
    Result,
};
use reqwest::{Client, Method};
use serde::de::DeserializeOwned;

pub struct Bitbucket {
    auth: (String, String),
    client: Client,
    project: String,
    repo: String,
}

impl Bitbucket {
    async fn call<T: DeserializeOwned>(&self, method: Method, url: &str) -> Result<T> {
        let result = self
            .client
            .request(
                method,
                format!(
                    "https://api.bitbucket.org/2.0/repositories/{}/{}{}",
                    self.project, self.repo, url
                ),
            )
            .basic_auth(&self.auth.0, Some(&self.auth.1))
            .header("Content-Type", "application/json")
            .send()
            .await?;

        let t: T = result.json().await?;
        Ok(t)
    }

    async fn call_paginated<T: DeserializeOwned>(&self, url: &str) -> Result<Vec<T>> {
        let mut collected_values: Vec<T> = vec![];
        let mut i = 1;
        loop {
            let mut page: Paginated<T> =
                self.call(Method::GET, &format!("/{url}?page={i}")).await?;

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
    async fn create_pr(self) -> Result<PullRequest> {
        unimplemented!();
    }
    async fn get_pr(self, branch: &str) -> Result<PullRequest> {
        let prs: Vec<PullRequest> = self.call_paginated(&format!("/pullrequests")).await?;

        prs.into_iter()
            .find(|pr| pr.source.branch.name == branch)
            .wrap_err(eyre!("Pull request on branch {branch} not found."))
    }
}
