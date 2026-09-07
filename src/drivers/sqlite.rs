use anyhow::{Result, anyhow};
use rusqlite::{Connection, Params, types::ValueRef};

use crate::drivers::QueryOutput;

use super::Driver;

pub struct SqliteDriver {
    connection: Connection,
}

impl SqliteDriver {
    pub fn new(dsn: &str) -> Result<Self> {
        let connection =
            Connection::open(dsn).map_err(|e| anyhow!("failed to connect sqlite: {e}"))?;

        Ok(Self { connection })
    }

    fn value_to_string(&self, value: ValueRef<'_>) -> String {
        match value {
            ValueRef::Null => "NULL".to_owned(),
            ValueRef::Integer(value) => value.to_string(),
            ValueRef::Real(value) => value.to_string(),

            ValueRef::Text(value) => String::from_utf8_lossy(value).into_owned(),

            ValueRef::Blob(value) => self.format_blob(value),
        }
    }

    fn format_blob(&self, bytes: &[u8]) -> String {
        use std::fmt::Write;

        let mut output = String::with_capacity(bytes.len() * 2 + 2);
        output.push_str("0x");

        for byte in bytes {
            let _ = write!(output, "{byte:02x}");
        }

        output
    }

    fn execute_query_with<P>(&mut self, query: &str, params: P) -> Result<QueryOutput>
    where
        P: Params,
    {
        let mut statement = self.connection.prepare(query)?;
        let column_count = statement.column_count();

        if column_count == 0 {
            let affected_rows = statement.execute(params)?;

            return Ok(QueryOutput {
                affected_rows,
                ..Default::default()
            });
        }

        let columns = statement
            .column_names()
            .into_iter()
            .map(str::to_owned)
            .collect();

        let mut result_rows = Vec::new();
        let mut rows = statement.query(params)?;

        while let Some(row) = rows.next()? {
            let mut values = Vec::with_capacity(column_count);

            for index in 0..column_count {
                let value = row.get_ref(index)?;
                values.push(self.value_to_string(value));
            }

            result_rows.push(values);
        }

        Ok(QueryOutput {
            columns,
            rows: result_rows,
            affected_rows: 0,
        })
    }
}

impl Driver for SqliteDriver {
    fn get_tables_query(&self) -> &'static str {
        "SELECT name FROM sqlite_master WHERE type='table'"
    }

    fn get_databases_query(&self) -> &'static str {
        "PRAGMA database_list"
    }

    fn get_tables_schema(&mut self, table: &str) -> Result<QueryOutput> {
        self.execute_query_with("SELECT * FROM pragma_table_info(?1)", [table])
    }

    fn name(&self) -> &'static str {
        "sqlite"
    }

    fn execute_query(&mut self, query: &str) -> Result<QueryOutput> {
        self.execute_query_with(query, [])
    }
}
