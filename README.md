# SQL Web

A single-binary web database browser for SQLite, MySQL, and PostgreSQL written in Rust with Axum, React, Tailwind, and SQLx.

This is a Rust rewrite of the Python [sqlite-web](https://github.com/coleifer/sqlite-web) project, extended to support multiple database types.

> Is Still in Building.
> If you are willing to help me enhance the frontend looklike or help me fix bug, welcome to give me a pull request.

## Features

- **Multi-database support**: SQLite, MySQL, and PostgreSQL
- **Single binary**: React frontend is built and embedded into the Rust binary
- **Web-based interface**: Browse and manage your databases through a browser
- **Data browsing**: View table contents with pagination
- **SQL query execution**: Run arbitrary SQL queries
- **Data manipulation**: Insert and update rows
- **Schema management**: Add/drop/rename columns and add/drop indexes
- **Read-only mode**: Prevent accidental modifications
- **Authentication**: Simple password protection

## Installation

### Prerequisites

- Rust 1.85 or later
- Cargo
- pnpm

### Building from source

```bash
git clone <repository-url>
cd sql-web
cargo build --release
```

`cargo build` runs `pnpm install` when needed and `pnpm build`, then embeds `frontend/dist` into the binary.

The binary will be available at `target/release/sql-web`.

## Usage

```bash
# SQLite database
sql-web --database-url "sqlite://path/to/database.db"

# MySQL database
sql-web --database-url "mysql://user:password@localhost/database_name"

# PostgreSQL database
sql-web --database-url "postgres://user:password@localhost/database_name"
```

### Command-line options

```text
sql-web [OPTIONS] --database-url <DATABASE_URL>

Options:
  -d, --database-url <DATABASE_URL>
          Database URL

  -H, --host <HOST>
          Host to bind to [default: 127.0.0.1]

  -p, --port <PORT>
          Port to bind to [default: 8080]

  -r, --readonly
          Enable read-only mode

  -R, --rows-per-page <ROWS_PER_PAGE>
          Rows per page for content view [default: 50]

  -Q, --query-rows-per-page <QUERY_ROWS_PER_PAGE>
          Rows per page for query results [default: 1000]

      --debug
          Enable debug logging

  -h, --help
          Print help
```

### Authentication

Set the `SQL_WEB_PASSWORD` environment variable to require password authentication:

```bash
export SQL_WEB_PASSWORD="your-secret-password"
sql-web --database-url "sqlite://example.db"
```

If no password is set, the default password is `admin`.

### Accessing the web interface

Once started, open your browser and navigate to:

- `http://localhost:8080` or your custom host/port

## Database URL Format

### SQLite

```text
sqlite://path/to/file.db
sqlite:///absolute/path/to/file.db
sqlite://file.db?mode=ro
```

### MySQL

```text
mysql://username:password@host:port/database
mysql://username:password@host/database
```

### PostgreSQL

```text
postgres://username:password@host:port/database
postgresql://username:password@host:port/database
postgres://username:password@host/database?sslmode=require
```

## Development

### Backend + embedded frontend

```bash
cargo run -- --database-url "sqlite://test.db" --debug
```

### Frontend dev server

```bash
cd frontend
pnpm install
pnpm dev
```

The Vite dev server proxies `/api` to `http://127.0.0.1:8080`.

### Skip frontend build during cargo checks

If `frontend/dist` already exists and you only want to check Rust quickly:

```bash
SQL_WEB_SKIP_FRONTEND_BUILD=1 cargo check
```

## Project structure

```text
sql-web/
├── build.rs             # Builds frontend before embedding
├── frontend/            # React + Tailwind frontend
├── src/
│   ├── api/             # Axum JSON API handlers
│   ├── assets.rs        # Embedded SPA static asset server
│   ├── config.rs        # Database configuration and management
│   ├── main.rs          # Application entry point
│   └── models.rs        # Shared API data structures
└── Cargo.toml
```

## Dependencies

- **Axum**: Web framework
- **React + Tailwind**: Frontend
- **SQLx**: Database connectivity
- **Clap**: Command-line argument parsing

## License

This project is inspired by and maintains compatibility with the original [sqlite-web](https://github.com/coleifer/sqlite-web) project by Charles Leifer.
