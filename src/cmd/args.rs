use clap::{ArgAction, Command, CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Generator, Shell};
use gr_bin::formatters::formatter::FormatterType;
use std::io;
use std::process;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum StateFilter {
    /// Show only open pull requests (default)
    Open,
    /// Show only closed pull requests
    Closed,
    /// Show only merged pull requests (GitLab and Bitbucket only)
    Merged,
    /// Show only locked pull requests (GitLab only)
    Locked,
    /// Show all pull requests
    All,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum UserFilter {
    /// Show only pull requests by me (GitLab only)
    Me,
    /// Show all pull requests
    All,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Default)]
pub enum OutputType {
    /// Print output in a human-readable way (default)
    #[default]
    Normal,
    /// Print output and logs as JSON
    Json,
}

impl Into<FormatterType> for OutputType  {
    fn into(self) -> FormatterType {
        match self {
            OutputType::Normal => FormatterType::Normal,
            OutputType::Json => FormatterType::Json,
        }
    }
}

#[derive(Debug, Subcommand)]
#[command(after_help = "Examples:

Create pull request on current branch:
$ gr pr create -m 'PR title'

Get information about the current branch PR:
$ gr pr get

Merge the current branch PR:
$ gr pr merge
")]
pub enum PrCommands {
    #[command(after_help = "Examples:

Create a pull request with a title:
$ gr pr create -m 'Do things'

Create a pull request with a title and a description:
$ gr pr create -m 'Do things' -d 'Do things and stuff'

Description can be provided from standard input (for example git-cliff):
$ git-cliff --tag PR --strip all | gr pr create -m 'Do things'

Create a pull request to merge into a different branch:
$ gr pr create -m 'Do things' --target staging

Create a pull request and merge it immediately (for fix branches):
$ gr pr create -m 'Fix things' --merge --delete")]
    /// Create pull request for the current branch
    /// 
    /// The only required field is the title (--message / -m), other fields will be filled by sane defaults:
    /// the description will be the list of commits, the target branch is the default branch.
    Create {
        /// The title of the pull request
        #[arg(short, long)]
        message: String,
        /// The description of the pull request (default: stdin, or the list of commits)
        #[arg(short, long)]
        description: Option<String>,
        /// Change the target branch (default: the default branch in the repo)
        #[arg(short, long)]
        target: Option<String>,
        /// Add reviewers by their username (can be added multiple times)
        #[arg(short, long = "reviewer")]
        reviewers: Option<Vec<String>>,
        /// Delete source branch after merging (Gitlab and Bitbucket only)
        #[arg(long)]
        delete: bool,
        /// Open the pull request in the browser
        #[arg(long)]
        open: bool,
        /// Merge the pull request instantly (good for hotfixes)
        #[arg(long)]
        merge: bool,
    },
    #[command(after_help = "Examples:

Get the pull request on the current branch:
$ gr pr get

Get the pull request on another branch:
$ gr pr get -b feature/branch")]
    /// Get the open pull request for the current branch
    Get {
        /// Open the pull request in the browser
        #[arg(long)]
        open: bool,
    },
    #[command(after_help = "Examples:

Open the pull request on the current branch:
$ gr pr open

Open the pull request on another branch:
$ gr pr open -b feature/branch")]
    /// Open the pull request in the browser
    Open {},
    #[command(after_help = "Examples:

List all open pull requests:
$ gr pr list

List all pull requests:
$ gr pr list --state=all

List your open pull requests:
$ gr pr list --user=me")]
    /// List pull requests for the current repo
    List {
        /// Filter by PR author
        #[arg(long, value_enum)]
        author: Option<UserFilter>,
        /// Filter by PR state
        #[arg(long, value_enum)]
        state: Option<StateFilter>,
    },
    #[command(after_help = "Examples:

Approve the pull request on the current branch:
$ gr pr approve")]
    /// Approve the pull request for the current branch
    Approve {},
    #[command(after_help = "Examples:

Merge the pull request, and go to the target branch:
$ gr pr merge")]
    /// Merge the pull request for the current branch
    /// 
    /// This operation will change the branches locally to the target branch and pull the merged changes.
    Merge {
        /// Delete remote and local branch after merging (remote is Gitlab and Bitbucket only)
        #[arg(long)]
        delete: bool,
    },
    #[command(after_help = "Examples:

Decline the pull request:
$ gr pr decline")]
    /// Close (decline) the pull request for the current branch
    #[command(alias = "decline")]
    Close {},
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(after_help = "Examples:

Login to the current repo's remote:
$ gr login

Login to arbitrary remote:
$ gr login github.com")]
    /// Login to a remote with a token
    Login {
        /// The host to login to (e.g. github.com, default: current repo)
        hostname: Option<String>,
        /// The repo which the authentication should only appeal
        #[arg(long)]
        repo: Option<String>,
        /// Use this token to authenticate, instead of interactive login
        #[arg(long)]
        token: Option<String>,
    },
    /// Interact with pull requests
    #[command(subcommand)]
    Pr(PrCommands),
    /// Generate tab completion to shell
    Completion { shell: Shell },
}

#[derive(Debug, Parser)]
#[command(name = "gr")]
#[command(version = "0.1.2")]
#[command(about = "Interact with remote repositories like you interact with git", long_about = None)]
#[command(after_help = "Examples:

Login to the current repo's remote:
$ gr login

Create pull request on current branch:
$ gr pr create -m 'PR title'

Get information about the current branch PR:
$ gr pr get

Generate bash completion:
$ source <(gr completion bash)")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
    /// Change the source branch (default: the current branch)
    #[arg(short, long, global = true)]
    pub branch: Option<String>,
    /// Change the repo directory (default: the current directory)
    #[arg(long, global = true)]
    pub dir: Option<String>,
    /// Change the authentication token (default: find in configuration)
    #[arg(long, global = true)]
    pub auth: Option<String>,
    /// Output type
    #[arg(long, short, global = true, default_value = "normal")]
    pub output: OutputType,
    /// Print logging information (-v info, -vv debug, -vvv trace)
    #[arg(long, short, global = true, action = ArgAction::Count)]
    pub verbose: u8,
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
