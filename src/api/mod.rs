use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

use crate::{
    AppState, SharedState,
    config::{
        DatabaseConfig, DatabaseManager, DatabaseType, escape_string_literal, is_write_operation,
        optional_sql_value,
    },
    models::{
        ColumnDetail, CreateColumnRequest, DatabaseStats, IndexDetail, QueryRequest, QueryResponse,
        TableStructure,
    },
};

const AUTH_COOKIE: &str = "sql_web_session";

pub fn router(state: SharedState) -> Router<SharedState> {
    let protected = Router::new()
        .route("/overview", get(overview))
        .route("/tables", get(tables))
        .route("/tables/{table}/structure", get(table_structure))
        .route("/tables/{table}/rows", get(table_rows).post(insert_row))
        .route("/tables/{table}/rows", patch(update_row))
        .route("/query", post(execute_query))
        .route("/tables/{table}/sql", post(execute_table_sql))
        .route("/tables/{table}/columns", post(add_column))
        .route(
            "/tables/{table}/columns/{column}",
            delete(drop_column).patch(rename_column),
        )
        .route("/tables/{table}/indexes", post(add_index))
        .route("/tables/{table}/indexes/{index}", delete(drop_index))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_auth));

    Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/session", get(session))
        .merge(protected)
        .with_state(state)
}

async fn require_auth(
    State(state): State<SharedState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if is_authenticated(request.headers(), &state) {
        next.run(request).await
    } else {
        ApiError::new(StatusCode::UNAUTHORIZED, "Authentication required").into_response()
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorResponse {
                error: self.message,
            }),
        )
            .into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(value: sqlx::Error) -> Self {
        ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, value.to_string())
    }
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    password: String,
}

#[derive(Debug, Serialize)]
struct AuthResponse {
    authenticated: bool,
}

async fn login(
    State(state): State<SharedState>,
    Json(request): Json<LoginRequest>,
) -> Result<impl IntoResponse, ApiError> {
    if request.password != state.auth_password {
        return Err(ApiError::new(StatusCode::UNAUTHORIZED, "Invalid password"));
    }

    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&format!(
            "{AUTH_COOKIE}={}; Path=/; HttpOnly; SameSite=Lax",
            state.auth_token
        ))
        .map_err(|_| ApiError::new(StatusCode::INTERNAL_SERVER_ERROR, "Invalid cookie value"))?,
    );

    Ok((
        headers,
        Json(AuthResponse {
            authenticated: true,
        }),
    ))
}

async fn logout() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_static("sql_web_session=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0"),
    );

    (
        headers,
        Json(AuthResponse {
            authenticated: false,
        }),
    )
}

async fn session(State(state): State<SharedState>, headers: HeaderMap) -> Json<AuthResponse> {
    Json(AuthResponse {
        authenticated: is_authenticated(&headers, &state),
    })
}

#[derive(Debug, Serialize)]
struct OverviewResponse {
    database_stats: DatabaseStats,
    tables: Vec<String>,
    version: String,
}

async fn overview(State(state): State<SharedState>) -> Result<Json<OverviewResponse>, ApiError> {
    let manager = manager(&state);
    let database_info = manager.get_database_info().await?;
    let tables = manager.get_tables().await?;

    let database_stats = DatabaseStats {
        database_name: database_info.base_name(),
        database_type: format!("{:?}", database_info.database_type),
        file_size: database_info.size,
        table_count: tables.len(),
        index_count: 0,
        trigger_count: 0,
        view_count: 0,
        created: database_info.created,
        modified: database_info.modified,
        readonly: database_info.readonly,
    };

    Ok(Json(OverviewResponse {
        database_stats,
        tables,
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}

#[derive(Debug, Serialize)]
struct TablesResponse {
    tables: Vec<String>,
}

async fn tables(State(state): State<SharedState>) -> Result<Json<TablesResponse>, ApiError> {
    Ok(Json(TablesResponse {
        tables: manager(&state).get_tables().await?,
    }))
}

async fn table_structure(
    State(state): State<SharedState>,
    Path(table): Path<String>,
) -> Result<Json<TableStructure>, ApiError> {
    let manager = manager(&state);
    let table_info = manager.get_table_info(&table).await?;
    let indexes = manager.get_indexes(&table).await.unwrap_or_default();

    let columns = table_info
        .columns
        .into_iter()
        .map(|column| ColumnDetail {
            name: column.name,
            data_type: column.data_type,
            nullable: column.nullable,
            default_value: column.default_value,
            is_primary_key: column.is_primary_key,
            is_auto_increment: false,
            max_length: None,
        })
        .collect();

    let indexes = indexes
        .into_iter()
        .map(|index| IndexDetail {
            name: index.name,
            columns: index.columns,
            unique: index.unique,
            index_type: "BTREE".to_string(),
        })
        .collect();

    Ok(Json(TableStructure {
        name: table,
        columns,
        indexes,
        foreign_keys: vec![],
        triggers: vec![],
        create_sql: None,
    }))
}

#[derive(Debug, Deserialize)]
struct RowsParams {
    page: Option<usize>,
    per_page: Option<usize>,
}

async fn table_rows(
    State(state): State<SharedState>,
    Path(table): Path<String>,
    Query(params): Query<RowsParams>,
) -> Result<Json<crate::config::TableRows>, ApiError> {
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(state.args.rows_per_page);
    Ok(Json(
        manager(&state)
            .get_table_rows(&table, page, per_page)
            .await?,
    ))
}

async fn execute_query(
    State(state): State<SharedState>,
    Json(request): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, ApiError> {
    run_query(state, request).await
}

async fn execute_table_sql(
    State(state): State<SharedState>,
    Path(_table): Path<String>,
    Json(request): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, ApiError> {
    run_query(state, request).await
}

async fn run_query(
    state: SharedState,
    request: QueryRequest,
) -> Result<Json<QueryResponse>, ApiError> {
    let sql = request.sql.trim();
    if sql.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "SQL query cannot be empty",
        ));
    }

    if state.db_config.readonly && is_write_operation(sql) {
        return Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "Write operations are not allowed in read-only mode",
        ));
    }

    let page = request.page.unwrap_or(1);
    let per_page = request.per_page.unwrap_or(state.args.query_rows_per_page);
    let mut final_sql = sql.to_string();

    if let Some(ordering) = request.ordering {
        let direction = if ordering < 0 { "DESC" } else { "ASC" };
        final_sql = format!(
            "SELECT * FROM ({}) AS _ ORDER BY {} {}",
            sql.trim_end_matches(';'),
            ordering.abs(),
            direction
        );
    }

    let result = manager(&state).execute_query(&final_sql).await?;

    Ok(Json(QueryResponse {
        columns: result.columns,
        rows: result.rows,
        total_rows: None,
        page,
        per_page,
        total_pages: 1,
        error: None,
        rows_affected: result.rows_affected,
    }))
}

#[derive(Debug, Deserialize)]
struct InsertRowRequest {
    data: HashMap<String, Option<String>>,
}

async fn insert_row(
    State(state): State<SharedState>,
    Path(table): Path<String>,
    Json(request): Json<InsertRowRequest>,
) -> Result<Json<QueryResponse>, ApiError> {
    ensure_writable(&state)?;
    if request.data.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "No row data provided",
        ));
    }

    let config = &state.db_config;
    let columns: Vec<String> = request
        .data
        .keys()
        .map(|column| config.quote_identifier(column))
        .collect();
    let values: Vec<String> = request
        .data
        .values()
        .map(|value| optional_sql_value(value.as_ref()))
        .collect();
    let sql = format!(
        "INSERT INTO {} ({}) VALUES ({})",
        config.quote_identifier(&table),
        columns.join(", "),
        values.join(", ")
    );

    mutation_response(manager(&state).execute_query(&sql).await?)
}

#[derive(Debug, Deserialize)]
struct UpdateRowRequest {
    data: HashMap<String, Option<String>>,
    where_clause: HashMap<String, String>,
}

async fn update_row(
    State(state): State<SharedState>,
    Path(table): Path<String>,
    Json(request): Json<UpdateRowRequest>,
) -> Result<Json<QueryResponse>, ApiError> {
    ensure_writable(&state)?;
    if request.data.is_empty() || request.where_clause.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "Both row data and where_clause are required",
        ));
    }

    let config = &state.db_config;
    let set_clauses: Vec<String> = request
        .data
        .iter()
        .map(|(column, value)| {
            format!(
                "{} = {}",
                config.quote_identifier(column),
                optional_sql_value(value.as_ref())
            )
        })
        .collect();
    let where_clause = build_where_clause(config, &request.where_clause);
    let sql = format!(
        "UPDATE {} SET {} WHERE {}",
        config.quote_identifier(&table),
        set_clauses.join(", "),
        where_clause
    );

    mutation_response(manager(&state).execute_query(&sql).await?)
}

async fn add_column(
    State(state): State<SharedState>,
    Path(table): Path<String>,
    Json(request): Json<CreateColumnRequest>,
) -> Result<Json<QueryResponse>, ApiError> {
    ensure_writable(&state)?;

    let config = &state.db_config;
    let mut sql = format!(
        "ALTER TABLE {} ADD COLUMN {} {}",
        config.quote_identifier(&table),
        config.quote_identifier(&request.name),
        request.data_type
    );

    if !request.nullable {
        sql.push_str(" NOT NULL");
    }
    if let Some(default_value) = request
        .default_value
        .as_ref()
        .filter(|value| !value.is_empty())
    {
        sql.push_str(" DEFAULT ");
        sql.push_str(&escape_string_literal(default_value));
    }
    if request.primary_key {
        sql.push_str(" PRIMARY KEY");
    }
    if request.auto_increment {
        match config.database_type {
            DatabaseType::Mysql => sql.push_str(" AUTO_INCREMENT"),
            DatabaseType::Postgres => {}
            DatabaseType::Sqlite => sql.push_str(" AUTOINCREMENT"),
        }
    }

    mutation_response(manager(&state).execute_query(&sql).await?)
}

#[derive(Debug, Deserialize)]
struct RenameColumnRequest {
    new_name: String,
}

async fn rename_column(
    State(state): State<SharedState>,
    Path((table, column)): Path<(String, String)>,
    Json(request): Json<RenameColumnRequest>,
) -> Result<Json<QueryResponse>, ApiError> {
    ensure_writable(&state)?;

    let config = &state.db_config;
    let sql = format!(
        "ALTER TABLE {} RENAME COLUMN {} TO {}",
        config.quote_identifier(&table),
        config.quote_identifier(&column),
        config.quote_identifier(&request.new_name)
    );

    mutation_response(manager(&state).execute_query(&sql).await?)
}

async fn drop_column(
    State(state): State<SharedState>,
    Path((table, column)): Path<(String, String)>,
) -> Result<Json<QueryResponse>, ApiError> {
    ensure_writable(&state)?;

    let config = &state.db_config;
    let sql = format!(
        "ALTER TABLE {} DROP COLUMN {}",
        config.quote_identifier(&table),
        config.quote_identifier(&column)
    );

    mutation_response(manager(&state).execute_query(&sql).await?)
}

#[derive(Debug, Deserialize)]
struct CreateIndexBody {
    name: String,
    columns: Vec<String>,
    unique: bool,
}

async fn add_index(
    State(state): State<SharedState>,
    Path(table): Path<String>,
    Json(request): Json<CreateIndexBody>,
) -> Result<Json<QueryResponse>, ApiError> {
    ensure_writable(&state)?;
    if request.name.trim().is_empty() || request.columns.is_empty() {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "Index name and columns are required",
        ));
    }

    let config = &state.db_config;
    let unique = if request.unique { "UNIQUE " } else { "" };
    let columns = request
        .columns
        .iter()
        .map(|column| config.quote_identifier(column))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "CREATE {}INDEX {} ON {} ({})",
        unique,
        config.quote_identifier(&request.name),
        config.quote_identifier(&table),
        columns
    );

    mutation_response(manager(&state).execute_query(&sql).await?)
}

async fn drop_index(
    State(state): State<SharedState>,
    Path((table, index)): Path<(String, String)>,
) -> Result<Json<QueryResponse>, ApiError> {
    ensure_writable(&state)?;

    let config = &state.db_config;
    let sql = match config.database_type {
        DatabaseType::Mysql => format!(
            "DROP INDEX {} ON {}",
            config.quote_identifier(&index),
            config.quote_identifier(&table)
        ),
        DatabaseType::Sqlite | DatabaseType::Postgres => {
            format!("DROP INDEX {}", config.quote_identifier(&index))
        }
    };

    mutation_response(manager(&state).execute_query(&sql).await?)
}

fn manager(state: &Arc<AppState>) -> DatabaseManager<'_> {
    DatabaseManager::new(&state.pool, state.db_config.clone())
}

fn ensure_writable(state: &AppState) -> Result<(), ApiError> {
    if state.db_config.readonly {
        Err(ApiError::new(
            StatusCode::FORBIDDEN,
            "Write operations are not allowed in read-only mode",
        ))
    } else {
        Ok(())
    }
}

fn mutation_response(result: crate::config::QueryResult) -> Result<Json<QueryResponse>, ApiError> {
    Ok(Json(QueryResponse {
        columns: result.columns,
        rows: result.rows,
        total_rows: None,
        page: 1,
        per_page: 1,
        total_pages: 1,
        error: None,
        rows_affected: result.rows_affected,
    }))
}

fn build_where_clause(config: &DatabaseConfig, conditions: &HashMap<String, String>) -> String {
    conditions
        .iter()
        .map(|(column, value)| {
            format!(
                "{} = {}",
                config.quote_identifier(column),
                escape_string_literal(value)
            )
        })
        .collect::<Vec<_>>()
        .join(" AND ")
}

fn is_authenticated(headers: &HeaderMap, state: &AppState) -> bool {
    headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|cookie_header| {
            cookie_header.split(';').find_map(|cookie| {
                let (name, value) = cookie.trim().split_once('=')?;
                (name == AUTH_COOKIE).then_some(value)
            })
        })
        .is_some_and(|value| value == state.auth_token)
}
