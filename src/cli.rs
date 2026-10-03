use clap::{ArgAction, Parser};

use crate::drivers::DriverKind;

/// A command-line query runner.
#[derive(Parser)]
#[command(
    name = "quro",
    about = env!("CARGO_PKG_DESCRIPTION"),
    version = env!("CARGO_PKG_VERSION"),

)]
pub struct Cli {
    /// Database URL to connect to.
    pub database_url: Option<String>,

    /// Database driver to use. Inferred from the database URL by default.
    #[arg(short, long)]
    pub driver: Option<DriverKind>,

    /// Execute a query instead of starting the REPL.
    #[arg(short, long)]
    pub query: Option<String>,

    /// List all supported drivers.
    #[arg(long, action = ArgAction::SetTrue)]
    pub list_drivers: bool,

    /// Name of the database to connect to from the configuration file.
    /// Other sources, including environment variables and the database URL, are ignored.
    #[arg(short, long)]
    pub name: Option<String>,

    /// List all databases defined in the configuration.
    #[arg(long)]
    pub list_databases: bool,

    /// Print the path to the configuration file.
    #[arg(short, long)]
    pub config_path: bool,

    /// Use default values if the configuration cannot be loaded.
    #[arg(short, long, action = ArgAction::SetTrue)]
    pub allow_default_config: bool,
}
