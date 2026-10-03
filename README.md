# Quro

**Quro** is a command-line query runner for databases such as PostgreSQL and SQLite. It provides a REPL for executing SQL queries against connected databases.

## Features

- **Interactive REPL**: Execute SQL queries in real-time.
- **Multiple databases**: Supports PostgreSQL and SQLite.
- **Simple configuration**: Connect using standard Data Source Names (DSNs).

## Installation

If you have the Rust toolchain installed, run:

```bash
cargo install quro
```

Download a binary for your operating system and architecture from [GitHub Releases](https://github.com/kostya-zero/quro/releases).

## Usage

Run the tool with a DSN:

```bash
# Provide a SQLite DSN
quro file:my_database.db

# Provide a PostgreSQL DSN
quro "postgres://user:password@localhost:5432/dbname"
```

Quro detects the driver from the DSN. You can also specify the driver explicitly or provide the DSN through an environment variable:

```bash
# Using DATABASE_URL environment variable
DATABASE_URL="postgres://user:password@localhost:5432/dbname" quro

# Specifying driver explicitly
quro -d postgres "user=myuser password=mypass dbname=mydb"
```


You can find even more options with `--help` argument.

## License

This project is licensed under the MIT License.
