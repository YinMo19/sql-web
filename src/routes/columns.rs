use crate::config::{DatabaseConfig, DatabaseManager, DatabasePool};
use crate::routes::index::AuthGuard;
use crate::template::{IntoTemplateResponse, TemplateResponse};
use askama::Template;
use rocket::form::Form;
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::{get, post, State};

#[derive(Template)]
#[template(path = "add_column.html")]
pub struct AddColumnTemplate {
    pub table_name: String,
    pub database_name: String,
    pub readonly: bool,
    pub flash_messages: Vec<crate::models::FlashMessage>,
    pub version: String,
    pub error: String,
}

#[derive(Template)]
#[template(path = "drop_column.html")]
pub struct DropColumnTemplate {
    pub table_name: String,
    pub columns: Vec<String>,
    pub database_name: String,
    pub readonly: bool,
    pub flash_messages: Vec<crate::models::FlashMessage>,
    pub version: String,
    pub error: String,
}

#[derive(Template)]
#[template(path = "rename_column.html")]
pub struct RenameColumnTemplate {
    pub table_name: String,
    pub columns: Vec<String>,
    pub database_name: String,
    pub readonly: bool,
    pub flash_messages: Vec<crate::models::FlashMessage>,
    pub version: String,
    pub error: String,
}

#[derive(rocket::FromForm)]
pub struct AddColumnForm {
    pub name: String,
    pub data_type: String,
    pub nullable: Option<bool>,
    pub default_value: Option<String>,
    pub primary_key: Option<bool>,
}

#[derive(rocket::FromForm)]
pub struct DropColumnForm {
    pub column_name: String,
}

#[derive(rocket::FromForm)]
pub struct RenameColumnForm {
    pub old_name: String,
    pub new_name: String,
}

#[get("/table/<table_name>/add-column")]
pub async fn add_column(
    table_name: String,
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<TemplateResponse<AddColumnTemplate>, Status> {
    if config.readonly {
        return Err(Status::Forbidden);
    }

    let pool = &db.0;
    let manager = DatabaseManager::new(pool, config.inner().clone());

    let database_info = manager
        .get_database_info()
        .await
        .map_err(|_| Status::InternalServerError)?;

    Ok(AddColumnTemplate {
        table_name: table_name.clone(),
        database_name: database_info.base_name(),
        readonly: database_info.readonly,
        flash_messages: vec![],
        version: "0.1.0".to_string(),
        error: "".to_string(),
    }
    .into_template_response())
}

#[post("/table/<table_name>/add-column", data = "<form>")]
pub async fn add_column_execute(
    table_name: String,
    form: Form<AddColumnForm>,
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<Redirect, Status> {
    if config.readonly {
        return Err(Status::Forbidden);
    }

    let pool = &db.0;
    let manager = DatabaseManager::new(pool, config.inner().clone());

    let mut sql = format!(
        "ALTER TABLE \"{}\" ADD COLUMN \"{}\" {}",
        table_name, form.name, form.data_type
    );

    if form.nullable != Some(true) {
        sql += " NOT NULL";
    }

    if let Some(ref default_val) = form.default_value {
        if !default_val.is_empty() {
            sql += &format!(" DEFAULT '{}'", default_val.replace("'", "''"));
        }
    }

    if form.primary_key == Some(true) {
        // Note: Adding primary key constraint to existing table is complex
        // This is a simplified version that may not work for all databases
        sql += " PRIMARY KEY";
    }

    match manager.execute_query(&sql).await {
        Ok(_) => Ok(Redirect::to(format!("/table/{}/structure", table_name))),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[get("/table/<table_name>/drop-column")]
pub async fn drop_column(
    table_name: String,
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<TemplateResponse<DropColumnTemplate>, Status> {
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

    Ok(DropColumnTemplate {
        table_name: table_name.clone(),
        columns,
        database_name: database_info.base_name(),
        readonly: database_info.readonly,
        flash_messages: vec![],
        version: "0.1.0".to_string(),
        error: "".to_string(),
    }
    .into_template_response())
}

#[post("/table/<table_name>/drop-column", data = "<form>")]
pub async fn drop_column_execute(
    table_name: String,
    form: Form<DropColumnForm>,
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<Redirect, Status> {
    if config.readonly {
        return Err(Status::Forbidden);
    }

    let pool = &db.0;
    let manager = DatabaseManager::new(pool, config.inner().clone());

    let sql = format!(
        "ALTER TABLE \"{}\" DROP COLUMN \"{}\"",
        table_name, form.column_name
    );

    match manager.execute_query(&sql).await {
        Ok(_) => Ok(Redirect::to(format!("/table/{}/structure", table_name))),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[get("/table/<table_name>/rename-column")]
pub async fn rename_column(
    table_name: String,
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<TemplateResponse<RenameColumnTemplate>, Status> {
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

    Ok(RenameColumnTemplate {
        table_name: table_name.clone(),
        columns,
        database_name: database_info.base_name(),
        readonly: database_info.readonly,
        flash_messages: vec![],
        version: "0.1.0".to_string(),
        error: "".to_string(),
    }
    .into_template_response())
}

#[post("/table/<table_name>/rename-column", data = "<form>")]
pub async fn rename_column_execute(
    table_name: String,
    form: Form<RenameColumnForm>,
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<Redirect, Status> {
    if config.readonly {
        return Err(Status::Forbidden);
    }

    let pool = &db.0;
    let manager = DatabaseManager::new(pool, config.inner().clone());

    let sql = format!(
        "ALTER TABLE \"{}\" RENAME COLUMN \"{}\" TO \"{}\"",
        table_name, form.old_name, form.new_name
    );

    match manager.execute_query(&sql).await {
        Ok(_) => Ok(Redirect::to(format!("/table/{}/structure", table_name))),
        Err(_) => Err(Status::InternalServerError),
    }
}
