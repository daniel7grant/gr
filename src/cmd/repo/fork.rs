use crate::cmd::{
    args::{Cli, Commands, RepoCommands},
    config::Configuration,
};
use eyre::{eyre, Result};
use tracing::instrument;

#[instrument(skip_all, fields(command = ?args.command))]
pub fn fork(args: Cli, _: Configuration) -> Result<()> {
    let Cli { command, .. } = args;
    if let Commands::Repo(RepoCommands::Fork { .. }) = command {
        Err(eyre!("Forking repos is not implemented"))
    } else {
        Err(eyre!("Invalid command!"))
    }
}
