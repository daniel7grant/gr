use super::utils::to_fixed_length;
use crate::vcs::common::{PullRequest, PullRequestState, Repository};
use colored::Colorize;

pub enum FormatterType {
    Json,
    Normal,
    Short,
}

pub trait Formatter {
    fn show_json(&self) -> String;
    fn show_normal(&self) -> String;
    fn show_short(&self) -> String;
    fn show(&self, formatter_type: FormatterType) -> String {
        match formatter_type {
            FormatterType::Json => self.show_json(),
            FormatterType::Normal => self.show_normal(),
            FormatterType::Short => self.show_short(),
        }
    }
}

const FULL_SIZE: usize = 80;
const ID_SIZE: usize = 6;
const TITLE_SIZE: usize = FULL_SIZE - ID_SIZE - 1;
const SHORT_BRANCH_SIZE: usize = 20;
const SHORT_TITLE_SIZE: usize = FULL_SIZE - ID_SIZE - 1 - SHORT_BRANCH_SIZE - 1;

impl Formatter for PullRequest {
    fn show_json(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
    fn show_normal(&self) -> String {
        let title = to_fixed_length(&self.title, TITLE_SIZE, true);
        let title = match self.state {
            PullRequestState::Open => title.bold(),
            PullRequestState::Closed => title.bold().red(),
            PullRequestState::Merged => title.bold().green(),
            PullRequestState::Locked => title.bold().magenta(),
        };
        let colored_id = format!("#{}", self.id).dimmed();
        let title_line = format!("{} {:>width$}", title, colored_id, width = ID_SIZE);
        let details_line = format!(
            "{} {} {} {} {} {}",
            "opened by".dimmed(),
            self.author.username,
            "on".dimmed(),
            self.created_at.date(),
            "updated on".dimmed(),
            self.updated_at.date(),
        );
        let branch_line = format!("{} -> {}", self.source.blue(), self.target.blue());
        let description = if !self.description.is_empty() {
            format!("\n{}\n---", self.description)
        } else {
            "".to_string()
        };
        let url_line = format!("{}", self.url.dimmed());

        format!(
            "{title_line}
{details_line}
{branch_line}
{description}
{url_line}
"
        )
    }
    fn show_short(&self) -> String {
        let title = to_fixed_length(&self.title, SHORT_TITLE_SIZE, true);
        let title = match self.state {
            PullRequestState::Open => title.bold(),
            PullRequestState::Closed => title.bold().red(),
            PullRequestState::Merged => title.bold().green(),
            PullRequestState::Locked => title.bold().magenta(),
        };
        let branch = to_fixed_length(&self.source, SHORT_BRANCH_SIZE, true).blue();
        let colored_id = format!("#{}", self.id).dimmed();
        format!("{} {} {:>6}\n", title, branch, colored_id)
    }
}

impl Formatter for Repository {
    fn show_json(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
    fn show_normal(&self) -> String {
        let title_line = to_fixed_length(&self.full_name, TITLE_SIZE, true).bold();
        let forked_line = if let Some(ref forked) = self.forked_from {
            format!("{} {}\n", "forked from".dimmed(), forked.full_name)
        } else {
            String::new()
        };
        let counts_line = format!(
            "{} stars, {} forks",
            self.stars_count.to_string().yellow(),
            self.forks_count.to_string().yellow(),
        );
        let description = if !self.description.is_empty() {
            format!("\n{}\n---", self.description)
        } else {
            "".to_string()
        };
        let url_line = format!("{}", self.html_url.dimmed());

        format!(
            "{title_line}
{forked_line}{counts_line}
{description}
{url_line}
"
        )
    }
    fn show_short(&self) -> String {
        let title = to_fixed_length(&self.full_name, SHORT_TITLE_SIZE, true);
        format!("{}\n", title)
    }
}
