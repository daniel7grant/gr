use dirs::config_dir;
use eyre::{eyre, Context, ContextCompat, Result};
use gr_bin::vcs::common::VersionControlSettings;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::read_to_string, fs::write};
use tracing::{info, instrument, trace};

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct RepositoryConfig {
    pub auth: Option<String>,
    pub default_branch: Option<String>,
    #[serde(default)]
    pub fork: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VcsConfig {
    #[serde(rename = "type")]
    pub vcs_type: Option<String>,
    pub auth: String,
    #[serde(default)]
    pub repositories: HashMap<String, RepositoryConfig>,
}

#[derive(Debug)]
pub struct Configuration {
    pub config_file_path: String,
    pub vcs: HashMap<String, VcsConfig>,
}

impl Configuration {
    #[instrument]
    pub fn get_default_config_file_path() -> Result<String> {
        let config_file_path = config_dir()
            .map(|dir| dir.join("gr.json"))
            .wrap_err("Configuration directory does not exist.")?;
        let config_file_path = config_file_path.into_os_string();
        let config_file_path = config_file_path
            .to_str()
            .wrap_err("Configuration filename cannot be found.")?;

        Ok(config_file_path.to_string())
    }

    #[instrument]
    pub fn parse(path: Option<String>) -> Result<Configuration> {
        let config_file_path = if let Some(path) = path {
            path
        } else {
            Configuration::get_default_config_file_path()?
        };
        info!("Configuration filename is {config_file_path}.");

        let config_content = read_to_string(&config_file_path).wrap_err(eyre!(
            "Configuration file {config_file_path} cannot be opened.",
        ));

        let vcs: HashMap<String, VcsConfig> = config_content
            .and_then(|content| {
                trace!(
                    "Configuration file content: {}.",
                    &content
                        .chars()
                        .filter(|c| !c.is_whitespace())
                        .collect::<String>()
                );
                serde_json::from_str(&content).wrap_err(eyre!(
                    "Configuration file {config_file_path} cannot be JSON parsed.",
                ))
            })
            .unwrap_or_default();

        Ok(Configuration {
            config_file_path,
            vcs,
        })
    }

    #[instrument]
    pub fn save(self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.vcs).wrap_err("Cannot serialize data.")?;
        trace!(
            "Configuration file to write: {:?}.",
            &content
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
        );
        write(&self.config_file_path, content).wrap_err(eyre!(
            "Cannot write to configuration file {}.",
            &self.config_file_path
        ))?;

        Ok(())
    }

    #[instrument]
    pub fn find_settings(&self, hostname: &str, repo: &str) -> Option<VersionControlSettings> {
        let vcs = self.vcs.get(hostname);
        vcs.map(|v| {
            let r = v.repositories.get(repo);

            VersionControlSettings {
                auth: r.and_then(|r| r.auth.clone()).unwrap_or(v.auth.clone()),
                default_branch: r.and_then(|r| r.default_branch.clone()),
                vcs_type: v.vcs_type.clone(),
                fork: r.map(|r| r.fork).unwrap_or_default(),
            }
        })
    }
}
