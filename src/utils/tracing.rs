use color_eyre::Result;
use std::env;
use tracing::metadata::LevelFilter;
use tracing_error::ErrorLayer;
use tracing_subscriber::{fmt, prelude::*};

pub fn init_tracing(verbosity: u8) -> Result<()> {
    if env::var("RUST_SPANTRACE").is_err() {
        env::set_var("RUST_SPANTRACE", if verbosity == 3 { "1" } else { "0" });
    }

    color_eyre::install()?;
    tracing_subscriber::registry()
        .with(ErrorLayer::default())
        .with(fmt::layer().with_filter(match verbosity {
            1 => LevelFilter::INFO,
            2 => LevelFilter::DEBUG,
            3 => LevelFilter::TRACE,
            _ => LevelFilter::OFF,
        }))
        .init();

    Ok(())
}
