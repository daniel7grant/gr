use crate::cmd::args::OutputType;
use color_eyre::Result;
use std::env;
use tracing_error::ErrorLayer;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init_tracing(verbosity: u8, output: OutputType) -> Result<()> {
    if env::var("RUST_SPANTRACE").is_err() {
        env::set_var("RUST_SPANTRACE", if verbosity == 3 { "1" } else { "0" });
    }
    if env::var("RUST_BACKTRACE").is_err() {
        env::set_var("RUST_BACKTRACE", if verbosity == 3 { "1" } else { "0" });
    }

    color_eyre::config::HookBuilder::default()
        .display_env_section(false)
        .install()?;

    // Set error layer and filter all traces except ours
    let subscriber = tracing_subscriber::registry()
        .with(ErrorLayer::default())
        .with(EnvFilter::from(match verbosity {
            1 => "gr=info",
            2 => "gr=debug",
            3 => "gr=trace",
            _ => "gr=error",
        }));

    // Select output to JSON, or leave it regular
    if output == OutputType::Json {
        subscriber
            .with(fmt::layer().with_target(verbosity > 1).json())
            .init();
    } else if verbosity == 0 {
        // If verbosity is zero we don't want to output any styles, only the level and error message
        subscriber
            .with(fmt::layer().with_target(false).without_time())
            .init();
    } else {
        // Otherwise output everything
        subscriber
            .with(fmt::layer().with_target(verbosity > 1))
            .init();
    }

    Ok(())
}
