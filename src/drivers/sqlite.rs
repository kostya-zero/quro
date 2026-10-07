use std::ffi::CString;

use anyhow::Result;
use rusqlite::{Connection, Params, types::ValueRef};

use crate::drivers::QueryOutput;

use super::Driver;

pub struct SqliteDriver {
    connection: Connection,
}

impl SqliteDriver {
    pub fn new(dsn: &str) -> Result<Self> {
        let connection = Connection::open(dsn)?;

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
        "SELECT schema, name, type, ncol AS columns \
        FROM pragma_table_list \
        WHERE name NOT GLOB 'sqlite_*' \
        ORDER BY schema, name"
    }

    fn get_databases_query(&self) -> &'static str {
        "PRAGMA database_list"
    }

    fn get_tables_schema(&mut self, table: &str) -> Result<QueryOutput> {
        self.execute_query_with(
            "SELECT \
                p.name AS \"column\", \
                p.type AS \"type\", \
                CASE WHEN p.\"notnull\" THEN 'no' ELSE 'yes' END AS \"nullable\", \
                coalesce(p.dflt_value, '') AS \"default\", \
                concat_ws(', ', \
                    CASE WHEN p.pk > 0 THEN 'PK' END, \
                    (SELECT group_concat('FK → ' || f.\"table\" || coalesce('(' || f.\"to\" || ')', ''), ', ') \
                     FROM pragma_foreign_key_list(?1) f \
                     WHERE f.\"from\" = p.name)) AS \"key\" \
            FROM pragma_table_info(?1) p \
            ORDER BY p.cid",
            [table],
        )
    }

    fn name(&self) -> &'static str {
        "sqlite"
    }

    fn execute_query(&mut self, query: &str) -> Result<QueryOutput> {
        self.execute_query_with(query, [])
    }

    fn is_complete(&self, sql: &str) -> bool {
        let Ok(sql) = CString::new(sql) else {
            return true;
        };

        unsafe { rusqlite::ffi::sqlite3_complete(sql.as_ptr()) != 0 }
    }
}
