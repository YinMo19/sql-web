use rocket::fairing::AdHoc;
use serde::{Deserialize, Serialize};
use sqlx::{AnyPool, Column, Row};
use url::Url;

pub struct DatabasePool(pub AnyPool);

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
            scheme => return Err(format!("Unsupported database scheme: {}", scheme).into()),
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

    #[allow(unused)]
    pub fn get_connection_url(&self) -> String {
        self.url.clone()
    }
}

impl DatabasePool {
    pub fn init() -> AdHoc {
        AdHoc::on_ignite("SQLx Database", |rocket| async {
            // Install default drivers for AnyPool
            sqlx::any::install_default_drivers();

            let config = rocket
                .state::<DatabaseConfig>()
                .expect("DatabaseConfig not managed");

            let pool = AnyPool::connect(&config.url)
                .await
                .expect("Failed to connect to database");

            rocket.manage(DatabasePool(pool))
        })
    }
}

#[derive(Debug, Clone)]
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
    config: DatabaseConfig,
}

impl<'a> DatabaseManager<'a> {
    pub fn new(pool: &'a AnyPool, config: DatabaseConfig) -> Self {
        Self { pool, config }
    }

    pub async fn get_database_info(&self) -> Result<DatabaseInfo, sqlx::Error> {
        match self.config.database_type {
            DatabaseType::Sqlite => self.get_sqlite_info().await,
            DatabaseType::Mysql => self.get_mysql_info().await,
            DatabaseType::Postgres => self.get_postgres_info().await,
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

    async fn get_mysql_info(&self) -> Result<DatabaseInfo, sqlx::Error> {
        Ok(DatabaseInfo {
            filename: None,
            size: None,
            created: None,
            modified: None,
            readonly: self.config.readonly,
            database_type: self.config.database_type.clone(),
        })
    }

    async fn get_postgres_info(&self) -> Result<DatabaseInfo, sqlx::Error> {
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
                    "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
                )
                .fetch_all(self.pool)
                .await?;

                let mut tables = Vec::new();
                for row in rows {
                    let name: String = row.try_get("name")?;
                    tables.push(name);
                }
                Ok(tables)
            }
            DatabaseType::Mysql => {
                let rows = sqlx::query("SHOW TABLES").fetch_all(self.pool).await?;

                let mut tables = Vec::new();
                for row in rows {
                    // MySQL SHOW TABLES returns a single column, but the column name varies
                    // by database name, so we get the first column
                    if let Some(column) = row.columns().first() {
                        let name: String = row.try_get(column.name())?;
                        tables.push(name);
                    }
                }
                Ok(tables)
            }
            DatabaseType::Postgres => {
                let rows = sqlx::query(
                    "SELECT tablename FROM pg_tables WHERE schemaname = 'public' ORDER BY tablename"
                )
                .fetch_all(self.pool)
                .await?;

                let mut tables = Vec::new();
                for row in rows {
                    let name: String = row.try_get("tablename")?;
                    tables.push(name);
                }
                Ok(tables)
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
        let rows = sqlx::query("PRAGMA table_info(?)")
            .bind(table_name)
            .fetch_all(self.pool)
            .await?;

        let mut columns = Vec::new();
        for row in rows {
            let name: String = row.try_get("name")?;
            let data_type: String = row.try_get("type")?;
            let notnull: i32 = row.try_get("notnull")?;
            let dflt_value: Option<String> = row.try_get("dflt_value").ok();
            let pk: i32 = row.try_get("pk")?;

            columns.push(ColumnInfo {
                name,
                data_type,
                nullable: notnull == 0,
                default_value: dflt_value,
                is_primary_key: pk != 0,
            });
        }

        Ok(TableInfo {
            name: table_name.to_string(),
            columns,
        })
    }

    async fn get_mysql_table_info(&self, table_name: &str) -> Result<TableInfo, sqlx::Error> {
        let rows = sqlx::query("DESCRIBE ?")
            .bind(table_name)
            .fetch_all(self.pool)
            .await?;

        let mut columns = Vec::new();
        for row in rows {
            let name: String = row.try_get("Field")?;
            let data_type: String = row.try_get("Type")?;
            let nullable: String = row.try_get("Null")?;
            let default_value: Option<String> = row.try_get("Default").ok();
            let key: String = row.try_get("Key")?;

            columns.push(ColumnInfo {
                name,
                data_type,
                nullable: nullable.to_uppercase() == "YES",
                default_value,
                is_primary_key: key == "PRI",
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
                column_name,
                data_type,
                is_nullable,
                column_default,
                CASE WHEN pk.column_name IS NOT NULL THEN true ELSE false END as is_primary_key
            FROM information_schema.columns c
            LEFT JOIN (
                SELECT ku.column_name
                FROM information_schema.table_constraints tc
                JOIN information_schema.key_column_usage ku
                    ON tc.constraint_name = ku.constraint_name
                WHERE tc.constraint_type = 'PRIMARY KEY'
                    AND tc.table_name = $1
            ) pk ON c.column_name = pk.column_name
            WHERE c.table_name = $1
            ORDER BY c.ordinal_position
            "#,
        )
        .bind(table_name)
        .fetch_all(self.pool)
        .await?;

        let mut columns = Vec::new();
        for row in rows {
            let name: String = row.try_get("column_name")?;
            let data_type: String = row.try_get("data_type")?;
            let is_nullable: String = row.try_get("is_nullable")?;
            let default_value: Option<String> = row.try_get("column_default").ok();
            let is_primary_key: bool = row.try_get("is_primary_key")?;

            columns.push(ColumnInfo {
                name,
                data_type,
                nullable: is_nullable.to_uppercase() == "YES",
                default_value,
                is_primary_key,
            });
        }

        Ok(TableInfo {
            name: table_name.to_string(),
            columns,
        })
    }

    pub async fn execute_query(&self, sql: &str) -> Result<QueryResult, sqlx::Error> {
        let rows = sqlx::query(sql).fetch_all(self.pool).await?;

        let mut result_rows = Vec::new();
        let mut columns = Vec::new();

        if let Some(first_row) = rows.first() {
            columns = first_row
                .columns()
                .iter()
                .map(|col| col.name().to_string())
                .collect();
        }

        for row in rows {
            let mut row_data = Vec::new();
            for (i, _column) in row.columns().iter().enumerate() {
                let value: Option<String> = row.try_get(i).ok();
                row_data.push(value);
            }
            result_rows.push(row_data);
        }

        Ok(QueryResult {
            columns,
            rows: result_rows,
            rows_affected: None,
        })
    }

    pub async fn get_table_row_count(&self, table_name: &str) -> Result<i64, sqlx::Error> {
        let sql = format!("SELECT COUNT(*) as count FROM \"{}\"", table_name);
        let row = sqlx::query(&sql).fetch_one(self.pool).await?;
        let count: i64 = row.try_get("count")?;
        Ok(count)
    }

    pub async fn get_indexes(&self, table_name: &str) -> Result<Vec<IndexInfo>, sqlx::Error> {
        match self.config.database_type {
            DatabaseType::Sqlite => {
                let rows = sqlx::query("PRAGMA index_list(?)")
                    .bind(table_name)
                    .fetch_all(self.pool)
                    .await?;

                let mut indexes = Vec::new();
                for row in rows {
                    let name: String = row.try_get("name")?;
                    let unique: i32 = row.try_get("unique")?;

                    indexes.push(IndexInfo {
                        name,
                        unique: unique != 0,
                        columns: vec![], // Would need additional query to get columns
                    });
                }
                Ok(indexes)
            }
            _ => Ok(vec![]), // Placeholder for MySQL and PostgreSQL
        }
    }
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct TableInfo {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
}

#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub default_value: Option<String>,
    pub is_primary_key: bool,
}

#[derive(Debug, Clone)]
pub struct IndexInfo {
    pub name: String,
    pub unique: bool,
    pub columns: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
    pub rows_affected: Option<u64>,
}
