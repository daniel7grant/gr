mod cmd;
mod utils;

use cmd::{
    args::{Cli, Commands, OutputType, PrCommands},
    config::Configuration,
    login::login::login,
    pr::{approve::approve, close::close, create::create, get::get, list::list, merge::merge},
};
use color_eyre::eyre::eyre;
use color_eyre::Result;
use std::process;
use tracing::error;
use utils::tracing::init_tracing;

fn run(mut args: Cli) -> Result<()> {
    let conf = Configuration::parse()?;

    match args.command {
        Commands::Login { .. } => login(args, conf),
        Commands::Pr(PrCommands::Create { .. }) => create(args, conf),
        Commands::Pr(PrCommands::Get { .. }) => get(args, conf),
        Commands::Pr(PrCommands::Open { .. }) => {
            args.command = Commands::Pr(PrCommands::Get { open: true });
            get(args, conf)
        }
        Commands::Pr(PrCommands::List { .. }) => list(args, conf),
        Commands::Pr(PrCommands::Approve { .. }) => approve(args, conf),
        Commands::Pr(PrCommands::Merge { .. }) => merge(args, conf),
        Commands::Pr(PrCommands::Close { .. }) => close(args, conf),
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
        Err(err) => match output {
            _ => {
                if verbose == 0 || output == OutputType::Json {
                    error!("{}", err.to_string());
                    process::exit(1)
                } else {
                    Err(err)
                }
            }
        },
    }
}
