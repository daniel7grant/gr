use crate::cmd::{
    args::{Cli, Commands, RepoCommands},
    config::Configuration,
};
use eyre::{eyre, Result};
use tracing::instrument;

#[instrument(skip_all, fields(command = ?args.command))]
pub fn new(args: Cli, _: Configuration) -> Result<()> {
    let Cli { command, .. } = args;
    if let Commands::Repo(RepoCommands::New { .. }) = command {
        Err(eyre!("Creating new repo is not implemented"))
    } else {
        Err(eyre!("Invalid command!"))
    }
}
