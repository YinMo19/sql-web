use crate::config::{DatabaseConfig, DatabaseManager, DatabasePool};
use crate::models::{
    FlashMessage as Flash, PaginationInfo, TableData, TableStructure, TableStructureForTemplate,
};
use crate::routes::index::AuthGuard;
use crate::template::{IntoTemplateResponse, TemplateResponse};
use crate::Args;
use askama::Template;
use rocket::form::Form;
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::{get, post, State};
use std::collections::HashMap;

#[derive(Template)]
#[template(path = "table_list.html")]
pub struct TableListTemplate {
    pub tables: Vec<String>,
    pub database_name: String,
    pub readonly: bool,
    pub flash_messages: Vec<Flash>,
    pub version: String,
}

#[derive(Template)]
#[template(path = "table_structure.html")]
pub struct TableStructureTemplate {
    pub table: TableStructureForTemplate,
    pub database_name: String,
    pub readonly: bool,
    pub flash_messages: Vec<Flash>,
    pub version: String,
}

#[derive(Template)]
#[template(path = "table_content.html")]
pub struct TableContentTemplate {
    pub table_data: crate::models::TableDataForTemplate,
    pub pagination: PaginationInfo,
    pub database_name: String,
    pub readonly: bool,
    pub flash_messages: Vec<Flash>,
    pub version: String,
}

#[allow(unused)]
#[derive(Template)]
#[template(path = "table_query.html")]
pub struct TableQueryTemplate {
    pub table_name: String,
    pub sql: String,
    pub has_result: bool,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub rows_affected_display: String,
    pub total_rows: Option<i64>,
    pub error: String,
    pub database_name: String,
    pub readonly: bool,
    pub flash_messages: Vec<Flash>,
    pub version: String,
    pub has_columns: bool,
    pub has_rows: bool,
    pub has_total_rows: bool,
}

impl TableQueryTemplate {
    pub fn new(
        table_name: String,
        sql: String,
        has_result: bool,
        columns: Vec<String>,
        rows: Vec<Vec<Option<String>>>,
        rows_affected: Option<u64>,
        total_rows: Option<i64>,
        error: String,
        database_name: String,
        readonly: bool,
        flash_messages: Vec<Flash>,
        version: String,
    ) -> Self {
        let rows_affected_display = match rows_affected {
            Some(count) => count.to_string(),
            None => "0".to_string(),
        };

        let processed_rows: Vec<Vec<String>> = rows
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|cell| cell.unwrap_or_else(|| "NULL".to_string()))
                    .collect()
            })
            .collect();

        let has_columns = !columns.is_empty();
        let has_rows = !processed_rows.is_empty();
        let has_total_rows = total_rows.is_some();

        Self {
            table_name,
            sql,
            has_result,
            columns,
            rows: processed_rows,
            rows_affected_display,
            total_rows,
            error,
            database_name,
            readonly,
            flash_messages,
            version,
            has_columns,
            has_rows,
            has_total_rows,
        }
    }
}

#[derive(Template)]
#[template(path = "table_insert.html")]
pub struct TableInsertTemplate {
    pub table_name: String,
    pub columns: Vec<crate::config::ColumnInfo>,
    pub database_name: String,
    pub readonly: bool,
    pub flash_messages: Vec<Flash>,
    pub version: String,
    pub error: String,
}

#[derive(Template)]
#[template(path = "table_update.html")]
pub struct TableUpdateTemplate {
    pub table_name: String,
    pub columns: Vec<crate::config::ColumnInfo>,
    pub row_values: Vec<String>,
    pub primary_key_values: Vec<String>,
    pub database_name: String,
    pub readonly: bool,
    pub flash_messages: Vec<Flash>,
    pub version: String,
    pub error: String,
}

impl TableUpdateTemplate {
    pub fn new(
        table_name: String,
        columns: Vec<crate::config::ColumnInfo>,
        row_data: HashMap<String, Option<String>>,
        primary_key: HashMap<String, String>,
        database_name: String,
        readonly: bool,
        flash_messages: Vec<Flash>,
        version: String,
        error: String,
    ) -> Self {
        let row_values: Vec<String> = columns
            .iter()
            .map(|col| {
                row_data
                    .get(&col.name)
                    .and_then(|v| v.as_ref())
                    .unwrap_or(&"NULL".to_string())
                    .clone()
            })
            .collect();

        let primary_key_values: Vec<String> = columns
            .iter()
            .filter(|col| col.is_primary_key)
            .map(|col| {
                primary_key
                    .get(&col.name)
                    .unwrap_or(&"".to_string())
                    .clone()
            })
            .collect();

        Self {
            table_name,
            columns,
            row_values,
            primary_key_values,
            database_name,
            readonly,
            flash_messages,
            version,
            error,
        }
    }
}

#[derive(Template)]
#[template(path = "table_export.html")]
pub struct TableExportTemplate {
    pub table_name: String,
    pub columns: Vec<String>,
    pub database_name: String,
    pub readonly: bool,
    pub flash_messages: Vec<Flash>,
    pub version: String,
}

#[derive(Template)]
#[template(path = "table_import.html")]
pub struct TableImportTemplate {
    pub table_name: String,
    pub columns: Vec<String>,
    pub database_name: String,
    pub readonly: bool,
    pub flash_messages: Vec<Flash>,
    pub version: String,
    pub error: String,
}

#[derive(rocket::FromForm)]
pub struct QueryForm {
    sql: String,
}

#[derive(rocket::FromForm)]
pub struct InsertForm {
    #[field(name = "data")]
    data: HashMap<String, String>,
}

#[derive(rocket::FromForm)]
pub struct UpdateForm {
    #[field(name = "data")]
    data: HashMap<String, String>,
    #[field(name = "pk")]
    pk: HashMap<String, String>,
}

#[get("/tables")]
pub async fn table_list(
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<TemplateResponse<TableListTemplate>, Status> {
    let pool = &db.0;
    let manager = DatabaseManager::new(pool, config.inner().clone());

    let database_info = manager
        .get_database_info()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let tables = manager
        .get_tables()
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(TableListTemplate {
        tables,
        database_name: database_info.base_name(),
        readonly: database_info.readonly,
        flash_messages: vec![],
        version: "0.1.0".to_string(),
    }
    .into_template_response())
}

#[get("/table/<table_name>/structure")]
pub async fn table_structure(
    table_name: String,
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<TemplateResponse<TableStructureTemplate>, Status> {
    let pool = &db.0;
    let manager = DatabaseManager::new(pool, config.inner().clone());

    let database_info = manager
        .get_database_info()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let table_info = manager
        .get_table_info(&table_name)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let indexes = manager
        .get_indexes(&table_name)
        .await
        .unwrap_or_else(|_| vec![]);

    // Convert to models::TableStructure format
    let columns: Vec<crate::models::ColumnDetail> = table_info
        .columns
        .into_iter()
        .map(|col| crate::models::ColumnDetail {
            name: col.name,
            data_type: col.data_type,
            nullable: col.nullable,
            default_value: col.default_value,
            is_primary_key: col.is_primary_key,
            is_auto_increment: false, // TODO: Detect auto increment
            max_length: None,         // TODO: Extract max length from type
        })
        .collect();

    let index_details: Vec<crate::models::IndexDetail> = indexes
        .into_iter()
        .map(|idx| crate::models::IndexDetail {
            name: idx.name,
            columns: idx.columns,
            unique: idx.unique,
            index_type: "BTREE".to_string(), // Default type
        })
        .collect();

    let table = TableStructure {
        name: table_name,
        columns,
        indexes: index_details,
        foreign_keys: vec![], // TODO: Implement foreign key detection
        triggers: vec![],     // TODO: Implement trigger detection
        create_sql: None,     // TODO: Get CREATE TABLE statement
    };

    Ok(TableStructureTemplate {
        table: TableStructureForTemplate::from_table_structure(table),
        database_name: database_info.base_name(),
        readonly: database_info.readonly,
        flash_messages: vec![],
        version: "0.1.0".to_string(),
    }
    .into_template_response())
}

#[get("/table/<table_name>/content?<page>&<per_page>")]
pub async fn table_content(
    table_name: String,
    page: Option<usize>,
    per_page: Option<usize>,
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    args: &State<Args>,
    _auth: AuthGuard,
) -> Result<TemplateResponse<TableContentTemplate>, Status> {
    let pool = &db.0;
    let manager = DatabaseManager::new(pool, config.inner().clone());

    let database_info = manager
        .get_database_info()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let page = page.unwrap_or(1);
    let per_page = per_page.unwrap_or(args.rows_per_page);
    let offset = (page - 1) * per_page;

    // Get total row count
    let total_rows = manager
        .get_table_row_count(&table_name)
        .await
        .map_err(|_| Status::InternalServerError)?;

    // Get table data with pagination
    let sql = format!(
        "SELECT * FROM \"{}\" LIMIT {} OFFSET {}",
        table_name, per_page, offset
    );

    let query_result = manager
        .execute_query(&sql)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let table_data = TableData {
        name: table_name.clone(),
        columns: query_result.columns,
        rows: query_result.rows,
        total_rows,
        page,
        per_page,
        total_pages: ((total_rows as f64) / (per_page as f64)).ceil() as usize,
    };

    let pagination = PaginationInfo::new(page, per_page, total_rows);

    Ok(TableContentTemplate {
        table_data: crate::models::TableDataForTemplate::from_table_data(table_data),
        pagination,
        database_name: database_info.base_name(),
        readonly: database_info.readonly,
        flash_messages: vec![],
        version: "0.1.0".to_string(),
    }
    .into_template_response())
}

#[get("/table/<table_name>/query")]
pub async fn table_query(
    table_name: String,
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<TemplateResponse<TableQueryTemplate>, Status> {
    let pool = &db.0;
    let manager = DatabaseManager::new(pool, config.inner().clone());

    let database_info = manager
        .get_database_info()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let default_sql = format!("SELECT * FROM \"{}\" LIMIT 100;", table_name);

    Ok(TableQueryTemplate::new(
        table_name.clone(),
        default_sql,
        false,
        vec![],
        vec![],
        None,
        None,
        String::new(),
        database_info.base_name(),
        database_info.readonly,
        vec![],
        "0.1.0".to_string(),
    )
    .into_template_response())
}

#[post("/table/<table_name>/query", data = "<form>")]
pub async fn table_query_execute(
    table_name: String,
    form: Form<QueryForm>,
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<TemplateResponse<TableQueryTemplate>, Status> {
    let pool = &db.0;
    let manager = DatabaseManager::new(pool, config.inner().clone());

    let database_info = manager
        .get_database_info()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let sql = form.sql.trim();
    if sql.is_empty() {
        return Ok(TableQueryTemplate::new(
            table_name.clone(),
            String::new(),
            false,
            vec![],
            vec![],
            None,
            None,
            "SQL query cannot be empty".to_string(),
            database_info.base_name(),
            database_info.readonly,
            vec![],
            "0.1.0".to_string(),
        )
        .into_template_response());
    }

    match manager.execute_query(sql).await {
        Ok(query_result) => Ok(TableQueryTemplate::new(
            table_name.clone(),
            sql.to_string(),
            true,
            query_result.columns,
            query_result.rows,
            query_result.rows_affected,
            None,
            String::new(),
            database_info.base_name(),
            database_info.readonly,
            vec![],
            "0.1.0".to_string(),
        )
        .into_template_response()),
        Err(e) => Ok(TableQueryTemplate::new(
            table_name.clone(),
            sql.to_string(),
            false,
            vec![],
            vec![],
            None,
            None,
            format!("SQL Error: {}", e),
            database_info.base_name(),
            database_info.readonly,
            vec![],
            "0.1.0".to_string(),
        )
        .into_template_response()),
    }
}

#[get("/table/<table_name>/insert")]
pub async fn table_insert(
    table_name: String,
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<TemplateResponse<TableInsertTemplate>, Status> {
    if config.readonly {
        return Err(Status::Forbidden);
    }

    let pool = &db.0;
    let manager = DatabaseManager::new(pool, config.inner().clone());

    let database_info = manager
        .get_database_info()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let table_info = manager
        .get_table_info(&table_name)
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(TableInsertTemplate {
        table_name: table_name.clone(),
        columns: table_info.columns,
        database_name: database_info.base_name(),
        readonly: database_info.readonly,
        flash_messages: vec![],
        version: "0.1.0".to_string(),
        error: String::new(),
    }
    .into_template_response())
}

#[post("/table/<table_name>/insert", data = "<form>")]
pub async fn table_insert_execute(
    table_name: String,
    form: Form<InsertForm>,
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<Redirect, Status> {
    if config.readonly {
        return Err(Status::Forbidden);
    }

    let pool = &db.0;
    let manager = DatabaseManager::new(pool, config.inner().clone());

    // Build INSERT statement
    let columns: Vec<String> = form.data.keys().cloned().collect();
    let values: Vec<String> = form
        .data
        .values()
        .map(|v| {
            if v.is_empty() {
                "NULL".to_string()
            } else {
                format!("'{}'", v.replace("'", "''"))
            }
        })
        .collect();

    let sql = format!(
        "INSERT INTO \"{}\" ({}) VALUES ({})",
        table_name,
        columns
            .iter()
            .map(|c| format!("\"{}\"", c))
            .collect::<Vec<_>>()
            .join(", "),
        values.join(", ")
    );

    match manager.execute_query(&sql).await {
        Ok(_) => Ok(Redirect::to(format!("/table/{}/content", table_name))),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[get("/table/<table_name>/update?<pk..>")]
pub async fn table_update(
    table_name: String,
    pk: HashMap<String, String>,
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<TemplateResponse<TableUpdateTemplate>, Status> {
    if config.readonly {
        return Err(Status::Forbidden);
    }

    let pool = &db.0;
    let manager = DatabaseManager::new(pool, config.inner().clone());

    let database_info = manager
        .get_database_info()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let table_info = manager
        .get_table_info(&table_name)
        .await
        .map_err(|_| Status::InternalServerError)?;

    // Get current row data
    let mut where_conditions = vec![];
    for (key, value) in &pk {
        where_conditions.push(format!("\"{}\" = '{}'", key, value.replace("'", "''")));
    }

    let sql = format!(
        "SELECT * FROM \"{}\" WHERE {}",
        table_name,
        where_conditions.join(" AND ")
    );

    let query_result = manager
        .execute_query(&sql)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let mut row_data = HashMap::new();
    if let Some(row) = query_result.rows.first() {
        for (i, column) in query_result.columns.iter().enumerate() {
            row_data.insert(column.clone(), row.get(i).unwrap_or(&None).clone());
        }
    }

    Ok(TableUpdateTemplate::new(
        table_name.clone(),
        table_info.columns,
        row_data,
        pk,
        database_info.base_name(),
        database_info.readonly,
        vec![],
        "0.1.0".to_string(),
        String::new(),
    )
    .into_template_response())
}

#[post("/table/<table_name>/update", data = "<form>")]
pub async fn table_update_execute(
    table_name: String,
    form: Form<UpdateForm>,
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<Redirect, Status> {
    if config.readonly {
        return Err(Status::Forbidden);
    }

    let pool = &db.0;
    let manager = DatabaseManager::new(pool, config.inner().clone());

    // Build UPDATE statement
    let set_clauses: Vec<String> = form
        .data
        .iter()
        .map(|(key, value)| {
            if value.is_empty() {
                format!("\"{}\" = NULL", key)
            } else {
                format!("\"{}\" = '{}'", key, value.replace("'", "''"))
            }
        })
        .collect();

    let mut where_conditions = vec![];
    for (key, value) in &form.pk {
        where_conditions.push(format!("\"{}\" = '{}'", key, value.replace("'", "''")));
    }

    let sql = format!(
        "UPDATE \"{}\" SET {} WHERE {}",
        table_name,
        set_clauses.join(", "),
        where_conditions.join(" AND ")
    );

    match manager.execute_query(&sql).await {
        Ok(_) => Ok(Redirect::to(format!("/table/{}/content", table_name))),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[get("/table/<table_name>/export")]
pub async fn table_export(
    table_name: String,
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<TemplateResponse<TableExportTemplate>, Status> {
    let pool = &db.0;
    let manager = DatabaseManager::new(pool, config.inner().clone());

    let database_info = manager
        .get_database_info()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let table_info = manager
        .get_table_info(&table_name)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let columns: Vec<String> = table_info.columns.iter().map(|c| c.name.clone()).collect();

    Ok(TableExportTemplate {
        table_name: table_name.clone(),
        columns,
        database_name: database_info.base_name(),
        readonly: database_info.readonly,
        flash_messages: vec![],
        version: "0.1.0".to_string(),
    }
    .into_template_response())
}

#[get("/table/<table_name>/import")]
pub async fn table_import(
    table_name: String,
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<TemplateResponse<TableImportTemplate>, Status> {
    if config.readonly {
        return Err(Status::Forbidden);
    }

    let pool = &db.0;
    let manager = DatabaseManager::new(pool, config.inner().clone());

    let database_info = manager
        .get_database_info()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let table_info = manager
        .get_table_info(&table_name)
        .await
        .map_err(|_| Status::InternalServerError)?;

    let columns: Vec<String> = table_info.columns.iter().map(|c| c.name.clone()).collect();

    Ok(TableImportTemplate {
        table_name: table_name.clone(),
        columns,
        database_name: database_info.base_name(),
        readonly: database_info.readonly,
        flash_messages: vec![],
        version: "0.1.0".to_string(),
        error: String::new(),
    }
    .into_template_response())
}
