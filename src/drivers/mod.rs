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
}
