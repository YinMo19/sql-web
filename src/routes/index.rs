use crate::config::{DatabaseConfig, DatabaseManager, DatabasePool};
use crate::models::{DatabaseStats, FlashMessage as Flash};
use crate::Args;

use crate::template::{IntoTemplateResponse, TemplateResponse};
use askama::Template;
use rocket::form::Form;
use rocket::http::{Cookie, CookieJar, Status};
use rocket::request::{FromRequest, Outcome, Request};
use rocket::response::Redirect;
use rocket::{get, post, State};

#[allow(unused)]
#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub database_stats: DatabaseStats,
    pub tables: Vec<String>,
    pub version: String,
    pub flash_messages: Vec<Flash>,
    pub file_size_display: String,
}

impl IndexTemplate {
    pub fn new(
        database_stats: DatabaseStats,
        tables: Vec<String>,
        version: String,
        flash_messages: Vec<Flash>,
    ) -> Self {
        let file_size_display = match database_stats.file_size {
            Some(size) => format!("{} bytes", size),
            None => "Unknown".to_string(),
        };

        Self {
            database_stats,
            tables,
            version,
            flash_messages,
            file_size_display,
        }
    }
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub error: Option<String>,
}

#[derive(rocket::FromForm)]
pub struct LoginForm {
    password: String,
}

pub struct AuthGuard;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthGuard {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let cookies = request.cookies();
        if let Some(_) = cookies.get_private("authenticated") {
            Outcome::Success(AuthGuard)
        } else {
            Outcome::Error((Status::Unauthorized, ()))
        }
    }
}

#[get("/", rank = 1)]
pub async fn index(
    db: &State<DatabasePool>,
    config: &State<DatabaseConfig>,
    _args: &State<Args>,
    _auth: AuthGuard,
) -> Result<TemplateResponse<IndexTemplate>, Status> {
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

    let database_stats = DatabaseStats {
        database_name: database_info.base_name(),
        database_type: format!("{:?}", database_info.database_type),
        file_size: database_info.size,
        table_count: tables.len(),
        index_count: 0,   // TODO: Implement index counting
        trigger_count: 0, // TODO: Implement trigger counting
        view_count: 0,    // TODO: Implement view counting
        created: database_info.created,
        modified: database_info.modified,
        readonly: database_info.readonly,
    };

    Ok(
        IndexTemplate::new(database_stats, tables, "0.1.0".to_string(), vec![])
            .into_template_response(),
    )
}

#[get("/", rank = 2)]
pub async fn index_redirect() -> Redirect {
    Redirect::to("/login")
}

#[get("/login")]
pub async fn login() -> TemplateResponse<LoginTemplate> {
    LoginTemplate { error: None }.into_template_response()
}

#[post("/login", data = "<form>")]
pub async fn login_post(
    form: Form<LoginForm>,
    cookies: &CookieJar<'_>,
    _args: &State<Args>,
) -> Result<Redirect, TemplateResponse<LoginTemplate>> {
    // For now, we'll use a simple password check
    // In a real application, you'd want to use proper authentication
    let expected_password =
        std::env::var("SQL_WEB_PASSWORD").unwrap_or_else(|_| "admin".to_string());

    if form.password == expected_password {
        cookies.add_private(Cookie::new("authenticated", "true"));
        Ok(Redirect::to("/"))
    } else {
        Err(LoginTemplate {
            error: Some("Invalid password".to_string()),
        }
        .into_template_response())
    }
}

#[get("/logout")]
pub async fn logout(cookies: &CookieJar<'_>) -> Redirect {
    cookies.remove_private(Cookie::build(("authenticated", "")));
    Redirect::to("/login")
}
