use chrono::Utc;
use rocket::{
    form::Form,
    get,
    http::{Cookie, CookieJar},
    post,
    response::{Flash, Redirect},
    routes, uri, Route,
};
use rocket_dyn_templates::Template;

use crate::{cata_log, meltdown::*, middleware::*, routes::*, services::default::*, structs::*};

#[post("/auth/login", data = "<login_form>")]
async fn post_login(login_form: Form<LoginForm>, cookies: &CookieJar<'_>, app_context: AppContext<'_>) -> Result<Flash<Redirect>, Flash<Redirect>> {
    let login = login_form.into_inner();

    if let Err(csrf_error) = verify_csrf_for_state_change(&app_context, &login.authenticity_token) {
        return Err(Flash::error(Redirect::to(uri!(get_login)), csrf_error.user_message()));
    }

    let user = match Users::get_user_by_username(login.username.clone()).await {
        Ok(user) => user,
        Err(_) => {
            cata_log!(Warning, format!("Login attempt with invalid username: {}", login.username));
            return Err(Flash::error(Redirect::to(uri!(get_login)), MeltDown::invalid_credentials().user_message()));
        }
    };

    match user.verify_password(login.password.clone()).await {
        Ok(true) => {
            let remember = login.remember_me.unwrap_or(false);

            let device_info = Some(format!("Login at {}", Utc::now().to_rfc3339()));

            let token_pair = match crate::services::default::jwt_service::generate_token_pair(&user, remember, device_info) {
                Ok(pair) => pair,
                Err(error) => {
                    return Err(Flash::error(Redirect::to(uri!(get_login)), error.user_message()));
                }
            };

            cookies.add(Cookie::build(Cookie::new("access_token", token_pair.access_token)).http_only(true).secure(true).build());

            cookies.add(Cookie::build(Cookie::new("refresh_token", token_pair.refresh_token)).http_only(true).secure(true).build());

            cookies.add(Cookie::build(Cookie::new("user_id", user.id.to_string())).http_only(true).secure(true).build());

            let access_expiry = token_pair.access_claims.exp as i64 - Utc::now().timestamp();
            let refresh_expiry = token_pair.refresh_claims.exp as i64 - Utc::now().timestamp();

            cata_log!(
                Debug,
                format!("Issuing token pair for user {}: access token expires in {}s, refresh token expires in {}s", user.id, access_expiry, refresh_expiry)
            );
            cata_log!(Info, format!("User {} logged in successfully", user.username));

            let is_admin = user.role == "admin";
            let redirect_uri = if is_admin { uri!(admin::get_admin_dashboard) } else { uri!(user::get_user_dashboard) };

            Ok(Flash::success(Redirect::to(redirect_uri), "Successfully logged in."))
        }
        Ok(false) | Err(_) => {
            cata_log!(Warning, format!("Failed login attempt for user: {}", login.username));
            Err(Flash::error(Redirect::to(uri!(get_login)), MeltDown::invalid_credentials().user_message()))
        }
    }
}

#[get("/auth/logout")]
fn get_logout(cookies: &CookieJar<'_>, jwt: Option<JWT>) -> Flash<Redirect> {
    cookies.remove(Cookie::new("access_token", ""));
    cookies.remove(Cookie::new("refresh_token", ""));
    cookies.remove(Cookie::new("user_id", ""));

    if let Some(jwt) = jwt {
        let user_id = jwt.user_id();
        cata_log!(Info, format!("User {} logged out", user_id));
    } else {
        cata_log!(Info, "Anonymous user logged out");
    }

    Flash::success(Redirect::to(uri!(get_login)), "Successfully logged out.")
}

#[get("/auth/register")]
async fn get_register(app_context: AppContext<'_>) -> Template {
    cata_log!(Info, "Rendering registration page");
    app_context.render("auth/register")
}

#[post("/auth/register", data = "<register_form>")]
async fn post_register(register_form: Form<RegisterForm>, app_context: AppContext<'_>) -> Flash<Redirect> {
    let register = register_form.into_inner();

    if let Err(csrf_error) = verify_csrf_for_state_change(&app_context, &register.authenticity_token) {
        return Flash::error(Redirect::to(uri!(get_register)), csrf_error.user_message());
    }

    match Users::register_user(register.clone()).await {
        Ok(()) => {
            cata_log!(Info, "User registered successfully");
            Flash::success(Redirect::to(uri!(get_login)), "Successfully registered.")
        }
        Err(err_msg) => {
            cata_log!(Error, format!("Registration error: {}", err_msg.log_message()));
            Flash::error(Redirect::to(uri!(get_register)), err_msg.user_message())
        }
    }
}

#[get("/auth/login")]
async fn get_login(app_context: AppContext<'_>, jwt: Option<JWT>) -> Result<Template, Flash<Redirect>> {
    if let Some(jwt) = jwt {
        let is_admin = jwt.is_admin();
        let redirect_uri = if is_admin { uri!(admin::get_admin_dashboard) } else { uri!(user::get_user_dashboard) };
        cata_log!(Info, format!("User {} is already logged in", jwt.get_username()));
        return Err(Flash::success(Redirect::to(redirect_uri), "Already logged in."));
    }

    cata_log!(Info, "Rendering login page");
    Ok(app_context.render("auth/login"))
}

#[get("/")]
pub async fn get_home(app_context: AppContext<'_>, jwt: Option<JWT>) -> Result<Template, Flash<Redirect>> {
    if let Some(jwt) = jwt {
        let is_admin = jwt.is_admin();
        let redirect_uri = if is_admin { uri!(admin::get_admin_dashboard) } else { uri!(user::get_user_dashboard) };
        return Err(Flash::success(Redirect::to(redirect_uri), "Already logged in."));
    }

    Ok(app_context.render("index"))
}

#[get("/oops")]
pub async fn page_not_found(app_context: AppContext<'_>) -> Template {
    app_context.render("oops/index")
}

#[post("/auth/refresh")]
async fn refresh_token(cookies: &CookieJar<'_>) -> Result<(), Flash<Redirect>> {
    let refresh_token = match cookies.get("refresh_token") {
        Some(cookie) => cookie.value().to_string(),
        None => {
            cata_log!(Warning, "Refresh attempt without refresh token");
            return Err(Flash::error(Redirect::to(uri!(get_login)), "Session expired. Please log in again."));
        }
    };

    let token_info = match crate::services::default::jwt_service::validate_refresh_token(&refresh_token) {
        Ok(info) => info,
        Err(error) => {
            cata_log!(Warning, format!("Invalid refresh token: {}", error.log_message()));

            cookies.remove(Cookie::new("access_token", ""));
            cookies.remove(Cookie::new("refresh_token", ""));
            cookies.remove(Cookie::new("user_id", ""));

            return Err(Flash::error(Redirect::to(uri!(get_login)), "Session expired. Please log in again."));
        }
    };

    let user_id = token_info.user_id;
    if crate::services::default::token_registry::is_refresh_token_used(user_id, &token_info.jti) {
        cata_log!(Warning, format!("Attempted reuse of refresh token: jti={} for user {}", token_info.jti, user_id));

        cookies.remove(Cookie::new("access_token", ""));
        cookies.remove(Cookie::new("refresh_token", ""));
        cookies.remove(Cookie::new("user_id", ""));

        crate::services::default::token_registry::invalidate_user_tokens(user_id);

        return Err(Flash::error(Redirect::to(uri!(get_login)), "Security concern detected. Please log in again."));
    }

    crate::services::default::token_registry::mark_refresh_token_used(user_id, &token_info.jti);

    let user = match Users::get_user_by_id(user_id).await {
        Ok(user) => user,
        Err(error) => {
            cata_log!(Error, format!("Failed to get user {}: {}", user_id, error.log_message()));
            return Err(Flash::error(Redirect::to(uri!(get_login)), "User account issue. Please log in again."));
        }
    };

    let token_pair = match crate::services::default::jwt_service::generate_token_pair(&user, token_info.remember, token_info.device_info) {
        Ok(pair) => pair,
        Err(error) => {
            cata_log!(Error, format!("Failed to generate new tokens: {}", error.log_message()));
            return Err(Flash::error(Redirect::to(uri!(get_login)), "Authentication error. Please log in again."));
        }
    };

    cookies.add(Cookie::build(Cookie::new("access_token", token_pair.access_token)).http_only(true).secure(true).build());

    cookies.add(Cookie::build(Cookie::new("refresh_token", token_pair.refresh_token)).http_only(true).secure(true).build());

    cookies.add(Cookie::build(Cookie::new("user_id", user_id.to_string())).http_only(true).secure(true).build());

    cata_log!(Info, format!("Refreshed tokens for user {}", user_id));

    Ok(())
}

pub fn routes() -> Vec<Route> {
    routes![get_home, page_not_found, get_login, get_logout, get_register, post_login, post_register, refresh_token]
}
