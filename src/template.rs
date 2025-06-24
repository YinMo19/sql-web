use askama::Template;
use rocket::http::{ContentType, Status};
use rocket::response::{self, Responder, Response};
use rocket::Request;
use std::io::Cursor;

/// A wrapper for Askama templates that implements Rocket's Responder trait
pub struct TemplateResponse<T: Template> {
    template: T,
}

impl<T: Template> TemplateResponse<T> {
    pub fn new(template: T) -> Self {
        Self { template }
    }
}

impl<'r, T: Template> Responder<'r, 'static> for TemplateResponse<T> {
    fn respond_to(self, _: &'r Request<'_>) -> response::Result<'static> {
        match self.template.render() {
            Ok(string) => Response::build()
                .header(ContentType::HTML)
                .sized_body(string.len(), Cursor::new(string))
                .ok(),
            Err(_) => Err(Status::InternalServerError),
        }
    }
}

/// Helper trait to convert templates to responses
pub trait IntoTemplateResponse<T: Template> {
    fn into_template_response(self) -> TemplateResponse<T>;
}

impl<T: Template> IntoTemplateResponse<T> for T {
    fn into_template_response(self) -> TemplateResponse<T> {
        TemplateResponse::new(self)
    }
}
