use crate::vcs::common::{PullRequest, PullRequestState};
use colored::Colorize;

pub enum FormatterType {
    Normal,
    Json,
}

pub trait Formatter {
    fn show_json(&self) -> String;
    fn show_normal(&self) -> String;
    fn show(&self, formatter_type: FormatterType) -> String {
        match formatter_type {
            FormatterType::Normal => self.show_normal(),
            FormatterType::Json => self.show_json(),
        }
    }
}

impl Formatter for PullRequest {
    fn show_json(&self) -> String {
        serde_json::to_string(&self).unwrap()
    }
    fn show_normal(&self) -> String {
        // On open false or failed browser opening, print the PR
        let max_width_title = if self.title.len() > 73 {
            format!("{}...", &self.title[0..70])
        } else {
            self.title.clone()
        };
        let colored_title = match self.state {
            PullRequestState::Open => max_width_title.bold(),
            PullRequestState::Closed => max_width_title.bold().red(),
            PullRequestState::Merged => max_width_title.bold().green(),
            PullRequestState::Locked => max_width_title.bold().magenta(),
        };
        let colored_id = format!("#{}", self.id).dimmed();
        let title_line = format!("{:<73} {:>6}", colored_title, colored_id);
        let details_line = format!(
            "{} {} {} {} {} {}",
            "opened by".dimmed(),
            self.author.username,
            "on".dimmed(),
            self.created_at.format("%Y-%m-%d"),
            "updated on".dimmed(),
            self.updated_at.format("%Y-%m-%d")
        );
        let branch_line = format!("{} -> {}", self.source.blue(), self.target.blue());
        let description = if self.description.len() > 0 {
            format!("{}\n---", self.description)
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
}
