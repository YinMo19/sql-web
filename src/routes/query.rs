use crate::config::{DatabaseConfig, DatabaseManager, DatabasePool};
use crate::models::{QueryRequest, QueryResponse};
use crate::routes::index::AuthGuard;

use crate::template::{IntoTemplateResponse, TemplateResponse};
use askama::Template;
use rocket::form::Form;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::{get, post, State};
use std::cmp;

#[derive(Template)]
#[template(path = "query.html")]
pub struct QueryTemplate {
    pub sql: String,
    pub has_result: bool,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub rows_affected_display: String,
    pub error: String,
    pub database_name: String,
}

impl QueryTemplate {
    pub fn new(
        sql: String,
        has_result: bool,
        columns: Vec<String>,
        rows: Vec<Vec<Option<String>>>,
        rows_affected: Option<u64>,
        error: String,
        database_name: String,
    ) -> Self {
        let rows_affected_display = match rows_affected {
            Some(count) => count.to_string(),
            None => "0".to_string(),
        };

        let processed_rows = rows
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|cell| cell.unwrap_or_else(|| "NULL".to_string()))
                    .collect()
            })
            .collect();

        Self {
            sql,
            has_result,
            columns,
            rows: processed_rows,
            rows_affected_display,
            error,
            database_name,
        }
    }
}

#[derive(rocket::FromForm)]
pub struct QueryForm {
    sql: String,
    page: Option<usize>,
    ordering: Option<i32>,
    export_json: Option<String>,
    export_csv: Option<String>,
}

#[get("/query")]
pub async fn query_page(
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<TemplateResponse<QueryTemplate>, Status> {
    let pool = &db.0;
    let manager = DatabaseManager::new(pool, config.inner().clone());

    let database_info = manager
        .get_database_info()
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(QueryTemplate::new(
        String::new(),
        false,
        vec![],
        vec![],
        None,
        String::new(),
        database_info.base_name(),
    )
    .into_template_response())
}

#[post("/query", data = "<form>")]
pub async fn execute_query(
    form: Form<QueryForm>,
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<TemplateResponse<QueryTemplate>, Status> {
    let pool = &db.0;
    let manager = DatabaseManager::new(pool, config.inner().clone());

    let database_info = manager
        .get_database_info()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let sql = form.sql.trim();
    if sql.is_empty() {
        return Ok(QueryTemplate::new(
            String::new(),
            false,
            vec![],
            vec![],
            None,
            "SQL query cannot be empty".to_string(),
            database_info.base_name(),
        )
        .into_template_response());
    }

    // Check if this is a read-only connection and the query is a write operation
    if config.readonly && is_write_operation(sql) {
        return Ok(QueryTemplate::new(
            sql.to_string(),
            false,
            vec![],
            vec![],
            None,
            "Write operations are not allowed in read-only mode".to_string(),
            database_info.base_name(),
        )
        .into_template_response());
    }

    let page = form.page.unwrap_or(1);
    let per_page = 1000; // Default query rows per page

    // Handle export requests
    if form.export_json.is_some() || form.export_csv.is_some() {
        // TODO: Implement export functionality
        // For now, fall through to regular query execution
    }

    // Apply ordering if specified
    let mut final_sql = sql.to_string();
    if let Some(ordering) = form.ordering {
        let direction = if ordering < 0 { "DESC" } else { "ASC" };
        final_sql = format!(
            "SELECT * FROM ({}) AS _ ORDER BY {} {}",
            sql.trim_end_matches(';'),
            ordering.abs(),
            direction
        );
    }

    // Apply pagination for SELECT queries
    let is_select = sql.trim_start().to_uppercase().starts_with("SELECT");
    let mut total_rows = None;

    if is_select && page > 1 {
        // Get total count for pagination
        let count_sql = format!(
            "SELECT COUNT(*) as count FROM ({}) AS _",
            sql.trim_end_matches(';')
        );
        match manager.execute_query(&count_sql).await {
            Ok(count_result) => {
                if let Some(first_row) = count_result.rows.first() {
                    if let Some(Some(count_str)) = first_row.first() {
                        total_rows = count_str.parse::<i64>().ok();
                    }
                }
            }
            Err(_) => {
                // If count fails, continue without pagination
            }
        }

        // Apply LIMIT and OFFSET
        let offset = (page - 1) * per_page;
        final_sql = format!(
            "SELECT * FROM ({}) AS _ LIMIT {} OFFSET {}",
            final_sql.trim_end_matches(';'),
            per_page,
            offset
        );
    }

    match manager.execute_query(&final_sql).await {
        Ok(query_result) => {
            let _total_pages = if let Some(total) = total_rows {
                cmp::max(1, (total as f64 / per_page as f64).ceil() as usize)
            } else {
                1
            };

            Ok(QueryTemplate::new(
                sql.to_string(),
                true,
                query_result.columns,
                query_result.rows,
                query_result.rows_affected,
                String::new(),
                database_info.base_name(),
            )
            .into_template_response())
        }
        Err(e) => Ok(QueryTemplate::new(
            sql.to_string(),
            false,
            vec![],
            vec![],
            None,
            format!("SQL Error: {}", e),
            database_info.base_name(),
        )
        .into_template_response()),
    }
}

#[post("/api/query", data = "<request>")]
pub async fn api_execute_query(
    request: Json<QueryRequest>,
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<Json<QueryResponse>, Status> {
    let pool = &db.0;
    let manager = DatabaseManager::new(pool, config.inner().clone());

    let sql = request.sql.trim();
    if sql.is_empty() {
        return Ok(Json(QueryResponse {
            columns: vec![],
            rows: vec![],
            total_rows: None,
            page: 1,
            per_page: 1000,
            total_pages: 1,
            error: Some("SQL query cannot be empty".to_string()),
            rows_affected: None,
        }));
    }

    // Check if this is a read-only connection and the query is a write operation
    if config.readonly && is_write_operation(sql) {
        return Ok(Json(QueryResponse {
            columns: vec![],
            rows: vec![],
            total_rows: None,
            page: 1,
            per_page: 1000,
            total_pages: 1,
            error: Some("Write operations are not allowed in read-only mode".to_string()),
            rows_affected: None,
        }));
    }

    let page = request.page.unwrap_or(1);
    let per_page = request.per_page.unwrap_or(1000);

    match manager.execute_query(sql).await {
        Ok(query_result) => {
            let total_pages = 1; // TODO: Implement proper pagination for API

            Ok(Json(QueryResponse {
                columns: query_result.columns,
                rows: query_result.rows,
                total_rows: None,
                page,
                per_page,
                total_pages,
                error: None,
                rows_affected: query_result.rows_affected,
            }))
        }
        Err(e) => Ok(Json(QueryResponse {
            columns: vec![],
            rows: vec![],
            total_rows: None,
            page,
            per_page,
            total_pages: 1,
            error: Some(format!("SQL Error: {}", e)),
            rows_affected: None,
        })),
    }
}

fn is_write_operation(sql: &str) -> bool {
    let sql_upper = sql.trim().to_uppercase();
    sql_upper.starts_with("INSERT")
        || sql_upper.starts_with("UPDATE")
        || sql_upper.starts_with("DELETE")
        || sql_upper.starts_with("DROP")
        || sql_upper.starts_with("CREATE")
        || sql_upper.starts_with("ALTER")
        || sql_upper.starts_with("TRUNCATE")
}
