#![feature(async_fn_traits)]

use crate::commands::connect::config::manage_configuration;
use crate::{
    cli::{Cli, Commands},
    commands::connect::config::Config,
    commands::upload,
};
use clap::Parser;
mod cli;
mod commands;
mod logging;
mod manifest;
mod operator_builder;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    crate::logging::configure_logging()?;

    let cli = Cli::parse();

    let mut config = Config::read();
    match cli.command {
        Commands::Connect { name, option } => {
            manage_configuration(&mut config, name, option).await?
        }
        Commands::Upload { info, name } => {
            let info = info.interactive_configure();
            upload::interface::upload(&info, config, &name).await?;
        }
    };

    Ok(())
}
