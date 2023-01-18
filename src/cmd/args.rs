use clap::{Command, CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Generator, Shell};
use std::io;
use std::process;

#[derive(Debug, Subcommand)]
pub enum PrCommands {
    /// Create pull request for the current branch
    Create {
        /// The title of the pull request
        #[arg(short, long)]
        message: String,
        /// The description of the pull request (default: the list of commits)
        #[arg(short, long)]
        description: Option<String>,
        /// Change the source branch (default: the current branch)
        #[arg(long)]
        branch: Option<String>,
        /// Change the target branch (default: the default branch in the repo)
        #[arg(long)]
        target: Option<String>,
        /// Close source branch after merging
        #[arg(long)]
        close: bool,
        /// Open the pull request in the browser
        #[arg(long)]
        open: bool,
    },
    /// Get the open pull request for the current branch
    Get {
        /// Change the source branch (default: the current branch)
        #[arg(long)]
        branch: Option<String>,
        /// Open the pull request in the browser
        #[arg(long)]
        open: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Interact with pull requests
    #[command(subcommand)]
    Pr(PrCommands),
    Completion {
        shell: Shell,
    },
}

#[derive(Debug, Parser)]
#[command(name = "gr")]
#[command(about = "Interact with remote repositories like you interact with git", long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

impl Cli {
    pub fn parse_args() -> Cli {
        let cli = Cli::parse();

        if let Commands::Completion { shell } = cli.command {
            let mut cmd = Cli::command();
            print_completions(shell, &mut cmd);
            process::exit(0);
        }

        cli
    }
}
