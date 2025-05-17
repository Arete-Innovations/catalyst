use rocket::{
    form::Form,
    get,
    http::{Cookie, CookieJar},
    post,
    response::{Flash, Redirect},
    routes, Route,
};
use rocket_dyn_templates::Template;

use crate::{
    cata_log,
    meltdown::*,
    middleware::*,
    vessel::structs::{Vessel, VesselLoginForm, VesselRegisterForm},
};

fn extract_tenant_name_from_path(path: &str) -> Option<String> {
    let path_segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if path_segments.len() >= 2 {
        let possible_tenant = path_segments[0].to_string();

        if !["auth", "api", "public", "admin", "user", "vessel"].contains(&possible_tenant.as_str()) {
            cata_log!(Info, format!("Found tenant name: {}", possible_tenant));
            return Some(possible_tenant);
        }
    }

    cata_log!(Info, "No tenant name found in path");
    None
}

#[get("/vessel/auth/login")]
pub async fn get_login_default(app_context: AppContext<'_>, jwt: Option<JWT>) -> Result<Template, Flash<Redirect>> {
    handle_vessel_login(app_context, jwt)
}

#[get("/<tenant>/vessel/auth/login")]
pub async fn get_login_tenant(tenant: &str, app_context: AppContext<'_>, jwt: Option<JWT>) -> Result<Template, Flash<Redirect>> {
    handle_vessel_login(app_context, jwt)
}

fn handle_vessel_login(app_context: AppContext<'_>, jwt: Option<JWT>) -> Result<Template, Flash<Redirect>> {
    if let Some(jwt) = jwt {
        if jwt.is_vessel_auth() && jwt.get_role() == "vessel" {
            cata_log!(Info, format!("User {} is already logged in to vessel system, redirecting to dashboard", jwt.get_username()));
            return Err(Flash::success(Redirect::to("/vessel/dashboard"), "Already logged in to vessel system."));
        } else if jwt.is_tenant_auth() {
            cata_log!(Info, format!("User {} is logged in to tenant system, showing vessel login page", jwt.get_username()));
        } else {
            cata_log!(
                Info,
                format!(
                    "User {} is logged in with role {}, auth system {:?}, showing vessel login page",
                    jwt.get_username(),
                    jwt.get_role(),
                    jwt.get_auth_system()
                )
            );
        }
    }

    cata_log!(Info, "Rendering vessel login page");
    Ok(app_context.render("vessel/login"))
}

#[post("/vessel/auth/login", data = "<login_form>")]
pub async fn post_login_default(login_form: Form<VesselLoginForm>, cookies: &CookieJar<'_>, app_context: AppContext<'_>, jwt: Option<JWT>) -> Result<Flash<Redirect>, Flash<Redirect>> {
    handle_vessel_post_login(login_form, cookies, app_context, jwt, None).await
}

#[post("/<tenant>/vessel/auth/login", data = "<login_form>")]
pub async fn post_login_tenant(tenant: &str, login_form: Form<VesselLoginForm>, cookies: &CookieJar<'_>, app_context: AppContext<'_>, jwt: Option<JWT>) -> Result<Flash<Redirect>, Flash<Redirect>> {
    handle_vessel_post_login(login_form, cookies, app_context, jwt, Some(tenant)).await
}

async fn handle_vessel_post_login(login_form: Form<VesselLoginForm>, cookies: &CookieJar<'_>, app_context: AppContext<'_>, jwt: Option<JWT>, tenant: Option<&str>) -> Result<Flash<Redirect>, Flash<Redirect>> {
    let login = login_form.into_inner();

    cata_log!(Info, format!("Vessel login attempt at path: {} (tenant: {:?})", app_context.request_uri(), tenant));

    let login_redirect = match tenant {
        Some(t) => format!("/{}/vessel/auth/login", t),
        None => "/vessel/auth/login".to_string(),
    };

    if let Err(csrf_error) = verify_csrf_for_state_change(&app_context, &login.authenticity_token) {
        cata_log!(Info, format!("CSRF verification failed, redirecting to: {}", login_redirect));
        return Err(Flash::error(Redirect::to(login_redirect), csrf_error.user_message()));
    }

    if let Some(jwt) = jwt {
        if jwt.is_tenant_auth() {
            cata_log!(Info, "User is currently logged into tenant system, clearing previous auth to switch to vessel system");

            cookies.remove(Cookie::new("access_token", ""));
            cookies.remove(Cookie::new("refresh_token", ""));
            cookies.remove(Cookie::new("user_id", ""));
        }
    }

    match Vessel::login_user(login).await {
        Ok((vessel, token_pair)) => {
            cata_log!(
                Info,
                format!(
                    "Vessel login successful for {} with role {} and auth_system {:?}, tenant: {}",
                    vessel.username, token_pair.access_claims.role, token_pair.access_claims.auth_system, vessel.name
                )
            );

            if let Some(jwt_tenant) = &token_pair.access_claims.tenant_name {
                cata_log!(Info, format!("JWT tenant name set to: {}", jwt_tenant));
            } else {
                cata_log!(Warning, "JWT does not contain a tenant name!");
            }

            cookies.add(Cookie::build(Cookie::new("access_token", token_pair.access_token)).http_only(true).secure(true).build());
            cookies.add(Cookie::build(Cookie::new("refresh_token", token_pair.refresh_token)).http_only(true).secure(true).build());
            cookies.add(Cookie::build(Cookie::new("user_id", vessel.id.to_string())).http_only(true).secure(true).build());

            let redirect_url = "/vessel/dashboard";
            cata_log!(Info, format!("Redirecting to vessel dashboard: {}", redirect_url));

            Ok(Flash::success(Redirect::to(redirect_url), "Successfully logged in."))
        }
        Err(error) => {
            cata_log!(Warning, format!("Vessel login failed: {}", error.log_message()));
            Err(Flash::error(Redirect::to(login_redirect), error.user_message()))
        }
    }
}

#[get("/vessel/auth/logout")]
pub fn get_logout(cookies: &CookieJar<'_>, jwt: Option<JWT>) -> Flash<Redirect> {
    cata_log!(Info, "Vessel logout initiated");

    fn remove_cookie_with_all_attributes(cookies: &CookieJar<'_>, name: &str, path_opt: Option<&str>) {
        let name_owned = name.to_string();
        let path_owned = path_opt.map(|p| p.to_string());

        let mut cookie = Cookie::new(name_owned.clone(), "");
        if let Some(ref path_value) = path_owned {
            cookie.set_path(path_value.clone());
        }
        cookie.set_http_only(true);
        cookie.set_secure(true);
        cookies.remove(cookie);

        let mut basic_cookie = Cookie::new(name_owned, "");
        if let Some(ref path_value) = path_owned {
            basic_cookie.set_path(path_value.clone());
        }
        cookies.remove(basic_cookie);
    }

    let cookie_names = ["access_token", "refresh_token", "user_id", "tenant", "tenant_id", "csrf_token"];

    let static_paths = ["/", "/vessel", "/vessel/auth", "/vessel/dashboard", "/auth"];

    for path in &static_paths {
        for &name in &cookie_names {
            remove_cookie_with_all_attributes(cookies, name, Some(path));
        }
    }

    for &name in &cookie_names {
        remove_cookie_with_all_attributes(cookies, name, None);
    }

    if let Some(ref jwt) = jwt {
        if let Some(tenant) = jwt.get_tenant_name() {
            let tenant_paths = [&format!("/{}", tenant), &format!("/{}/auth", tenant), &format!("/{}/admin", tenant), &format!("/{}/user", tenant)];

            for path in &tenant_paths {
                for &name in &cookie_names {
                    remove_cookie_with_all_attributes(cookies, name, Some(path));
                }
            }

            cata_log!(Info, format!("Cleared cookies for tenant: {}", tenant));
        }
    }

    if let Some(jwt) = jwt {
        let user_id = jwt.user_id();
        cata_log!(Info, format!("Vessel {} logged out", user_id));
    } else {
        cata_log!(Info, "Anonymous user logged out (vessel)");
    }

    let login_redirect = "/vessel/auth/login";
    cata_log!(Info, format!("Redirecting logout to: {}", login_redirect));

    Flash::success(Redirect::to(login_redirect), "Successfully logged out.")
}

#[get("/vessel/auth/register")]
pub async fn get_register(app_context: AppContext<'_>) -> Template {
    cata_log!(Info, "Rendering vessel registration page");
    app_context.render("vessel/register")
}

#[post("/vessel/auth/register", data = "<register_form>")]
pub async fn post_register(register_form: Form<VesselRegisterForm>, app_context: AppContext<'_>) -> Flash<Redirect> {
    let register = register_form.into_inner();

    if let Err(csrf_error) = verify_csrf_for_state_change(&app_context, &register.authenticity_token) {
        return Flash::error(Redirect::to("/vessel/auth/register"), csrf_error.user_message());
    }

    match Vessel::register_user(register).await {
        Ok(vessel) => {
            cata_log!(Info, format!("Vessel '{}' registered successfully with ID {}", vessel.name, vessel.id));
            Flash::success(Redirect::to("/vessel/auth/login"), "Successfully registered. Your new tenant database has been provisioned. You can now log in.")
        }
        Err(err) => {
            cata_log!(Error, format!("Vessel registration failed: {}", err.log_message()));

            let error_message = match err.melt_type {
                MeltType::ValidationFailed => err.user_message(),
                _ => "Registration failed. Please try again later or contact support.".to_string(),
            };

            Flash::error(Redirect::to("/vessel/auth/register"), error_message)
        }
    }
}

#[post("/vessel/auth/refresh")]
pub async fn refresh_token(cookies: &CookieJar<'_>) -> Result<(), Flash<Redirect>> {
    let refresh_token = match cookies.get("refresh_token") {
        Some(cookie) => cookie.value().to_string(),
        None => {
            cata_log!(Warning, "Refresh attempt without refresh token (vessel)");
            return Err(Flash::error(Redirect::to("/vessel/auth/login"), "Session expired. Please log in again."));
        }
    };

    match Vessel::refresh_user_token(&refresh_token).await {
        Ok((vessel, token_pair)) => {
            if let Some(jwt_tenant) = &token_pair.access_claims.tenant_name {
                cata_log!(Info, format!("Refresh: JWT tenant name set to: {}", jwt_tenant));
            } else {
                cata_log!(Warning, "Refresh: JWT does not contain a tenant name!");
            }

            cookies.add(Cookie::build(Cookie::new("access_token", token_pair.access_token)).http_only(true).secure(true).build());
            cookies.add(Cookie::build(Cookie::new("refresh_token", token_pair.refresh_token)).http_only(true).secure(true).build());
            cookies.add(Cookie::build(Cookie::new("user_id", vessel.id.to_string())).http_only(true).secure(true).build());
            Ok(())
        }
        Err(error) => {
            cata_log!(Warning, format!("Invalid refresh token (vessel): {}", error.log_message()));

            cookies.remove(Cookie::new("access_token", ""));
            cookies.remove(Cookie::new("refresh_token", ""));
            cookies.remove(Cookie::new("user_id", ""));

            Err(Flash::error(Redirect::to("/vessel/auth/login"), "Session expired. Please log in again."))
        }
    }
}

pub fn auth_routes() -> Vec<Route> {
    routes![get_login_default, get_login_tenant, post_login_default, post_login_tenant, get_logout, get_register, post_register, refresh_token]
}
