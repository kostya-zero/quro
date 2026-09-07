use std::str::FromStr;

use anyhow::Result;
use colored::Colorize;
use rustyline::{DefaultEditor, error::ReadlineError};
use tabled::{
    builder::Builder,
    settings::{
        Color, Format, Modify, Style,
        object::{Columns, Rows},
        style::BorderColor,
    },
};
use thiserror::Error;

use crate::{
    drivers::{Driver, QueryOutput},
    terminal::{escape_control_chars, print_error},
};

pub struct Session {
    driver: Box<dyn Driver>,
}

#[derive(Debug)]
pub enum Command {
    Version,
    Tables,
    Db,
    Schema,
    Help,
    Driver,
    Exit,
}

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("unknown command: {0}")]
    CommandNotFound(String),

    #[error("{0}")]
    CommandError(String),

    #[error("exit")]
    Exit,
}

impl FromStr for Command {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            ".version" => Ok(Self::Version),
            ".db" => Ok(Self::Db),
            ".driver" => Ok(Self::Driver),
            ".tables" => Ok(Self::Tables),
            ".schema" => Ok(Self::Schema),
            ".help" => Ok(Self::Help),
            ".exit" | ".quit" => Ok(Self::Exit),
            _ => Err(format!("unknown command: {s}")),
        }
    }
}

impl Session {
    pub fn new(driver: Box<dyn Driver>) -> Self {
        Self { driver }
    }

    fn execute_internal_command(&mut self, command: &str) -> Result<(), SessionError> {
        let mut parts = command.splitn(2, ' ');
        let cmd = Command::from_str(parts.next().unwrap())
            .map_err(|_| SessionError::CommandNotFound(command.to_string()))?;
        match cmd {
            Command::Version => println!("{}", env!("CARGO_PKG_VERSION")),
            Command::Exit => return Err(SessionError::Exit),
            Command::Tables => self.execute_query(self.driver.get_tables_query()),
            Command::Db => self.execute_query(self.driver.get_databases_query()),
            Command::Driver => println!("{}", self.driver.name()),
            Command::Schema => {
                let table = parts
                    .next()
                    .filter(|table| !table.is_empty())
                    .ok_or_else(|| {
                        SessionError::CommandError("table name is required".to_string())
                    })?;

                let result = self.driver.get_tables_schema(table);
                self.display_query_result(result);
            }
            Command::Help => {
                let columns = vec!["command".to_string(), "description".to_string()];

                let rows = vec![
                    vec![".help".to_string(), "prints help message".to_string()],
                    vec![".version".to_string(), "prints version of quro".to_string()],
                    vec![".driver".to_string(), "prints driver name".to_string()],
                    vec![".exit, .quit".to_string(), "exit quro".to_string()],
                    vec![".db".to_string(), "prints all databases".to_string()],
                    vec![".tables".to_string(), "prints all tables".to_string()],
                    vec![
                        ".schema <table>".to_string(),
                        "prints schema of specific table".to_string(),
                    ],
                ];

                self.render_table(QueryOutput {
                    columns,
                    rows,
                    affected_rows: 0,
                });
            }
        }

        Ok(())
    }

    pub fn render_table(&self, data: QueryOutput) {
        let mut b = Builder::new();
        b.push_record(data.columns.into_iter().map(escape_control_chars));

        for row in data.rows {
            b.push_record(row.into_iter().map(escape_control_chars));
        }

        let mut t = b.build();
        t.with(
            Modify::new(Rows::first())
                .with(Format::content(|s| s.bold().to_string()))
                .with(BorderColor::filled(Color::FG_BRIGHT_BLACK)),
        );
        t.with(Style::rounded())
            .with(BorderColor::filled(Color::FG_BRIGHT_BLACK));
        t.with(BorderColor::filled(Color::FG_BRIGHT_BLACK));

        t.modify(Rows::new(1..), BorderColor::filled(Color::FG_BRIGHT_BLACK));
        t.modify(
            Columns::new(1..),
            BorderColor::filled(Color::FG_BRIGHT_BLACK),
        );

        println!("{t}");
    }

    pub fn execute_query(&mut self, query: &str) {
        let result = self.driver.execute_query(query);
        self.display_query_result(result);
    }

    fn display_query_result(&self, result: Result<QueryOutput>) {
        match result {
            Ok(data) => {
                if data.columns.is_empty() && data.rows.is_empty() {
                    println!("OK, rows affected {}.", data.affected_rows);
                } else {
                    self.render_table(data);
                }
            }
            Err(error) => print_error(&format!(
                "database error: {}",
                format_database_error(&error)
            )),
        }
    }

    pub fn run_repl(&mut self) -> Result<()> {
        let mut rl = DefaultEditor::new()?;

        let welcome_header = format!(
            "Quro v{} · {}",
            env!("CARGO_PKG_VERSION"),
            self.driver.name()
        );

        println!("{}", welcome_header.blue().bold());
        println!("{}", "Use '.help' to see available commands.".dimmed());
        let mut buf = String::new();
        loop {
            let prompt = if buf.is_empty() { "quro> " } else { "....> " };
            let readline = rl.readline(prompt);
            match readline {
                Ok(line) => {
                    rl.add_history_entry(line.as_str())?;
                    let trimmed = line.trim();

                    if trimmed.is_empty() {
                        continue;
                    }

                    if trimmed.starts_with('.') {
                        match self.execute_internal_command(trimmed) {
                            Err(SessionError::Exit) => break,
                            Err(error) => print_error(&error.to_string()),
                            Ok(()) => {}
                        }
                        continue;
                    }

                    buf.push(' ');
                    buf.push_str(trimmed);
                    if !trimmed.ends_with(';') {
                        continue;
                    }

                    self.execute_query(&buf);
                    buf.clear();
                }
                Err(ReadlineError::Interrupted) => {
                    if !buf.is_empty() {
                        buf.clear();
                        println!("Buffer has been cleared.");
                        continue;
                    }
                }
                Err(ReadlineError::Eof) => {
                    break;
                }
                Err(err) => {
                    print_error(&format!("{err}"));
                    break;
                }
            }
        }

        println!("Goodbye!");

        Ok(())
    }
}

fn format_database_error(error: &anyhow::Error) -> String {
    if let Some(db_error) = error
        .downcast_ref::<postgres::Error>()
        .and_then(postgres::Error::as_db_error)
    {
        return db_error.to_string();
    }

    error.to_string()
}
