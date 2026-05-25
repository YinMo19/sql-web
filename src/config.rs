use serde::{Deserialize, Serialize};
use sqlx::{AnyPool, Column, Row};
use std::collections::BTreeMap;
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub database_type: DatabaseType,
    pub readonly: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseType {
    Sqlite,
    Mysql,
    Postgres,
}

impl DatabaseConfig {
    pub fn from_url(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let parsed_url = Url::parse(url)?;

        let database_type = match parsed_url.scheme() {
            "sqlite" => DatabaseType::Sqlite,
            "mysql" => DatabaseType::Mysql,
            "postgres" | "postgresql" => DatabaseType::Postgres,
            scheme => return Err(format!("Unsupported database scheme: {scheme}").into()),
        };

        let readonly = parsed_url
            .query_pairs()
            .any(|(key, value)| key == "mode" && value == "ro");

        Ok(DatabaseConfig {
            url: url.to_string(),
            database_type,
            readonly,
        })
    }

    pub fn quote_identifier(&self, identifier: &str) -> String {
        match self.database_type {
            DatabaseType::Mysql => format!("`{}`", identifier.replace('`', "``")),
            DatabaseType::Sqlite | DatabaseType::Postgres => {
                format!("\"{}\"", identifier.replace('"', "\"\""))
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DatabaseInfo {
    pub filename: Option<String>,
    pub size: Option<u64>,
    pub created: Option<chrono::DateTime<chrono::Utc>>,
    pub modified: Option<chrono::DateTime<chrono::Utc>>,
    pub readonly: bool,
    pub database_type: DatabaseType,
}

impl DatabaseInfo {
    pub fn base_name(&self) -> String {
        match &self.filename {
            Some(path) => std::path::Path::new(path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            None => "database".to_string(),
        }
    }
}

pub struct DatabaseManager<'a> {
    pool: &'a AnyPool,
    pub config: DatabaseConfig,
}

impl<'a> DatabaseManager<'a> {
    pub fn new(pool: &'a AnyPool, config: DatabaseConfig) -> Self {
        Self { pool, config }
    }

    pub async fn get_database_info(&self) -> Result<DatabaseInfo, sqlx::Error> {
        match self.config.database_type {
            DatabaseType::Sqlite => self.get_sqlite_info().await,
            DatabaseType::Mysql => self.get_remote_info().await,
            DatabaseType::Postgres => self.get_remote_info().await,
        }
    }

    async fn get_sqlite_info(&self) -> Result<DatabaseInfo, sqlx::Error> {
        let filename = if let Ok(url) = Url::parse(&self.config.url) {
            url.path().to_string()
        } else {
            self.config.url.clone()
        };

        let (size, created, modified) = if let Ok(metadata) = std::fs::metadata(&filename) {
            let created = metadata.created().ok().and_then(|t| {
                chrono::DateTime::from_timestamp(
                    t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64,
                    0,
                )
            });
            let modified = metadata.modified().ok().and_then(|t| {
                chrono::DateTime::from_timestamp(
                    t.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs() as i64,
                    0,
                )
            });
            (Some(metadata.len()), created, modified)
        } else {
            (None, None, None)
        };

        Ok(DatabaseInfo {
            filename: Some(filename),
            size,
            created,
            modified,
            readonly: self.config.readonly,
            database_type: self.config.database_type.clone(),
        })
    }

    async fn get_remote_info(&self) -> Result<DatabaseInfo, sqlx::Error> {
        Ok(DatabaseInfo {
            filename: None,
            size: None,
            created: None,
            modified: None,
            readonly: self.config.readonly,
            database_type: self.config.database_type.clone(),
        })
    }

    pub async fn get_tables(&self) -> Result<Vec<String>, sqlx::Error> {
        match self.config.database_type {
            DatabaseType::Sqlite => {
                let rows = sqlx::query(
                    "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
                )
                .fetch_all(self.pool)
                .await?;
                rows.into_iter().map(|row| row.try_get("name")).collect()
            }
            DatabaseType::Mysql => {
                let rows = sqlx::query("SHOW TABLES").fetch_all(self.pool).await?;
                let mut tables = Vec::new();
                for row in rows {
                    if let Some(column) = row.columns().first() {
                        tables.push(row.try_get(column.name())?);
                    }
                }
                Ok(tables)
            }
            DatabaseType::Postgres => {
                let rows = sqlx::query(
                    "SELECT tablename FROM pg_tables WHERE schemaname = 'public' ORDER BY tablename",
                )
                .fetch_all(self.pool)
                .await?;
                rows.into_iter()
                    .map(|row| row.try_get("tablename"))
                    .collect()
            }
        }
    }

    pub async fn get_table_info(&self, table_name: &str) -> Result<TableInfo, sqlx::Error> {
        match self.config.database_type {
            DatabaseType::Sqlite => self.get_sqlite_table_info(table_name).await,
            DatabaseType::Mysql => self.get_mysql_table_info(table_name).await,
            DatabaseType::Postgres => self.get_postgres_table_info(table_name).await,
        }
    }

    async fn get_sqlite_table_info(&self, table_name: &str) -> Result<TableInfo, sqlx::Error> {
        let sql = format!(
            "PRAGMA table_info({})",
            self.config.quote_identifier(table_name)
        );
        let rows = sqlx::query(&sql).fetch_all(self.pool).await?;

        let mut columns = Vec::new();
        for row in rows {
            columns.push(ColumnInfo {
                name: row.try_get("name")?,
                data_type: row.try_get("type")?,
                nullable: row.try_get::<i32, _>("notnull")? == 0,
                default_value: row.try_get("dflt_value").ok(),
                is_primary_key: row.try_get::<i32, _>("pk")? != 0,
            });
        }

        Ok(TableInfo {
            name: table_name.to_string(),
            columns,
        })
    }

    async fn get_mysql_table_info(&self, table_name: &str) -> Result<TableInfo, sqlx::Error> {
        let sql = format!("DESCRIBE {}", self.config.quote_identifier(table_name));
        let rows = sqlx::query(&sql).fetch_all(self.pool).await?;

        let mut columns = Vec::new();
        for row in rows {
            columns.push(ColumnInfo {
                name: row.try_get("Field")?,
                data_type: row.try_get("Type")?,
                nullable: row
                    .try_get::<String, _>("Null")?
                    .eq_ignore_ascii_case("YES"),
                default_value: row.try_get("Default").ok(),
                is_primary_key: row.try_get::<String, _>("Key")? == "PRI",
            });
        }

        Ok(TableInfo {
            name: table_name.to_string(),
            columns,
        })
    }

    async fn get_postgres_table_info(&self, table_name: &str) -> Result<TableInfo, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT
                c.column_name,
                c.data_type,
                c.is_nullable,
                c.column_default,
                CASE WHEN pk.column_name IS NOT NULL THEN true ELSE false END as is_primary_key
            FROM information_schema.columns c
            LEFT JOIN (
                SELECT ku.column_name
                FROM information_schema.table_constraints tc
                JOIN information_schema.key_column_usage ku
                    ON tc.constraint_name = ku.constraint_name
                   AND tc.table_schema = ku.table_schema
                WHERE tc.constraint_type = 'PRIMARY KEY'
                    AND tc.table_schema = 'public'
                    AND tc.table_name = $1
            ) pk ON c.column_name = pk.column_name
            WHERE c.table_schema = 'public' AND c.table_name = $1
            ORDER BY c.ordinal_position
            "#,
        )
        .bind(table_name)
        .fetch_all(self.pool)
        .await?;

        let mut columns = Vec::new();
        for row in rows {
            columns.push(ColumnInfo {
                name: row.try_get("column_name")?,
                data_type: row.try_get("data_type")?,
                nullable: row
                    .try_get::<String, _>("is_nullable")?
                    .eq_ignore_ascii_case("YES"),
                default_value: row.try_get("column_default").ok(),
                is_primary_key: row.try_get("is_primary_key")?,
            });
        }

        Ok(TableInfo {
            name: table_name.to_string(),
            columns,
        })
    }

    pub async fn execute_query(&self, sql: &str) -> Result<QueryResult, sqlx::Error> {
        if returns_rows(sql) {
            let rows = sqlx::query(sql).fetch_all(self.pool).await?;
            let columns = rows
                .first()
                .map(|row| {
                    row.columns()
                        .iter()
                        .map(|col| col.name().to_string())
                        .collect()
                })
                .unwrap_or_default();

            let mut result_rows = Vec::new();
            for row in rows {
                let mut row_data = Vec::new();
                for i in 0..row.columns().len() {
                    row_data.push(any_cell_to_string(&row, i));
                }
                result_rows.push(row_data);
            }

            Ok(QueryResult {
                columns,
                rows: result_rows,
                rows_affected: None,
            })
        } else {
            let result = sqlx::query(sql).execute(self.pool).await?;
            Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                rows_affected: Some(result.rows_affected()),
            })
        }
    }

    pub async fn get_table_row_count(&self, table_name: &str) -> Result<i64, sqlx::Error> {
        let sql = format!(
            "SELECT COUNT(*) as count FROM {}",
            self.config.quote_identifier(table_name)
        );
        let row = sqlx::query(&sql).fetch_one(self.pool).await?;
        row.try_get("count")
    }

    pub async fn get_table_rows(
        &self,
        table_name: &str,
        page: usize,
        per_page: usize,
    ) -> Result<TableRows, sqlx::Error> {
        let page = page.max(1);
        let per_page = per_page.max(1);
        let offset = (page - 1) * per_page;
        let total_rows = self.get_table_row_count(table_name).await?;
        let total_pages = if total_rows == 0 {
            1
        } else {
            ((total_rows as f64) / (per_page as f64)).ceil() as usize
        };

        let sql = format!(
            "SELECT * FROM {} LIMIT {} OFFSET {}",
            self.config.quote_identifier(table_name),
            per_page,
            offset
        );
        let query_result = self.execute_query(&sql).await?;

        Ok(TableRows {
            name: table_name.to_string(),
            columns: query_result.columns,
            rows: query_result.rows,
            total_rows,
            page,
            per_page,
            total_pages,
        })
    }

    pub async fn get_indexes(&self, table_name: &str) -> Result<Vec<IndexInfo>, sqlx::Error> {
        match self.config.database_type {
            DatabaseType::Sqlite => self.get_sqlite_indexes(table_name).await,
            DatabaseType::Mysql => self.get_mysql_indexes(table_name).await,
            DatabaseType::Postgres => self.get_postgres_indexes(table_name).await,
        }
    }

    async fn get_sqlite_indexes(&self, table_name: &str) -> Result<Vec<IndexInfo>, sqlx::Error> {
        let sql = format!(
            "PRAGMA index_list({})",
            self.config.quote_identifier(table_name)
        );
        let rows = sqlx::query(&sql).fetch_all(self.pool).await?;

        let mut indexes = Vec::new();
        for row in rows {
            let name: String = row.try_get("name")?;
            let unique: i32 = row.try_get("unique")?;
            let column_sql = format!("PRAGMA index_info({})", self.config.quote_identifier(&name));
            let column_rows = sqlx::query(&column_sql).fetch_all(self.pool).await?;
            let mut columns = Vec::new();
            for column_row in column_rows {
                columns.push(column_row.try_get("name")?);
            }

            indexes.push(IndexInfo {
                name,
                unique: unique != 0,
                columns,
            });
        }
        Ok(indexes)
    }

    async fn get_mysql_indexes(&self, table_name: &str) -> Result<Vec<IndexInfo>, sqlx::Error> {
        let sql = format!(
            "SHOW INDEX FROM {}",
            self.config.quote_identifier(table_name)
        );
        let rows = sqlx::query(&sql).fetch_all(self.pool).await?;
        let mut map: BTreeMap<String, IndexInfo> = BTreeMap::new();

        for row in rows {
            let name: String = row.try_get("Key_name")?;
            let column: String = row.try_get("Column_name")?;
            let non_unique: i64 = row.try_get("Non_unique")?;
            let entry = map.entry(name.clone()).or_insert(IndexInfo {
                name,
                unique: non_unique == 0,
                columns: vec![],
            });
            entry.columns.push(column);
        }

        Ok(map.into_values().collect())
    }

    async fn get_postgres_indexes(&self, table_name: &str) -> Result<Vec<IndexInfo>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT indexname, indexdef FROM pg_indexes WHERE schemaname = 'public' AND tablename = $1 ORDER BY indexname",
        )
        .bind(table_name)
        .fetch_all(self.pool)
        .await?;

        let mut indexes = Vec::new();
        for row in rows {
            let name: String = row.try_get("indexname")?;
            let definition: String = row.try_get("indexdef")?;
            indexes.push(IndexInfo {
                name,
                unique: definition.to_uppercase().contains("CREATE UNIQUE INDEX"),
                columns: extract_index_columns(&definition),
            });
        }
        Ok(indexes)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TableInfo {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub is_primary_key: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct IndexInfo {
    pub name: String,
    pub unique: bool,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
    pub rows_affected: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TableRows {
    pub name: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
    pub total_rows: i64,
    pub page: usize,
    pub per_page: usize,
    pub total_pages: usize,
}

pub fn escape_string_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

pub fn optional_sql_value(value: Option<&String>) -> String {
    match value {
        Some(value) if !value.is_empty() => escape_string_literal(value),
        _ => "NULL".to_string(),
    }
}

pub fn is_write_operation(sql: &str) -> bool {
    let sql_upper = sql.trim_start().to_uppercase();
    sql_upper.starts_with("INSERT")
        || sql_upper.starts_with("UPDATE")
        || sql_upper.starts_with("DELETE")
        || sql_upper.starts_with("DROP")
        || sql_upper.starts_with("CREATE")
        || sql_upper.starts_with("ALTER")
        || sql_upper.starts_with("TRUNCATE")
}

fn returns_rows(sql: &str) -> bool {
    let sql_upper = sql.trim_start().to_uppercase();
    sql_upper.starts_with("SELECT")
        || sql_upper.starts_with("WITH")
        || sql_upper.starts_with("SHOW")
        || sql_upper.starts_with("DESCRIBE")
        || sql_upper.starts_with("PRAGMA")
}

fn any_cell_to_string(row: &sqlx::any::AnyRow, index: usize) -> Option<String> {
    row.try_get::<Option<String>, _>(index)
        .ok()
        .flatten()
        .or_else(|| {
            row.try_get::<Option<i64>, _>(index)
                .ok()
                .flatten()
                .map(|v| v.to_string())
        })
        .or_else(|| {
            row.try_get::<Option<i32>, _>(index)
                .ok()
                .flatten()
                .map(|v| v.to_string())
        })
        .or_else(|| {
            row.try_get::<Option<f64>, _>(index)
                .ok()
                .flatten()
                .map(|v| v.to_string())
        })
        .or_else(|| {
            row.try_get::<Option<f32>, _>(index)
                .ok()
                .flatten()
                .map(|v| v.to_string())
        })
        .or_else(|| {
            row.try_get::<Option<bool>, _>(index)
                .ok()
                .flatten()
                .map(|v| v.to_string())
        })
}

fn extract_index_columns(definition: &str) -> Vec<String> {
    definition
        .rsplit_once('(')
        .and_then(|(_, rest)| rest.split_once(')'))
        .map(|(columns, _)| {
            columns
                .split(',')
                .map(|column| column.trim().trim_matches('"').to_string())
                .collect()
        })
        .unwrap_or_default()
}
