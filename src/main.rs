use std::{env, process::exit};

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use colored::Colorize;

use crate::{
    cli::Cli,
    config::{Config, config_path, load_config},
    drivers::{
        Driver, DriverKind, format_database_error, postgres::PostgresDriver, sqlite::SqliteDriver,
    },
    session::Session,
    terminal::{print_error, print_warn},
};

mod cli;
mod config;
mod drivers;
mod session;
mod terminal;

fn detect_driver(dsn: &str) -> Option<DriverKind> {
    let lower = dsn.to_ascii_lowercase();

    if lower.starts_with("postgres://") || lower.starts_with("postgresql://") {
        return Some(DriverKind::Postgres);
    }

    if lower.ends_with(".db")
        || lower.ends_with(".sqlite")
        || lower.ends_with(".sqlite3")
        || lower.starts_with("file:")
        || lower == ":memory:"
    {
        return Some(DriverKind::Sqlite);
    }

    None
}

fn connect(dsn: &str, driver_kind: DriverKind) -> Result<Box<dyn Driver>> {
    let driver: Box<dyn Driver> = match driver_kind {
        DriverKind::Sqlite => {
            Box::new(SqliteDriver::new(dsn).context("Failed to connect to the SQLite database")?)
        }
        DriverKind::Postgres => Box::new(
            PostgresDriver::new(dsn).context("Failed to connect to the PostgreSQL database")?,
        ),
    };

    Ok(driver)
}

fn main() {
    let args = Cli::parse();
    if args.list_drivers {
        println!("postgres\nsqlite");
        return;
    }

    if args.config_path {
        println!("{}", config_path().to_string_lossy());
        return;
    }

    let config = if config_path().exists() {
        match load_config() {
            Ok(c) => c,
            Err(e) => {
                if !args.allow_default_config {
                    print_error(&format!("Failed to load the configuration: {e}"));
                    exit(1)
                }
                print_warn(&format!(
                    "Failed to load the configuration; using defaults: {e}"
                ));
                Config::default()
            }
        }
    } else {
        Config::default()
    };

    if args.list_databases {
        config
            .databases
            .iter()
            .for_each(|f| println!("{} {}", f.0, format!("({})", f.1).bright_black()));
        return;
    }

    let database_url = if let Some(name) = args.name {
        match config.databases.get(&name) {
            Some(url) => url.clone(),
            None => {
                print_error(&format!(
                    "Database '{name}' was not found in the configuration."
                ));
                exit(1)
            }
        }
    } else {
        match args.database_url.or_else(|| env::var("DATABASE_URL").ok()) {
            Some(url) => url,
            None => {
                if let Err(e) = Cli::command().print_help() {
                    print_error(&format!("Could not print help: {e}"));
                }
                exit(1)
            }
        }
    };

    let driver_to_use = if let Some(d) = args.driver {
        d
    } else if let Some(d) = detect_driver(&database_url) {
        d
    } else {
        print_error("Could not detect the database driver. Specify it explicitly with '--driver'.");
        exit(1)
    };

    let driver = match connect(&database_url, driver_to_use) {
        Ok(d) => d,
        Err(e) => {
            print_error(&format!("{e}: {}", format_database_error(&e)));
            exit(1)
        }
    };

    let mut session = Session::new(driver, config);

    if let Some(q) = args.query {
        match session.execute_query(&q) {
            Ok(d) => {
                session.render_table(d);
                return;
            }
            Err(e) => {
                print_error(&format!("Database error: {}", format_database_error(&e)));
                exit(1)
            }
        }
    }

    if let Err(e) = session.run_repl() {
        print_error(&format!("REPL error: {e}"));
        exit(1)
    }
}
