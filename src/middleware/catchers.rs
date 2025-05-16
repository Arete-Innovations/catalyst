use rocket::{
    catch,
    request::Request,
    response::{Redirect, Responder},
    serde::json::Json,
    uri,
};
use rocket_dyn_templates::Template;
use serde_json::json;

use super::app_context;
use crate::{cata_log, meltdown::*, routes::*};

fn extract_tenant_name(req: &Request) -> String {
    let path = req.uri().path().as_str();
    let parts: Vec<&str> = path.split('/').collect();

    if parts.len() > 1 && !parts[1].is_empty() && !parts[1].starts_with("api") && !parts[1].starts_with("public") {
        parts[1].to_string()
    } else {
        "main".to_string()
    }
}

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

    let tenant = extract_tenant_name(req);
    let path = req.uri().path().as_str();

    if path.contains("/admin/") {
        cata_log!(Info, format!("Redirecting admin access attempt to tenant admin login for tenant: {}", tenant));
        return Ok(Redirect::to(format!("/{}/auth/login", tenant)));
    }

    Ok(Redirect::to(format!("/{}/auth/login", tenant)))
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

    let tenant = extract_tenant_name(req);
    let path = req.uri().path().as_str();

    let is_vessel_auth = req.local_cache(|| Option::<bool>::None).as_ref().map(|b| *b).unwrap_or(false);

    if path.contains("/admin/") {
        cata_log!(Info, format!("Redirecting forbidden admin access to tenant admin login for tenant: {}", tenant));
        return Ok(Redirect::to(format!("/{}/auth/login", tenant)));
    }

    Ok(Redirect::to(format!("/{}/auth/login", tenant)))
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
        tenant_name: Some(extract_tenant_name(req)),
        request_uri: req.uri().path().to_string(),
    };

    let mut map = serde_json::Map::new();
    map.insert(
        "app_context".to_string(),
        json!({
            "tenant_name": extract_tenant_name(req),
            "request_uri": req.uri().path().to_string()
        }),
    );

    let context_json = serde_json::to_value(&context).unwrap();
    if let serde_json::Value::Object(obj) = context_json {
        for (k, v) in obj {
            map.insert(k, v);
        }
    }

    Ok(Template::render("oops/index", &map))
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

    let tenant = extract_tenant_name(req);
    let path = req.uri().path().as_str();

    if path.contains("/auth/login") {
        Ok(Redirect::to(format!("/{}/auth/login", tenant)))
    } else if path.contains("/auth/register") {
        Ok(Redirect::to(format!("/{}/auth/register", tenant)))
    } else {
        Ok(Redirect::to(format!("/{}", tenant)))
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

    let tenant = extract_tenant_name(req);
    Ok(Redirect::to(format!("/{}/not-found", tenant)))
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
            let tenant = extract_tenant_name(req);
            let path = req.uri().path().as_str();

            match status.code {
                401 => Redirect::to(format!("/{}/auth/login", tenant)).respond_to(req),
                403 => Redirect::to(format!("/{}/auth/login", tenant)).respond_to(req),
                404 => {
                    let context = app_context::BaseContext {
                        lang: json!({}),
                        translations: json!({}),
                        flash: None,
                        title: Some("Page Not Found".to_string()),
                        csrf_token: None,
                        environment: "dev".to_string(),
                        sparks: crate::services::makeuse::get_template_components(true),
                        tenant_name: Some(tenant.clone()),
                        request_uri: req.uri().path().to_string(),
                    };
                    let mut map = serde_json::Map::new();
                    map.insert(
                        "app_context".to_string(),
                        json!({
                            "tenant_name": tenant.clone(),
                            "request_uri": req.uri().path().to_string()
                        }),
                    );

                    let context_json = serde_json::to_value(&context).unwrap();
                    if let serde_json::Value::Object(obj) = context_json {
                        for (k, v) in obj {
                            map.insert(k, v);
                        }
                    }

                    Template::render("oops/index", &map).respond_to(req)
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
                        tenant_name: Some(tenant.clone()),
                        request_uri: req.uri().path().to_string(),
                    };
                    let mut map = serde_json::Map::new();
                    map.insert(
                        "app_context".to_string(),
                        json!({
                            "tenant_name": tenant.clone(),
                            "request_uri": req.uri().path().to_string()
                        }),
                    );

                    let context_json = serde_json::to_value(&context).unwrap();
                    if let serde_json::Value::Object(obj) = context_json {
                        for (k, v) in obj {
                            map.insert(k, v);
                        }
                    }

                    Template::render("oops/index", &map).respond_to(req)
                }
            }
        }
    }
}
