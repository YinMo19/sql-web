use crate::config::{DatabaseConfig, DatabaseManager, DatabasePool};
use crate::models::FlashMessage as Flash;
use crate::routes::index::AuthGuard;
use crate::template::{IntoTemplateResponse, TemplateResponse};
use askama::Template;
use rocket::form::Form;
use rocket::http::Status;
use rocket::response::Redirect;
use rocket::{get, post, State};

#[derive(Template)]
#[template(path = "add_index.html")]
pub struct AddIndexTemplate {
    pub table_name: String,
    pub columns: Vec<String>,
    pub database_name: String,
    pub readonly: bool,
    pub flash_messages: Vec<Flash>,
    pub version: String,
    pub error: String,
}

#[derive(Template)]
#[template(path = "drop_index.html")]
pub struct DropIndexTemplate {
    pub table_name: String,
    pub indexes: Vec<String>,
    pub database_name: String,
    pub readonly: bool,
    pub flash_messages: Vec<Flash>,
    pub version: String,
    pub error: String,
}

#[derive(rocket::FromForm)]
pub struct AddIndexForm {
    pub index_name: String,
    pub columns: Vec<String>,
    pub unique: Option<bool>,
}

#[derive(rocket::FromForm)]
pub struct DropIndexForm {
    pub index_name: String,
}

#[get("/table/<table_name>/add-index")]
pub async fn add_index(
    table_name: String,
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<TemplateResponse<AddIndexTemplate>, Status> {
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

    Ok(AddIndexTemplate {
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

#[post("/table/<table_name>/add-index", data = "<form>")]
pub async fn add_index_execute(
    table_name: String,
    form: Form<AddIndexForm>,
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<Redirect, Status> {
    if config.readonly {
        return Err(Status::Forbidden);
    }

    let pool = &db.0;
    let manager = DatabaseManager::new(pool, config.inner().clone());

    let index_name = form.index_name.trim();
    if index_name.is_empty() || form.columns.is_empty() {
        return Err(Status::BadRequest);
    }

    let unique_clause = if form.unique.unwrap_or(false) {
        "UNIQUE "
    } else {
        ""
    };

    let columns_str = form.columns.join(", ");

    let sql = format!(
        "CREATE {}INDEX \"{}\" ON \"{}\" ({})",
        unique_clause, index_name, table_name, columns_str
    );

    match manager.execute_query(&sql).await {
        Ok(_) => Ok(Redirect::to(format!("/table/{}/structure", table_name))),
        Err(_) => Err(Status::InternalServerError),
    }
}

#[get("/table/<table_name>/drop-index")]
pub async fn drop_index(
    table_name: String,
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<TemplateResponse<DropIndexTemplate>, Status> {
    if config.readonly {
        return Err(Status::Forbidden);
    }

    let pool = &db.0;
    let manager = DatabaseManager::new(pool, config.inner().clone());

    let database_info = manager
        .get_database_info()
        .await
        .map_err(|_| Status::InternalServerError)?;

    let indexes = manager
        .get_indexes(&table_name)
        .await
        .unwrap_or_else(|_| vec![]);

    let index_names: Vec<String> = indexes.iter().map(|i| i.name.clone()).collect();

    Ok(DropIndexTemplate {
        table_name: table_name.clone(),
        indexes: index_names,
        database_name: database_info.base_name(),
        readonly: database_info.readonly,
        flash_messages: vec![],
        version: "0.1.0".to_string(),
        error: String::new(),
    }
    .into_template_response())
}

#[post("/table/<table_name>/drop-index", data = "<form>")]
pub async fn drop_index_execute(
    table_name: String,
    form: Form<DropIndexForm>,
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _auth: AuthGuard,
) -> Result<Redirect, Status> {
    if config.readonly {
        return Err(Status::Forbidden);
    }

    let pool = &db.0;
    let manager = DatabaseManager::new(pool, config.inner().clone());

    let sql = format!("DROP INDEX \"{}\"", form.index_name);

    match manager.execute_query(&sql).await {
        Ok(_) => Ok(Redirect::to(format!("/table/{}/structure", table_name))),
        Err(_) => Err(Status::InternalServerError),
    }
}
