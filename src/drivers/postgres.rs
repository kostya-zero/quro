use anyhow::{Result, anyhow};
use native_tls::TlsConnector;
use postgres::{Client, SimpleQueryMessage};
use postgres_native_tls::MakeTlsConnector;

use crate::drivers::{Driver, QueryOutput};

pub struct PostgresDriver {
    client: Client,
}

impl PostgresDriver {
    pub fn new(dsn: &str) -> Result<Self> {
        let tls = MakeTlsConnector::new(TlsConnector::new()?);
        let client =
            Client::connect(dsn, tls).map_err(|e| anyhow!("failed to connect to database: {e}"))?;
        Ok(Self { client })
    }
}

impl Driver for PostgresDriver {
    fn get_tables_query(&self) -> &'static str {
        "SELECT table_name FROM information_schema.tables WHERE table_schema='public'"
    }

    fn get_databases_query(&self) -> &'static str {
        "SELECT \
            datname AS \"name\", \
            pg_get_userbyid(datdba) AS \"owner\", \
            pg_size_pretty(pg_database_size(datname)) AS \"size\", \
            pg_encoding_to_char(encoding) AS \"encoding\", \
             datcollate AS \"collate\" \
        FROM pg_database \
        WHERE datistemplate = false \
        ORDER BY pg_database_size(datname) DESC"
    }

    fn get_tables_schema(&mut self, table: &str) -> Result<QueryOutput> {
        let rows = self.client.query(
            "SELECT column_name, data_type \
             FROM information_schema.columns \
             WHERE table_schema = 'public' AND table_name = $1 \
             ORDER BY ordinal_position",
            &[&table],
        )?;

        Ok(QueryOutput {
            columns: vec!["column_name".to_owned(), "data_type".to_owned()],
            rows: rows
                .into_iter()
                .map(|row| vec![row.get(0), row.get(1)])
                .collect(),
            affected_rows: 0,
        })
    }

    fn name(&self) -> &'static str {
        "postgres"
    }

    fn execute_query(&mut self, query: &str) -> Result<QueryOutput> {
        let messages = self.client.simple_query(query)?;

        let mut results = QueryOutput::default();

        for message in messages {
            match message {
                SimpleQueryMessage::Row(row) => {
                    let values = (0..row.len())
                        .map(|index| row.get(index).unwrap_or("NULL").to_owned())
                        .collect();
                    results.rows.push(values);
                }
                SimpleQueryMessage::CommandComplete(affected_rows) => {
                    results.affected_rows = affected_rows as usize;
                }
                SimpleQueryMessage::RowDescription(columns) => {
                    results.columns = columns
                        .iter()
                        .map(|column| column.name().to_owned())
                        .collect();
                }
                _ => {}
            }
        }

        Ok(results)
    }
}
