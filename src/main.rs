use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use axum::Router;
use clap::Parser;
use sqlx::AnyPool;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use url::Url;
use uuid::Uuid;

mod api;
mod assets;
mod config;
mod models;

use config::DatabaseConfig;

#[derive(Parser, Debug, Clone)]
#[command(name = "sql-web")]
#[command(about = "A web-based database browser for SQLite, MySQL, and PostgreSQL")]
pub struct Args {
    /// Database URL or SQLite database file path
    #[arg(value_name = "DATABASE", conflicts_with = "database_url")]
    pub database: Option<String>,

    /// Database URL (e.g., mysql://user:pass@host/db, postgres://user:pass@host/db)
    #[arg(short = 'd', long, value_name = "DATABASE_URL")]
    pub database_url: Option<String>,

    /// Host to bind to
    #[arg(short = 'H', long, default_value = "127.0.0.1")]
    pub host: String,

    /// Port to bind to
    #[arg(short, long, default_value = "8080")]
    pub port: u16,

    /// Enable read-only mode
    #[arg(short, long)]
    pub readonly: bool,

    /// Rows per page for content view
    #[arg(short = 'R', long, default_value = "50")]
    pub rows_per_page: usize,

    /// Rows per page for query results
    #[arg(short = 'Q', long, default_value = "1000")]
    pub query_rows_per_page: usize,

    /// Enable debug mode
    #[arg(long)]
    pub debug: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub args: Args,
    pub db_config: DatabaseConfig,
    pub pool: AnyPool,
    pub auth_token: String,
}

pub type SharedState = Arc<AppState>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    init_tracing(args.debug);
    sqlx::any::install_default_drivers();

    let database_url = resolve_database_url(&args)?;
    let mut db_config = DatabaseConfig::from_url(&database_url)
        .map_err(|error| anyhow::anyhow!("Invalid database URL: {error}"))?;
    db_config.readonly = db_config.readonly || args.readonly;

    let pool = AnyPool::connect(&db_config.url)
        .await
        .context("Failed to connect to database")?;

    let addr: SocketAddr = format!("{}:{}", args.host, args.port)
        .parse()
        .context("Invalid bind address")?;

    let state = Arc::new(AppState {
        args,
        db_config,
        pool,
        auth_token: Uuid::new_v4().to_string(),
    });

    let app = Router::new()
        .nest("/api", api::router(state.clone()))
        .fallback(assets::serve)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("Failed to bind to {addr}"))?;

    tracing::info!("sql-web listening on http://{addr}");
    axum::serve(listener, app).await?;

    Ok(())
}

fn resolve_database_url(args: &Args) -> anyhow::Result<String> {
    let input = args
        .database_url
        .as_deref()
        .or(args.database.as_deref())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "missing database: use `sql-web ./database.db` or `sql-web --database-url <URL>`"
            )
        })?;

    if has_supported_scheme(input) {
        return Ok(input.to_string());
    }

    if is_sqlite_file_path(input) {
        return sqlite_file_url(Path::new(input));
    }

    Err(anyhow::anyhow!(
        "unsupported database input `{input}`: pass a mysql/postgres/sqlite URL, or a SQLite file ending in .db, .sqlite, .sqlite3, or .db3"
    ))
}

fn has_supported_scheme(input: &str) -> bool {
    Url::parse(input)
        .map(|url| matches!(url.scheme(), "sqlite" | "mysql" | "postgres" | "postgresql"))
        .unwrap_or(false)
}

fn is_sqlite_file_path(input: &str) -> bool {
    Path::new(input)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "db" | "sqlite" | "sqlite3" | "db3"
            )
        })
        .unwrap_or(false)
}

fn sqlite_file_url(path: &Path) -> anyhow::Result<String> {
    let absolute_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let file_url = Url::from_file_path(normalize_path(&absolute_path))
        .map_err(|_| anyhow::anyhow!("invalid SQLite database path: {}", path.display()))?
        .to_string();
    Ok(file_url.replacen("file:", "sqlite:", 1))
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            component => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

fn init_tracing(debug: bool) {
    let default_filter = if debug { "sql_web=debug,info" } else { "info" };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| default_filter.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
