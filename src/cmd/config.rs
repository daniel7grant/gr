use color_eyre::{
    eyre::{Context, ContextCompat},
    Result,
};
use dirs::config_dir;
use serde::Deserialize;
use std::{collections::HashMap, fs::read_to_string};

#[derive(Debug, Deserialize)]
pub struct RepositoryConfig {
    pub auth: String,
}

#[derive(Debug, Deserialize)]
pub struct VcsConfig {
    #[serde(rename = "type")]
    pub vcs_type: Option<String>,
    pub auth: String,
    #[serde(default)]
    pub repositories: HashMap<String, RepositoryConfig>,
}

#[derive(Debug)]
pub struct Configuration {
    pub vcs: HashMap<String, VcsConfig>,
}

impl Configuration {
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

    pub fn parse() -> Result<Configuration> {
        let config_file_path = Configuration::get_default_config_file_path()?;
        let config_content = read_to_string(&config_file_path).wrap_err("");

        let vcs: HashMap<String, VcsConfig> = config_content
            .and_then(|content| serde_json::from_str(&content).wrap_err(""))
            .unwrap_or_default();

        Ok(Configuration { vcs })
    }

    pub fn find_type(&self, hostname: &str) -> Option<String> {
        let vcs = self.vcs.get(hostname);
        vcs.and_then(|v| v.vcs_type.clone())
    }

    pub fn find_auth(&self, hostname: &str, repo: &str) -> Option<String> {
        let vcs = self.vcs.get(hostname);
        vcs.map(|v| {
            v.repositories
                .get(repo)
                .map_or(v.auth.clone(), |r| r.auth.clone())
        })
    }
}
