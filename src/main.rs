mod cmd;
mod utils;

use cmd::{
    args::{Cli, Commands, PrCommands},
    config::Configuration,
    login::login::login,
    pr::{approve::approve, close::close, create::create, get::get, list::list, merge::merge},
};
use color_eyre::eyre::eyre;
use color_eyre::Result;
use std::process;
use utils::tracing::init_tracing;

async fn run(mut args: Cli) -> Result<()> {
    let conf = Configuration::parse()?;

    match args.command {
        Commands::Login { .. } => login(args, conf).await,
        Commands::Pr(PrCommands::Create { .. }) => create(args, conf).await,
        Commands::Pr(PrCommands::Get { .. }) => get(args, conf).await,
        Commands::Pr(PrCommands::Open { .. }) => {
            args.command = Commands::Pr(PrCommands::Get { open: true });
            get(args, conf).await
        }
        Commands::Pr(PrCommands::List { .. }) => list(args, conf).await,
        Commands::Pr(PrCommands::Approve { .. }) => approve(args, conf).await,
        Commands::Pr(PrCommands::Merge { .. }) => merge(args, conf).await,
        Commands::Pr(PrCommands::Close { .. }) => close(args, conf).await,
        Commands::Completion { .. } => Err(eyre!("Invalid command.")),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse_args();
    let verbose = args.verbose;

    init_tracing(verbose)?;

    match run(args).await {
        Ok(_) => Ok(()),
        Err(err) => {
            if verbose == 0 {
                println!("Error: {}", err);
                process::exit(1)
            } else {
                Err(err)
            }
        }
    }
}
