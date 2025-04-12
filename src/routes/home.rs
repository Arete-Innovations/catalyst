use crate::cata_log;
use crate::meltdown::MeltDown;
use crate::middleware::*;
use crate::routes::*;
use crate::structs::*;
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header as JWTHeader};
use rocket::form::Form;
use rocket::http::{Cookie, CookieJar};
use rocket::response::Flash;
use rocket::response::Redirect;
use rocket::uri;
use rocket::{get, post, routes, Route};
use rocket_dyn_templates::Template;
use std::env;

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
            let expiration = Utc::now().checked_add_signed(Duration::seconds(86400)).unwrap_or_else(|| Utc::now()).timestamp();

            let claims = Claims {
                sub: user.id.to_string(),
                exp: expiration as usize,
            };

            let token = match encode(&JWTHeader::default(), &claims, &EncodingKey::from_secret(env::var("JWT_SECRET").map_err(|e| MeltDown::from(e))?.as_ref())) {
                Ok(token) => token,
                Err(e) => {
                    let error = MeltDown::from(e);
                    cata_log!(Error, error.log_message());
                    return Err(Flash::error(Redirect::to(uri!(get_login)), error.user_message()));
                }
            };

            cookies.add(Cookie::new("token", token));
            cookies.add(Cookie::new("user_id", user.id.to_string()));
            cata_log!(Info, format!("User {} logged in successfully", user.username));

            let is_admin = Users::is_admin(user.id).await.unwrap_or(false);
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
fn get_logout(cookies: &CookieJar<'_>) -> Flash<Redirect> {
    cookies.remove(Cookie::build("token"));
    cookies.remove(Cookie::build("user_id"));
    cata_log!(Info, "User logged out");
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
        match jwt_to_id(&jwt) {
            Ok(user_id) => match Users::get_user_by_id(user_id).await {
                Ok(user) => {
                    let is_admin = Users::is_admin(user.id).await.unwrap_or(false);
                    let redirect_uri = if is_admin { uri!(admin::get_admin_dashboard) } else { uri!(user::get_user_dashboard) };
                    cata_log!(Info, format!("User {} is already logged in", user.username));
                    return Err(Flash::success(Redirect::to(redirect_uri), "Already logged in."));
                }
                Err(err) => {
                    cata_log!(Warning, format!("JWT user lookup failed: {}", err.log_message()));
                }
            },
            Err(err) => {
                cata_log!(Warning, format!("JWT ID parsing failed: {}", err.log_message()));
            }
        }
    }

    cata_log!(Info, "Rendering login page");
    Ok(app_context.render("auth/login"))
}

#[get("/")]
pub async fn get_home(app_context: AppContext<'_>, jwt: Option<JWT>) -> Result<Template, Flash<Redirect>> {
    if let Some(jwt) = jwt {
        match jwt_to_id(&jwt) {
            Ok(user_id) => match Users::get_user_by_id(user_id).await {
                Ok(user) => {
                    let is_admin = Users::is_admin(user.id).await.unwrap_or(false);
                    let redirect_uri = if is_admin { uri!(admin::get_admin_dashboard) } else { uri!(user::get_user_dashboard) };
                    return Err(Flash::success(Redirect::to(redirect_uri), "Already logged in."));
                }
                Err(err) => {
                    cata_log!(Warning, format!("JWT user lookup failed: {}", err.log_message()));
                }
            },
            Err(err) => {
                cata_log!(Warning, format!("JWT ID parsing failed: {}", err.log_message()));
            }
        }
    }

    Ok(app_context.render("index"))
}

#[get("/oops")]
pub async fn page_not_found(app_context: AppContext<'_>) -> Template {
    app_context.render("oops/index")
}

pub fn routes() -> Vec<Route> {
    routes![get_home, page_not_found, get_login, get_logout, get_register, post_login, post_register]
}

// HTMX partial routes for home module
#[get("/partials/login-form")]
fn login_form_partial(_app_context: AppContext<'_>) -> HtmxResult {
    Ok(HtmxSuccess::with_content("<form hx-post='/auth/login' hx-swap='outerHTML'> ... Login form fields ... </form>"))
}

#[get("/partials/register-form")]
fn register_form_partial(_app_context: AppContext<'_>) -> HtmxResult {
    Ok(HtmxSuccess::with_content("<form hx-post='/auth/register' hx-swap='outerHTML'> ... Register form fields ... </form>"))
}

pub fn partials() -> Vec<Route> {
    routes![login_form_partial, register_form_partial]
}
