use crate::cata_log;
use crate::middleware::*;
use crate::structs::*;
use chrono::{Duration, Utc};
use jsonwebtoken::{encode, EncodingKey, Header as JWTHeader};
use rocket::form::Form;
use rocket::http::{Cookie, CookieJar};
use rocket::response::{Flash, Redirect};
use rocket::uri;
use rocket::{get, post, routes, Route};
use rocket_dyn_templates::Template;
use std::env;

#[post("/login", data = "<login_form>")]
async fn post_login(login_form: Form<LoginForm>, cookies: &CookieJar<'_>, app_context: AppContext<'_>) -> Result<Flash<Redirect>, Flash<Redirect>> {
    let login = login_form.into_inner();

    if let Err(_) = verify_csrf_for_state_change(&app_context, &login.authenticity_token) {
        return Err(Flash::error(Redirect::to(uri!(get_login)), "Invalid request. Please try again."));
    }

    if let Ok(user) = Users::get_user_by_username(&login.username) {
        if user.verify_password(&login.password) {
            let expiration = Utc::now().checked_add_signed(Duration::seconds(86400)).unwrap().timestamp();
            let claims = Claims {
                sub: user.id.to_string(),
                exp: expiration as usize,
            };

            let token = encode(&JWTHeader::default(), &claims, &EncodingKey::from_secret(env::var("JWT_SECRET").unwrap().as_ref())).unwrap();

            cookies.add(Cookie::new("token", token));
            cookies.add(Cookie::new("user_id", user.id.to_string()));
            cata_log!(Info, format!("User {} logged in successfully", user.username));

            let redirect_uri = if Users::is_admin(user.id) {
                uri!(crate::routes::admin::get_dashboard)
            } else {
                uri!(crate::routes::user::get_dashboard)
            };

            return Ok(Flash::success(Redirect::to(redirect_uri), "Successfully logged in."));
        }
    }

    cata_log!(Warning, "Invalid login attempt");
    Err(Flash::error(Redirect::to(uri!(get_login)), "Invalid username or password."))
}

#[get("/logout")]
fn get_logout(cookies: &CookieJar<'_>) -> Flash<Redirect> {
    cookies.remove(Cookie::build("token"));
    cata_log!(Info, "User logged out");
    Flash::success(Redirect::to(uri!(get_login)), "Successfully logged out.")
}

#[get("/register")]
async fn get_register(app_context: AppContext<'_>) -> Template {
    cata_log!(Info, "Rendering registration page");
    app_context.render("auth/register")
}

#[post("/register", data = "<register_form>")]
async fn post_register(register_form: Form<RegisterForm>, app_context: AppContext<'_>) -> Flash<Redirect> {
    let register = register_form.into_inner();

    if let Err(_) = verify_csrf_for_state_change(&app_context, &register.authenticity_token) {
        return Flash::error(Redirect::to(uri!(get_register)), "Invalid request. Please try again.");
    }

    match Users::register_user(register) {
        Ok(()) => {
            cata_log!(Info, "User registered successfully");
            Flash::success(Redirect::to(uri!(get_login)), "Successfully registered.")
        }
        Err(err_msg) => {
            cata_log!(Error, format!("Registration error: {}", err_msg));
            Flash::error(Redirect::to(uri!(get_register)), err_msg)
        }
    }
}

#[get("/login")]
async fn get_login(app_context: AppContext<'_>, jwt: Option<JWT>) -> Result<Template, Flash<Redirect>> {
    if let Some(jwt) = jwt {
        if let Ok(user) = jwt_to_user(&jwt.0.sub) {
            let redirect_uri = if Users::is_admin(user.id) {
                uri!(crate::routes::admin::get_dashboard)
            } else {
                uri!(crate::routes::user::get_dashboard)
            };
            cata_log!(Info, format!("User {} is already logged in", user.username));
            return Err(Flash::success(Redirect::to(redirect_uri), "Already logged in."));
        } else {
            cata_log!(Warning, "JWT invalid");
        }
    }

    cata_log!(Info, "Rendering login page");
    Ok(app_context.render("auth/login"))
}

pub fn routes() -> Vec<Route> {
    routes![get_login, get_logout, get_register, post_login, post_register]
}
