use crate::cmd::{args::Commands, config::Configuration};
use color_eyre::{eyre::eyre, Result};

pub async fn login(command: Commands, conf: Configuration) -> Result<()> {
    if let Commands::Login { .. } = command {
        Err(eyre!("Unimplemented."))
    } else {
        Err(eyre!("Invalid command!"))
    }
}
