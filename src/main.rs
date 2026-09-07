use std::{env, process::exit};

use anyhow::{Result, anyhow};
use clap::{CommandFactory, Parser};

use crate::{
    cli::Cli,
    drivers::{Driver, DriverKind, postgres::PostgresDriver, sqlite::SqliteDriver},
    session::Session,
    terminal::print_error,
};

mod cli;
mod drivers;
mod session;
mod terminal;

fn detect_driver(dsn: &str) -> Option<DriverKind> {
    let lower = dsn.to_ascii_lowercase();

    if lower.starts_with("postgres://") || lower.starts_with("postgresql://") {
        return Some(DriverKind::Postgres);
    }

    if lower.starts_with("sqlite:") || lower.starts_with("file:") {
        return Some(DriverKind::Sqlite);
    }

    if lower.ends_with(".db")
        || lower.ends_with(".sqlite")
        || lower.ends_with(".sqlite3")
        || lower == ":memory:"
    {
        return Some(DriverKind::Sqlite);
    }

    None
}

fn connect(dsn: &str, driver_kind: DriverKind) -> Result<Box<dyn Driver>> {
    let driver: Box<dyn Driver> = match driver_kind {
        DriverKind::Sqlite => Box::new(
            SqliteDriver::new(dsn)
                .map_err(|e| anyhow!("Failed to connect to sqlite database: {e}"))?,
        ),
        DriverKind::Postgres => Box::new(
            PostgresDriver::new(dsn)
                .map_err(|e| anyhow!("Failed to connect to postgres database: {e}"))?,
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

    let database_url = match args.database_url.or_else(|| env::var("DATABASE_URL").ok()) {
        Some(database_url) => database_url,
        None => {
            Cli::command().print_help().unwrap();
            exit(1)
        }
    };

    let driver_to_use = if let Some(d) = args.driver {
        d
    } else if let Some(d) = detect_driver(&database_url) {
        d
    } else {
        print_error(
            "Failed to auto-detect database driver. Please, specify the driver name explicitly with '--driver'.",
        );
        exit(1)
    };

    let driver = match connect(&database_url, driver_to_use) {
        Ok(d) => d,
        Err(e) => {
            print_error(&format!("Driver error: {e}"));
            exit(1)
        }
    };

    let mut session = Session::new(driver);

    if let Some(q) = args.query {
        session.execute_query(&q);
        return;
    }

    if let Err(e) = session.run_repl() {
        print_error(&format!("REPL Error: {e}"));
        exit(1)
    }
}
