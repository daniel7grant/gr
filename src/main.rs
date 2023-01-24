mod cmd;

use cmd::{
    args::{Cli, Commands, PrCommands},
    config::Configuration,
    pr::{create::create, get::get},
};
use color_eyre::eyre::eyre;
use color_eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Cli::parse_args();
    let conf = Configuration::parse()?;

    match args.command {
        Commands::Pr(PrCommands::Create { .. }) => create(args.command, conf).await,
        Commands::Pr(PrCommands::Get { .. }) => get(args.command, conf).await,
        Commands::Pr(PrCommands::Open { branch, dir }) => {
            let command = Commands::Pr(PrCommands::Get {
                branch,
                dir,
                open: true,
            });
            get(command, conf).await
        }
        Commands::Completion { .. } => Err(eyre!("Invalid command.")),
    }
}
