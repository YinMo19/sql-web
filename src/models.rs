use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableData {
    pub name: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
    pub total_rows: i64,
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDataForTemplate {
    pub name: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub total_rows: i64,
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
}

impl TableDataForTemplate {
    pub fn from_table_data(data: TableData) -> Self {
        let processed_rows = data
            .rows
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|cell| cell.unwrap_or_else(|| "NULL".to_string()))
                    .collect()
            })
            .collect();

        Self {
            name: data.name,
            columns: data.columns,
            rows: processed_rows,
            total_rows: data.total_rows,
            page: data.page,
            per_page: data.per_page,
            total_pages: data.total_pages,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    pub sql: String,
    pub page: Option<usize>,
    pub per_page: Option<usize>,
    pub ordering: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponse {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
    pub total_rows: Option<i64>,
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
    pub error: Option<String>,
    pub rows_affected: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResponseForTemplate {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub total_rows: Option<i64>,
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
    pub error: String,
    pub rows_affected_display: String,
}

impl QueryResponseForTemplate {
    #[allow(unused)]
    pub fn from_query_response(response: QueryResponse) -> Self {
        let processed_rows = response
            .rows
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|cell| cell.unwrap_or_else(|| "NULL".to_string()))
                    .collect()
            })
            .collect();

        let rows_affected_display = match response.rows_affected {
            Some(count) => count.to_string(),
            None => "0".to_string(),
        };

        let error = response.error.unwrap_or_else(|| String::new());

        Self {
            columns: response.columns,
            rows: processed_rows,
            total_rows: response.total_rows,
            page: response.page,
            per_page: response.per_page,
            total_pages: response.total_pages,
            error,
            rows_affected_display,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStructure {
    pub name: String,
    pub columns: Vec<ColumnDetail>,
    pub indexes: Vec<IndexDetail>,
    pub foreign_keys: Vec<ForeignKeyDetail>,
    pub triggers: Vec<TriggerDetail>,
    pub create_sql: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStructureForTemplate {
    pub name: String,
    pub columns: Vec<ColumnDetailForTemplate>,
    pub indexes: Vec<IndexDetail>,
    pub foreign_keys: Vec<ForeignKeyDetail>,
    pub triggers: Vec<TriggerDetail>,
    pub create_sql: String,
}

impl TableStructureForTemplate {
    pub fn from_table_structure(structure: TableStructure) -> Self {
        let processed_columns = structure
            .columns
            .into_iter()
            .map(ColumnDetailForTemplate::from_column_detail)
            .collect();

        Self {
            name: structure.name,
            columns: processed_columns,
            indexes: structure.indexes,
            foreign_keys: structure.foreign_keys,
            triggers: structure.triggers,
            create_sql: structure.create_sql.unwrap_or_else(|| String::new()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDetail {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub is_primary_key: bool,
    pub is_auto_increment: bool,
    pub max_length: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDetailForTemplate {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: String,
    pub has_default: bool,
    pub is_primary_key: bool,
    pub is_auto_increment: bool,
    pub max_length: i32,
    pub has_max_length: bool,
}

impl ColumnDetailForTemplate {
    pub fn from_column_detail(detail: ColumnDetail) -> Self {
        let has_default = detail.default_value.is_some();
        let default_value = detail.default_value.unwrap_or_else(|| String::new());
        let has_max_length = detail.max_length.is_some();
        let max_length = detail.max_length.unwrap_or(0);

        Self {
            name: detail.name,
            data_type: detail.data_type,
            nullable: detail.nullable,
            default_value,
            has_default,
            is_primary_key: detail.is_primary_key,
            is_auto_increment: detail.is_auto_increment,
            max_length,
            has_max_length,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDetail {
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
    pub index_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForeignKeyDetail {
    pub name: String,
    pub column: String,
    pub referenced_table: String,
    pub referenced_column: String,
    pub on_delete: String,
    pub on_update: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerDetail {
    pub name: String,
    pub event: String,
    pub timing: String,
    pub table_name: String,
    pub definition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertRequest {
    pub table: String,
    pub data: HashMap<String, Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRequest {
    pub table: String,
    pub data: HashMap<String, Option<String>>,
    pub where_clause: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRequest {
    pub table: String,
    pub where_clause: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTableRequest {
    pub name: String,
    pub columns: Vec<CreateColumnRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateColumnRequest {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub primary_key: bool,
    pub auto_increment: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddColumnRequest {
    pub table: String,
    pub column: CreateColumnRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropColumnRequest {
    pub table: String,
    pub column: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenameColumnRequest {
    pub table: String,
    pub old_name: String,
    pub new_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateIndexRequest {
    pub table: String,
    pub name: String,
    pub columns: Vec<String>,
    pub unique: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropIndexRequest {
    pub table: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRequest {
    pub table: String,
    pub format: String, // "json" or "csv"
    pub columns: Option<Vec<String>>,
    pub where_clause: Option<String>,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRequest {
    pub table: String,
    pub format: String, // "json" or "csv"
    pub data: String,
    pub create_columns: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
    pub total_rows: i64,
    pub has_prev: bool,
    pub has_next: bool,
    pub prev_page: usize,
    pub next_page: usize,
}

impl PaginationInfo {
    pub fn new(page: usize, per_page: usize, total_rows: i64) -> Self {
        let total_pages = if total_rows == 0 {
            1
        } else {
            ((total_rows as f64) / (per_page as f64)).ceil() as usize
        };

        let has_prev = page > 1;
        let has_next = page < total_pages;
        let prev_page = if has_prev { page - 1 } else { 1 };
        let next_page = if has_next { page + 1 } else { total_pages };

        Self {
            page,
            per_page,
            total_pages,
            total_rows,
            has_prev,
            has_next,
            prev_page,
            next_page,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashMessage {
    pub category: String, // "success", "danger", "warning", "info"
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub database_name: String,
    pub database_type: String,
    pub file_size: Option<u64>,
    pub table_count: usize,
    pub index_count: usize,
    pub trigger_count: usize,
    pub view_count: usize,
    pub created: Option<chrono::DateTime<chrono::Utc>>,
    pub modified: Option<chrono::DateTime<chrono::Utc>>,
    pub readonly: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewInfo {
    pub name: String,
    pub definition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub database_url: String,
    pub database_type: String,
    pub readonly: bool,
    pub connected: bool,
    pub version: Option<String>,
}

// Helper function to format file size
pub fn format_file_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if size == 0 {
        return "0 B".to_string();
    }

    let base = 1024_f64;
    let log = (size as f64).log(base).floor() as usize;
    let unit_index = log.min(UNITS.len() - 1);
    let value = size as f64 / base.powi(unit_index as i32);

    format!("{:.1} {}", value, UNITS[unit_index])
}

// Helper function to escape SQL identifiers
pub fn escape_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace("\"", "\"\""))
}

// Helper function to escape SQL string literals
pub fn escape_string_literal(value: &str) -> String {
    format!("'{}'", value.replace("'", "''"))
}

// Helper function to build WHERE clause from HashMap
pub fn build_where_clause(conditions: &HashMap<String, String>) -> String {
    if conditions.is_empty() {
        return String::new();
    }

    let clauses: Vec<String> = conditions
        .iter()
        .map(|(key, value)| {
            format!(
                "{} = {}",
                escape_identifier(key),
                escape_string_literal(value)
            )
        })
        .collect();

    format!("WHERE {}", clauses.join(" AND "))
}
