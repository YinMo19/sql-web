# SQL Web

# YinMo19

A web-based database browser for SQLite, MySQL, and PostgreSQL written in Rust using Rocket and SQLx.

This is a Rust rewrite of the Python [sqlite-web](https://github.com/coleifer/sqlite-web) project, extended to support multiple database types.

> Is Still in Building.
> If you are willing to help me enhance the frontend looklike or help me fix bug, welcome to give me a pull request.

## Features

- **Multi-database support**: SQLite, MySQL, and PostgreSQL
- **Web-based interface**: Browse and manage your databases through a web browser
- **Table management**: Create, drop, and modify tables
- **Data browsing**: View table contents with pagination
- **SQL query execution**: Run arbitrary SQL queries
- **Data manipulation**: Insert, update, and delete rows
- **Schema management**: Add/drop columns and indexes
- **Data export/import**: Export data as JSON or CSV (planned)
- **Read-only mode**: Prevent accidental modifications
- **Authentication**: Simple password protection

## Installation

### Prerequisites

- Rust 1.70 or later
- Cargo

### Building from source

1. Clone the repository:

    ```bash
    git clone <repository-url>
    cd sql-web
    ```

2. Build the project:

    ```bash
    cargo build --release
    ```

3. The binary will be available at `target/release/sql-web`

## Usage

### Basic usage

```bash
# SQLite database
sql-web --database-url "sqlite://path/to/database.db"

# MySQL database
sql-web --database-url "mysql://user:password@localhost/database_name"

# PostgreSQL database
sql-web --database-url "postgres://user:password@localhost/database_name"
```

### Command-line options

```
sql-web [OPTIONS] --database-url <DATABASE_URL>

Options:
  -d, --database-url <DATABASE_URL>
          Database URL (e.g., sqlite://db.sqlite, mysql://user:pass@host/db, postgres://user:pass@host/db)

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

  -d, --debug
          Enable debug mode

  -h, --help
          Print help
```

### Examples

#### SQLite

```bash
# Basic SQLite usage
sql-web --database-url "sqlite://example.db"

# Read-only SQLite
sql-web --database-url "sqlite://example.db?mode=ro" --readonly
```

#### MySQL

```bash
# Connect to MySQL
sql-web --database-url "mysql://root:password@localhost/mydb"

# MySQL with custom port
sql-web --database-url "mysql://user:pass@localhost:3307/mydb"
```

#### PostgreSQL

```bash
# Connect to PostgreSQL
sql-web --database-url "postgres://user:password@localhost/mydb"

# PostgreSQL with SSL
sql-web --database-url "postgres://user:pass@localhost/mydb?sslmode=require"
```

### Authentication

Set the `SQL_WEB_PASSWORD` environment variable to require password authentication:

```bash
export SQL_WEB_PASSWORD="your-secret-password"
sql-web --database-url "sqlite://example.db"
```

If no password is set, the default password is `admin`.

### Accessing the web interface

Once started, open your web browser and navigate to:

- http://localhost:8080 (or your custom host/port)

## Database URL Format

### SQLite

```
sqlite://path/to/file.db
sqlite:///absolute/path/to/file.db
sqlite://file.db?mode=ro  # Read-only mode
```

### MySQL

```
mysql://username:password@host:port/database
mysql://username:password@host/database  # Default port 3306
```

### PostgreSQL

```
postgres://username:password@host:port/database
postgresql://username:password@host:port/database
postgres://username:password@host/database?sslmode=require
```

## Features Overview

### Database Overview

- View database statistics
- List all tables
- Quick SQL query execution

### Table Management

- **Browse**: View table contents with pagination
- **Structure**: Examine table schema, columns, indexes
- **Query**: Execute custom SQL queries on specific tables
- **Insert**: Add new rows to tables
- **Edit**: Modify existing rows
- **Delete**: Remove rows from tables

### Schema Operations

- **Add Column**: Add new columns to existing tables
- **Drop Column**: Remove columns (MySQL/PostgreSQL only)
- **Rename Column**: Rename existing columns
- **Add Index**: Create new indexes
- **Drop Index**: Remove indexes
- **Create Table**: Create new tables
- **Drop Table**: Remove entire tables

### Query Interface

- Execute arbitrary SQL queries
- View results in a formatted table
- Export results (planned feature)
- Query history and bookmarks (planned feature)

## Development

### Running in development mode

```bash
cargo run -- --database-url "sqlite://test.db" --debug
```

### Project structure

```
sql-web/
├── src/
│   ├── main.rs          # Application entry point
│   ├── config.rs        # Database configuration and management
│   ├── models.rs        # Data structures and models
│   └── routes/          # HTTP route handlers
│       ├── mod.rs
│       ├── index.rs     # Home page and authentication
│       ├── query.rs     # SQL query execution
│       ├── tables.rs    # Table operations
│       ├── columns.rs   # Column management
│       └── indexes.rs   # Index management
├── templates/           # Askama HTML templates
├── static/             # Static assets (CSS, JS)
└── Cargo.toml
```

### Dependencies

- **Rocket**: Web framework
- **SQLx**: Database connectivity
- **Askama**: Template engine
- **Clap**: Command-line argument parsing

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is inspired by and maintains compatibility with the original [sqlite-web](https://github.com/coleifer/sqlite-web) project by Charles Leifer.

## Roadmap

- [ ] Complete data export/import functionality
- [ ] Query history and bookmarks
- [ ] Foreign key relationship visualization
- [ ] Database migrations support
- [ ] Multiple database connections
- [ ] User management and permissions
- [ ] API endpoints for programmatic access
- [ ] Docker support
- [ ] Configuration file support
