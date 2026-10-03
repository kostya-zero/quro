use anyhow::Result;
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
        let client = Client::connect(dsn, tls)?;
        Ok(Self { client })
    }
}

impl Driver for PostgresDriver {
    fn get_tables_query(&self) -> &'static str {
        "SELECT \
            n.nspname AS \"schema\", \
            c.relname AS \"name\", \
            CASE c.relkind \
                WHEN 'r' THEN 'table' \
                WHEN 'v' THEN 'view' \
                WHEN 'm' THEN 'materialized view' \
                WHEN 'p' THEN 'partitioned table' \
                WHEN 'f' THEN 'foreign table' \
            END AS \"type\", \
            pg_size_pretty(pg_total_relation_size(c.oid)) AS \"size\" \
        FROM pg_class c \
        JOIN pg_namespace n ON n.oid = c.relnamespace \
        WHERE c.relkind IN ('r', 'v', 'm', 'p', 'f') \
            AND NOT c.relispartition \
            AND n.nspname !~ '^pg_' \
            AND n.nspname <> 'information_schema' \
        ORDER BY 1, 2"
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
        // to_regclass resolves `schema.table`, quoted names, and search_path. It yields NULL
        // for unknown tables instead of returning an error.
        let rows = self.client.query(
            "SELECT \
                a.attname::text, \
                format_type(a.atttypid, a.atttypmod), \
                CASE WHEN a.attnotnull THEN 'no' ELSE 'yes' END, \
                coalesce(pg_get_expr(d.adbin, d.adrelid), ''), \
                concat_ws(', ', \
                    (SELECT 'PK' FROM pg_constraint c \
                     WHERE c.conrelid = a.attrelid AND c.contype = 'p' AND a.attnum = ANY(c.conkey)), \
                    (SELECT string_agg('FK → ' || c.confrelid::regclass::text || '(' || fa.attname || ')', ', ') \
                     FROM pg_constraint c \
                     CROSS JOIN LATERAL unnest(c.conkey, c.confkey) AS k(attnum, fattnum) \
                     JOIN pg_attribute fa ON fa.attrelid = c.confrelid AND fa.attnum = k.fattnum \
                     WHERE c.conrelid = a.attrelid AND c.contype = 'f' AND k.attnum = a.attnum)) \
            FROM pg_attribute a \
            LEFT JOIN pg_attrdef d ON d.adrelid = a.attrelid AND d.adnum = a.attnum \
            WHERE a.attrelid = to_regclass($1) AND a.attnum > 0 AND NOT a.attisdropped \
            ORDER BY a.attnum",
            &[&table],
        )?;

        Ok(QueryOutput {
            columns: ["column", "type", "nullable", "default", "key"]
                .map(str::to_owned)
                .to_vec(),
            rows: rows
                .into_iter()
                .map(|row| (0..row.len()).map(|index| row.get(index)).collect())
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
