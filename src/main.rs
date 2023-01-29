mod cmd;

use cmd::{
    args::{Cli, Commands, PrCommands},
    config::Configuration,
    login::login::login,
    pr::{approve::approve, close::close, create::create, get::get, list::list, merge::merge},
};
use color_eyre::eyre::eyre;
use color_eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let mut args = Cli::parse_args();
    let conf = Configuration::parse()?;

    match args.command {
        Commands::Login { .. } => login(args, conf).await,
        Commands::Pr(PrCommands::Create { .. }) => create(args, conf).await,
        Commands::Pr(PrCommands::Get { .. }) => get(args, conf).await,
        Commands::Pr(PrCommands::Open { branch, dir, auth }) => {
            args.command = Commands::Pr(PrCommands::Get {
                branch,
                dir,
                open: true,
                auth,
            });
            get(args, conf).await
        }
        Commands::Pr(PrCommands::List { .. }) => list(args, conf).await,
        Commands::Pr(PrCommands::Approve { .. }) => approve(args, conf).await,
        Commands::Pr(PrCommands::Merge { .. }) => merge(args, conf).await,
        Commands::Pr(PrCommands::Close { .. }) => close(args, conf).await,
        Commands::Completion { .. } => Err(eyre!("Invalid command.")),
    }
}
