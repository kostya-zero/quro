use anyhow::{Result, anyhow, bail};
use colored::Colorize;
use rustyline::{DefaultEditor, error::ReadlineError};
use tabled::{
    builder::Builder,
    settings::{
        Color, Format, Style,
        object::{Rows, Segment},
        style::BorderColor,
    },
};

use crate::{
    config::{BorderStyle, Config},
    drivers::{Driver, QueryOutput, format_database_error},
    terminal::{escape_control_chars, print_error},
};

pub struct Session {
    driver: Box<dyn Driver>,
    config: Config,
}

impl Session {
    pub fn new(driver: Box<dyn Driver>, config: Config) -> Self {
        Self { driver, config }
    }

    fn execute_internal_command(&mut self, command: &str) -> Result<bool> {
        let (cmd, args) = command.split_once(' ').unwrap_or((command, ""));
        match cmd.to_ascii_lowercase().as_str() {
            ".version" => println!("{}", env!("CARGO_PKG_VERSION")),
            ".exit" | ".quit" => return Ok(true),
            ".tables" => {
                let result = self
                    .execute_query(self.driver.get_tables_query())
                    .map_err(|e| anyhow!("Database error: {}", format_database_error(&e)))?;
                self.display_query_result(result);
            }
            ".db" => {
                let result = self
                    .execute_query(self.driver.get_databases_query())
                    .map_err(|e| anyhow!("Database error: {}", format_database_error(&e)))?;
                self.display_query_result(result);
            }
            ".driver" => println!("{}", self.driver.name()),
            ".schema" => {
                let table = args.trim();
                if table.is_empty() {
                    bail!("A table name is required.");
                }

                let result = self
                    .driver
                    .get_tables_schema(table)
                    .map_err(|e| anyhow!("Database error: {}", format_database_error(&e)))?;
                if result.rows.is_empty() {
                    bail!("Table not found: {table}");
                }
                self.display_query_result(result);
            }
            ".help" => {
                let columns = vec!["command".to_string(), "description".to_string()];

                let rows = vec![
                    vec![".help".to_string(), "Show available commands".to_string()],
                    vec![".version".to_string(), "Show the Quro version".to_string()],
                    vec![".driver".to_string(), "Show the driver name".to_string()],
                    vec![".exit, .quit".to_string(), "Exit Quro".to_string()],
                    vec![".db".to_string(), "List all databases".to_string()],
                    vec![".tables".to_string(), "List all tables".to_string()],
                    vec![
                        ".schema <table>".to_string(),
                        "Show the schema for a table".to_string(),
                    ],
                ];

                self.render_table(QueryOutput {
                    columns,
                    rows,
                    affected_rows: 0,
                });
            }
            _ => bail!("Unknown command: {command}"),
        }

        Ok(false)
    }

    pub fn render_table(&self, data: QueryOutput) {
        let mut b = Builder::new();
        b.push_record(data.columns.into_iter().map(escape_control_chars));

        for row in data.rows {
            b.push_record(row.into_iter().map(escape_control_chars));
        }

        let mut t = b.build();

        match (&self.config.appearence.border_style, t.count_rows() == 1) {
            (BorderStyle::None, _) => t.with(Style::empty()),
            (BorderStyle::Rounded, true) => t.with(Style::rounded().remove_horizontals()),
            (BorderStyle::Rounded, false) => t.with(Style::rounded()),
            (BorderStyle::Modern, true) => t.with(Style::modern().remove_horizontal()),
            (BorderStyle::Modern, false) => t.with(Style::modern()),
            (BorderStyle::Psql, true) => t.with(Style::psql().remove_horizontals()),
            (BorderStyle::Psql, false) => t.with(Style::psql()),
            (BorderStyle::Ascii, true) => t.with(Style::ascii().remove_horizontal()),
            (BorderStyle::Ascii, false) => t.with(Style::ascii()),
            (BorderStyle::AsciiRounded, _) => t.with(Style::ascii_rounded()),
            (BorderStyle::ModernRounded, true) => {
                t.with(Style::modern_rounded().remove_horizontal())
            }
            (BorderStyle::ModernRounded, false) => t.with(Style::modern_rounded()),
            (BorderStyle::Sharp, true) => t.with(Style::sharp().remove_horizontals()),
            (BorderStyle::Sharp, false) => t.with(Style::sharp()),
            (BorderStyle::Extended, true) => t.with(Style::extended().remove_horizontal()),
            (BorderStyle::Extended, false) => t.with(Style::extended()),
            (BorderStyle::Dots, true) => t.with(Style::dots().remove_horizontal()),
            (BorderStyle::Dots, false) => t.with(Style::dots()),
            (BorderStyle::Markdown, true) => t.with(Style::markdown().remove_horizontals()),
            (BorderStyle::Markdown, false) => t.with(Style::markdown()),
            (BorderStyle::ReStructuredText, true) => {
                t.with(Style::re_structured_text().remove_horizontals())
            }
            (BorderStyle::ReStructuredText, false) => t.with(Style::re_structured_text()),
            (BorderStyle::Blank, _) => t.with(Style::blank()),
        };
        t.modify(Segment::all(), BorderColor::filled(Color::FG_BRIGHT_BLACK));
        t.modify(Rows::first(), Format::content(|s| s.bold().to_string()));

        println!("{t}");
    }

    pub fn execute_query(&mut self, query: &str) -> Result<QueryOutput> {
        self.driver.execute_query(query)
    }

    fn display_query_result(&self, result: QueryOutput) {
        if result.columns.is_empty() && result.rows.is_empty() {
            println!("OK. Rows affected: {}.", result.affected_rows);
        } else {
            self.render_table(result);
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

                    if line.is_empty() {
                        continue;
                    }

                    if line.starts_with('.') {
                        match self.execute_internal_command(&line) {
                            Ok(true) => return Ok(()),
                            Ok(false) => {}
                            Err(error) => print_error(&error.to_string()),
                        }
                        continue;
                    }

                    buf.push(' ');
                    buf.push_str(&line);
                    if !is_complete(&buf) {
                        buf.push('\n');
                        continue;
                    }

                    match self.execute_query(&buf) {
                        Ok(d) => self.render_table(d),
                        Err(e) => {
                            print_error(&format!("Database error: {}", format_database_error(&e)))
                        }
                    }
                    buf.clear();
                }
                Err(ReadlineError::Interrupted) => {
                    if !buf.is_empty() {
                        buf.clear();
                        println!("The query buffer was cleared.");
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

/// Checks whether the buffer ends with a `;` outside of strings, comments and dollar-quoted bodies.
fn is_complete(sql: &str) -> bool {
    let mut i = 0;
    let mut ends_with_semicolon = false;
    while i < sql.len() {
        let rest = &sql[i..];
        if rest.starts_with("--") {
            i += rest.find('\n').unwrap_or(rest.len());
        } else if let Some(comment) = rest.strip_prefix("/*") {
            let Some(end) = comment.find("*/") else {
                return false;
            };
            i += end + 4;
        } else if let Some(literal) = rest.strip_prefix('\'') {
            // Doubled single quotes are treated as adjacent string literals.
            let Some(end) = literal.find('\'') else {
                return false;
            };
            i += end + 2;
            ends_with_semicolon = false;
        } else if let Some(tag) = dollar_tag(rest) {
            let Some(end) = rest[tag.len()..].find(tag) else {
                return false;
            };
            i += end + tag.len() * 2;
            ends_with_semicolon = false;
        } else {
            let c = rest.chars().next().expect("rest is non-empty");
            if !c.is_whitespace() {
                ends_with_semicolon = c == ';';
            }
            i += c.len_utf8();
        }
    }
    ends_with_semicolon
}

/// Matches `$$` or `$tag$`, but not `$1`.
fn dollar_tag(s: &str) -> Option<&str> {
    let body = s.strip_prefix('$')?;
    if body.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    let end = body.find(|c: char| !(c.is_alphanumeric() || c == '_'))?;
    (body.as_bytes()[end] == b'$').then(|| &s[..end + 2])
}
