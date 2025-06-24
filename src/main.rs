use clap::Parser;
use rocket::fs::FileServer;
use rocket::routes;

mod config;
mod models;
mod routes;
mod template;

use config::{DatabaseConfig, DatabasePool};
use routes::*;

#[derive(Parser, Debug)]
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

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    let args = Args::parse();

    let db_config = DatabaseConfig::from_url(&args.database_url).expect("Invalid database URL");

    let host = args.host.clone();
    let port = args.port;

    let figment = rocket::Config::figment()
        // I recommand you to replace it and build a binary to use, but this almost
        // use yourself, so security is considered behind the functions.
        .merge(("secret_key", "h/ie6GKkDtaurjNrQYCRsrSaWLNRVA2hSeyMSD8NycZphe7Le6ZZiJsdareCfE3jIuMV9hG/nbxRCJNKhUBkuw=="))
        .merge(("address", host))
        .merge(("port", port));

    let _res = rocket::custom(figment)
        .manage(args)
        .manage(db_config.clone())
        .mount(
            "/",
            routes![
                index::index,
                index::index_redirect,
                index::login,
                index::login_post,
                index::logout,
                query::query_page,
                query::execute_query,
                query::api_execute_query,
                tables::table_list,
                tables::table_structure,
                tables::table_content,
                tables::table_query,
                tables::table_query_execute,
                tables::table_insert,
                tables::table_update,
                tables::table_update_execute,
                tables::table_export,
                tables::table_import,
                indexes::add_index,
                indexes::add_index_execute,
                indexes::drop_index,
                indexes::drop_index_execute,
                columns::add_column,
                columns::add_column_execute,
                columns::drop_column,
                columns::drop_column_execute,
                columns::rename_column,
                columns::rename_column_execute,
            ],
        )
        .mount("/static", FileServer::from("static"))
        .attach(DatabasePool::init())
        .ignite()
        .await?
        .launch()
        .await?;

    Ok(())
}
