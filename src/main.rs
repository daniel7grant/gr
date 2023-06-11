mod cmd;
mod utils;

use cmd::{
    args::{Cli, Commands, PrCommands, RepoCommands},
    config::Configuration,
    login::login::login,
    pr::{
        approve::approve, close::close, create::create, get::get as get_pr, list::list,
        merge::merge,
    },
    repo::{delete::delete, fork::fork, get::get as get_repo, new::new},
};
use eyre::{eyre, Result};
use std::process;
use tracing::error;
use utils::tracing::init_tracing;

fn run(mut args: Cli) -> Result<()> {
    let conf = Configuration::parse(args.config.clone())?;

    match args.command {
        Commands::Login { .. } => login(args, conf),
        Commands::Pr(PrCommands::Create { .. }) => create(args, conf),
        Commands::Pr(PrCommands::Get { .. }) => get_pr(args, conf),
        Commands::Pr(PrCommands::Open { .. }) => {
            args.command = Commands::Pr(PrCommands::Get { open: true });
            get_pr(args, conf)
        }
        Commands::Pr(PrCommands::List { .. }) => list(args, conf),
        Commands::Pr(PrCommands::Approve { .. }) => approve(args, conf),
        Commands::Pr(PrCommands::Merge { .. }) => merge(args, conf),
        Commands::Pr(PrCommands::Close { .. }) => close(args, conf),
        Commands::Repo(RepoCommands::New { .. }) => new(args, conf),
        Commands::Repo(RepoCommands::Fork { .. }) => fork(args, conf),
        Commands::Repo(RepoCommands::Get { .. }) => get_repo(args, conf),
        Commands::Repo(RepoCommands::Open { .. }) => {
            args.command = Commands::Repo(RepoCommands::Get { open: true });
            get_repo(args, conf)
        }
        Commands::Repo(RepoCommands::Delete { .. }) => delete(args, conf),
        Commands::Completion { .. } => Err(eyre!("Invalid command.")),
    }
}

fn main() -> Result<()> {
    let args = Cli::parse_args();
    let Cli {
        output, verbose, ..
    } = args;

    init_tracing(verbose, output)?;

    match run(args) {
        Ok(_) => Ok(()),
        Err(err) => {
            match verbose {
                0 => error!("{}", err),
                _ => {
                    for err in err.chain() {
                        error!("{}", err);
                    }
                }
            }
            process::exit(1)
        }
    }
}
