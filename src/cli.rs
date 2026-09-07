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
    /// A database URL that Quro had to connect to.
    pub database_url: Option<String>,

    /// A driver to use. Usually determined from database URL,
    /// but can be specified explicitly.
    #[arg(short, long)]
    pub driver: Option<DriverKind>,

    /// Execute a specified query instead of launching REPL.
    #[arg(short, long)]
    pub query: Option<String>,

    /// Print a list of all supported drivers.
    #[arg(short, long, action = ArgAction::SetTrue)]
    pub list_drivers: bool,
}
