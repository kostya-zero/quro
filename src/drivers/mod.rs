use anyhow::Result;
use clap::ValueEnum;

pub mod postgres;
pub mod sqlite;

#[derive(Debug, ValueEnum, Clone)]
pub enum DriverKind {
    Sqlite,
    Postgres,
}

#[derive(Debug, Clone, Default)]
pub struct QueryOutput {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub affected_rows: usize,
}

pub trait Driver {
    fn get_tables_query(&self) -> &'static str;
    fn get_databases_query(&self) -> &'static str;
    fn get_tables_schema(&mut self, table: &str) -> Result<QueryOutput>;
    fn name(&self) -> &'static str;
    fn execute_query(&mut self, query: &str) -> Result<QueryOutput>;
    fn is_complete(&self, sql: &str) -> bool;
}

pub fn format_database_error(error: &anyhow::Error) -> String {
    if let Some(db_error) = error
        .downcast_ref::<::postgres::Error>()
        .and_then(::postgres::Error::as_db_error)
    {
        return db_error.message().to_owned();
    }

    if let Some(sqlite_error) = error.downcast_ref::<rusqlite::Error>() {
        return sqlite_error.to_string();
    }

    error.root_cause().to_string()
}
