use std::{net::SocketAddr, sync::Arc};

use anyhow::Context;
use axum::Router;
use clap::Parser;
use sqlx::AnyPool;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
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
    /// Database URL (e.g., sqlite://db.sqlite, mysql://user:pass@host/db, postgres://user:pass@host/db)
    #[arg(short, long)]
    pub database_url: String,

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

    let mut db_config = DatabaseConfig::from_url(&args.database_url)
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
