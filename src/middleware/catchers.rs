use crate::cata_log;
use crate::meltdown::*;
use crate::routes::*;
use rocket::catch;
use rocket::request::Request;
use rocket::response::{Redirect, Responder};
use rocket::serde::json::Json;
use rocket::uri;
use rocket_dyn_templates::Template;
use serde_json::json;

use super::app_context;

#[catch(401)]
pub fn unauthorized(req: &Request) -> Result<Redirect, Json<serde_json::Value>> {
    cata_log!(Warning, format!("Unauthorized access attempt to {}", req.uri()));

    if req.uri().path().starts_with("/api") || accepts_json(req) {
        let error = MeltDown::new(MeltType::Unauthorized, "Authentication required");
        return Err(Json(json!({
            "error": {
                "code": 401,
                "message": error.user_message()
            }
        })));
    }

    Ok(Redirect::to(uri!(home::get_login)))
}

#[catch(403)]
pub fn forbidden(req: &Request) -> Result<Redirect, Json<serde_json::Value>> {
    cata_log!(Warning, format!("Forbidden access attempt to {}", req.uri()));

    if req.uri().path().starts_with("/api") || accepts_json(req) {
        let error = MeltDown::new(MeltType::Forbidden, "Insufficient permissions");
        return Err(Json(json!({
            "error": {
                "code": 403,
                "message": error.user_message()
            }
        })));
    }

    Ok(Redirect::to(uri!(home::get_login)))
}

#[catch(404)]
pub fn not_found(req: &Request) -> Result<Template, Json<serde_json::Value>> {
    cata_log!(Warning, format!("Not found: {}", req.uri()));

    if req.uri().path().starts_with("/api") || accepts_json(req) {
        let error = MeltDown::new(MeltType::NotFound, "Resource");
        return Err(Json(json!({
            "error": {
                "code": 404,
                "message": error.user_message()
            }
        })));
    }

    let context = app_context::BaseContext {
        lang: json!({}),
        translations: json!({}),
        flash: None,
        title: Some("Page Not Found".to_string()),
        csrf_token: None,
        environment: "dev".to_string(),
        sparks: crate::services::makeuse::get_template_components(true),
    };

    Ok(Template::render("oops/index", &context))
}

#[catch(422)]
pub fn unprocessable_entity(req: &Request) -> Result<Redirect, Json<serde_json::Value>> {
    cata_log!(Warning, format!("Form validation error on {}", req.uri()));
    let form_error = match req.local_cache(|| Option::<String>::None) {
        Some(msg) => msg.clone(),
        None => "The form has invalid data".to_string(),
    };

    let error = MeltDown::validation_failed(&form_error);

    if req.uri().path().starts_with("/api") || accepts_json(req) {
        return Err(Json(json!({
            "error": {
                "code": 422,
                "message": error.user_message()
            }
        })));
    }

    match req.uri().path() {
        path if path.contains("/auth/login") => Ok(Redirect::to(uri!(home::get_login))),
        path if path.contains("/auth/register") => Ok(Redirect::to(uri!(home::get_register))),
        _ => Ok(Redirect::to(uri!(home::get_home))),
    }
}

#[catch(500)]
pub fn internal_error(req: &Request) -> Result<Redirect, Json<serde_json::Value>> {
    cata_log!(Error, format!("Internal server error on {}", req.uri()));
    if req.uri().path().starts_with("/api") || accepts_json(req) {
        let error = MeltDown::new(MeltType::Unknown, "Internal server error");
        return Err(Json(json!({
            "error": {
                "code": 500,
                "message": error.user_message()
            }
        })));
    }

    Ok(Redirect::to(uri!(home::page_not_found)))
}

fn accepts_json(request: &Request) -> bool {
    request.headers().get("Accept").any(|v| v.contains("application/json"))
}

impl<'r> Responder<'r, 'static> for MeltDown {
    fn respond_to(self, req: &'r Request<'_>) -> rocket::response::Result<'static> {
        self.log();

        let status = self.status_code();

        if req.uri().path().starts_with("/api") || accepts_json(req) {
            Json(json!({
                "error": {
                    "code": status.code,
                    "message": self.user_message()
                }
            }))
            .respond_to(req)
        } else {
            match status.code {
                401 => Redirect::to(uri!(home::get_login)).respond_to(req),
                403 => Redirect::to(uri!(home::get_login)).respond_to(req),
                404 => {
                    let context = app_context::BaseContext {
                        lang: json!({}),
                        translations: json!({}),
                        flash: None,
                        title: Some("Page Not Found".to_string()),
                        csrf_token: None,
                        environment: "dev".to_string(),
                        sparks: crate::services::makeuse::get_template_components(true),
                    };
                    Template::render("oops/index", &context).respond_to(req)
                }
                _ => {
                    let context = app_context::BaseContext {
                        lang: json!({}),
                        translations: json!({}),
                        flash: Some(("error".to_string(), self.user_message())),
                        title: Some("Error".to_string()),
                        csrf_token: None,
                        environment: "dev".to_string(),
                        sparks: crate::services::makeuse::get_template_components(true),
                    };
                    Template::render("oops/index", &context).respond_to(req)
                }
            }
        }
    }
}
