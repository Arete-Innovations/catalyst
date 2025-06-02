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

use crate::{cata_log, meltdown::*, middleware::*, routes::*, services::default::*, structs::*, vessel::structs::Vessel};

#[post("/<tenant>/auth/login", data = "<login_form>")]
async fn post_login(tenant: &str, login_form: Form<LoginForm>, cookies: &CookieJar<'_>, app_context: AppContext<'_>, jwt: Option<JWT>) -> Result<Flash<Redirect>, Flash<Redirect>> {
    match Vessel::tenant_exists(tenant).await {
        Ok(exists) => {
            if !exists {
                cata_log!(Warning, format!("Attempted to login to non-existent tenant: {}", tenant));
                let error = MeltDown::new(MeltType::NotFound, "Tenant not found");
                return Err(Flash::error(Redirect::to("/vessel/auth/login"), error.user_message()));
            }
        }
        Err(e) => {
            cata_log!(Error, format!("Error checking tenant existence: {}", e.log_message()));
            let error = MeltDown::new(MeltType::DatabaseError, "Database error");
            return Err(Flash::error(Redirect::to("/vessel/auth/login"), error.user_message()));
        }
    }

    let login = login_form.into_inner();

    if let Err(csrf_error) = verify_csrf_for_state_change(&app_context, &login.authenticity_token) {
        return Err(Flash::error(Redirect::to(uri!(get_login(tenant))), csrf_error.user_message()));
    }

    if let Some(jwt) = jwt {
        let should_clear = if jwt.is_vessel_auth() {
            cata_log!(Info, "User is currently logged into vessel system, clearing previous auth to switch to tenant system");
            true
        } else if jwt.is_tenant_auth() {
            if let Some(jwt_tenant) = jwt.get_tenant_name() {
                if *jwt_tenant != tenant {
                    cata_log!(Info, format!("User is switching from tenant {} to tenant {}, clearing previous auth", jwt_tenant, tenant));
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        if should_clear {
            cookies.remove(Cookie::new("access_token", ""));
            cookies.remove(Cookie::new("refresh_token", ""));
            cookies.remove(Cookie::new("user_id", ""));
        }
    }

    let user = match Users::get_user_by_username(login.username.clone(), tenant).await {
        Ok(user) => user,
        Err(_) => {
            cata_log!(Warning, format!("Login attempt with invalid username: {} for tenant: {}", login.username, tenant));
            return Err(Flash::error(Redirect::to(uri!(get_login(tenant))), MeltDown::invalid_credentials().user_message()));
        }
    };

    match user.verify_password(login.password.clone()).await {
        Ok(true) => {
            let remember = login.remember_me.unwrap_or(false);

            let device_info = Some(format!("Login at {} for tenant: {}", Utc::now().to_rfc3339(), tenant));

            crate::services::default::jwt_service::set_current_tenant(tenant);

            let token_pair = match crate::services::default::jwt_service::generate_token_pair(&user, remember, device_info) {
                Ok(pair) => pair,
                Err(error) => {
                    return Err(Flash::error(Redirect::to(uri!(get_login(tenant))), error.user_message()));
                }
            };

            cookies.add(Cookie::build(Cookie::new("access_token", token_pair.access_token)).http_only(true).secure(true).build());
            cookies.add(Cookie::build(Cookie::new("refresh_token", token_pair.refresh_token)).http_only(true).secure(true).build());
            cookies.add(Cookie::build(Cookie::new("user_id", user.id.to_string())).http_only(true).secure(true).build());

            let access_expiry = token_pair.access_claims.exp as i64 - Utc::now().timestamp();
            let refresh_expiry = token_pair.refresh_claims.exp as i64 - Utc::now().timestamp();

            cata_log!(
                Debug,
                format!(
                    "Issuing token pair for user {}: access token expires in {}s, refresh token expires in {}s, auth_system: {:?} (tenant: {})",
                    user.id, access_expiry, refresh_expiry, token_pair.access_claims.auth_system, tenant
                )
            );
            cata_log!(Info, format!("User {} logged in successfully (tenant: {})", user.username, tenant));

            let is_admin = user.role == "admin";
            let redirect_uri = if is_admin { uri!(admin::get_admin_dashboard(tenant)) } else { uri!(user::get_user_dashboard(tenant)) };

            Ok(Flash::success(Redirect::to(redirect_uri), "Successfully logged in."))
        }
        Ok(false) | Err(_) => {
            cata_log!(Warning, format!("Failed login attempt for user: {} (tenant: {})", login.username, tenant));
            Err(Flash::error(Redirect::to(uri!(get_login(tenant))), MeltDown::invalid_credentials().user_message()))
        }
    }
}

#[get("/<tenant>/auth/logout")]
fn get_logout(tenant: &str, cookies: &CookieJar<'_>, jwt: Option<JWT>) -> Flash<Redirect> {
    cata_log!(Info, format!("Tenant logout initiated for tenant: {}", tenant));

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

    let static_paths = [
        "/",
        "/vessel",
        "/vessel/auth",
        "/auth",
        &format!("/{}", tenant),
        &format!("/{}/auth", tenant),
        &format!("/{}/admin", tenant),
        &format!("/{}/user", tenant),
    ];

    for path in &static_paths {
        for &name in &cookie_names {
            remove_cookie_with_all_attributes(cookies, name, Some(path));
        }
    }

    for &name in &cookie_names {
        remove_cookie_with_all_attributes(cookies, name, None);
    }

    if let Some(ref jwt) = jwt {
        if let Some(jwt_tenant) = jwt.get_tenant_name() {
            if jwt_tenant != tenant {
                let jwt_tenant_paths = [&format!("/{}", jwt_tenant), &format!("/{}/auth", jwt_tenant), &format!("/{}/admin", jwt_tenant), &format!("/{}/user", jwt_tenant)];

                for path in &jwt_tenant_paths {
                    for &name in &cookie_names {
                        remove_cookie_with_all_attributes(cookies, name, Some(path));
                    }
                }

                cata_log!(Info, format!("Cleared cookies for JWT tenant: {}", jwt_tenant));
            }
        }
    }

    if let Some(jwt) = jwt {
        let user_id = jwt.user_id();
        cata_log!(Info, format!("User {} logged out (tenant: {})", user_id, tenant));
    } else {
        cata_log!(Info, format!("Anonymous user logged out (tenant: {})", tenant));
    }

    cata_log!(Info, format!("Tenant logout complete, redirecting to login page for: {}", tenant));
    Flash::success(Redirect::to(uri!(get_login(tenant))), "Successfully logged out.")
}

#[get("/<tenant>/auth/register")]
async fn get_register(tenant: &str, app_context: AppContext<'_>) -> Result<Template, Flash<Redirect>> {
    match Vessel::tenant_exists(tenant).await {
        Ok(exists) => {
            if !exists {
                cata_log!(Warning, format!("Attempted to access registration for non-existent tenant: {}", tenant));
                let error = MeltDown::new(MeltType::NotFound, "Tenant not found");
                return Err(Flash::error(Redirect::to("/vessel/auth/login"), error.user_message()));
            }
        }
        Err(e) => {
            cata_log!(Error, format!("Error checking tenant existence: {}", e.log_message()));
            let error = MeltDown::new(MeltType::DatabaseError, "Database error");
            return Err(Flash::error(Redirect::to("/vessel/auth/login"), error.user_message()));
        }
    }

    cata_log!(Info, format!("Rendering registration page for tenant: {}", tenant));
    let tenant_data = TenantData::new(tenant, ());
    Ok(app_context.render_with("auth/register", tenant_data))
}

#[post("/<tenant>/auth/register", data = "<register_form>")]
async fn post_register(tenant: &str, register_form: Form<RegisterForm>, app_context: AppContext<'_>) -> Result<Flash<Redirect>, Flash<Redirect>> {
    match Vessel::tenant_exists(tenant).await {
        Ok(exists) => {
            if !exists {
                cata_log!(Warning, format!("Attempted to register for non-existent tenant: {}", tenant));
                let error = MeltDown::new(MeltType::NotFound, "Tenant not found");
                return Err(Flash::error(Redirect::to("/vessel/auth/login"), error.user_message()));
            }
        }
        Err(e) => {
            cata_log!(Error, format!("Error checking tenant existence: {}", e.log_message()));
            let error = MeltDown::new(MeltType::DatabaseError, "Database error");
            return Err(Flash::error(Redirect::to("/vessel/auth/login"), error.user_message()));
        }
    }

    let register = register_form.into_inner();

    if let Err(csrf_error) = verify_csrf_for_state_change(&app_context, &register.authenticity_token) {
        return Err(Flash::error(Redirect::to(uri!(get_register(tenant))), csrf_error.user_message()));
    }

    match Users::register_user(register.clone(), tenant).await {
        Ok(()) => {
            cata_log!(Info, format!("User registered successfully (tenant: {})", tenant));
            Ok(Flash::success(Redirect::to(uri!(get_login(tenant))), "Successfully registered."))
        }
        Err(err_msg) => {
            cata_log!(Error, format!("Registration error (tenant: {}): {}", tenant, err_msg.log_message()));
            Err(Flash::error(Redirect::to(uri!(get_register(tenant))), err_msg.user_message()))
        }
    }
}

#[get("/<tenant>/auth/login")]
async fn get_login(tenant: &str, app_context: AppContext<'_>, jwt: Option<JWT>, cookies: &CookieJar<'_>) -> Result<Template, Flash<Redirect>> {
    match Vessel::tenant_exists(tenant).await {
        Ok(exists) => {
            if !exists {
                cata_log!(Warning, format!("Attempted to access login for non-existent tenant: {}", tenant));
                let error = MeltDown::new(MeltType::NotFound, "Tenant not found");
                return Err(Flash::error(Redirect::to("/vessel/auth/login"), error.user_message()));
            }
        }
        Err(e) => {
            cata_log!(Error, format!("Error checking tenant existence: {}", e.log_message()));
            let error = MeltDown::new(MeltType::DatabaseError, "Database error");
            return Err(Flash::error(Redirect::to("/vessel/auth/login"), error.user_message()));
        }
    }

    let mut context_data = serde_json::Map::new();

    context_data.insert("title".to_string(), serde_json::Value::String(format!("{} Login", tenant)));

    let show_login = if let Some(ref jwt) = jwt {
        if jwt.is_tenant_auth() {
            if let Some(jwt_tenant) = jwt.get_tenant_name() {
                if *jwt_tenant != tenant {
                    cata_log!(Warning, format!("User {} is logged in for tenant {} but trying to access tenant {}", jwt.get_username(), jwt_tenant, tenant));

                    cookies.remove(Cookie::new("access_token", ""));
                    cookies.remove(Cookie::new("refresh_token", ""));
                    cookies.remove(Cookie::new("user_id", ""));

                    return Ok(app_context.render_with("auth/login", TenantData::new(tenant, context_data)));
                }
            }

            let is_admin = jwt.is_admin();
            let redirect_uri = if is_admin { uri!(admin::get_admin_dashboard(tenant)) } else { uri!(user::get_user_dashboard(tenant)) };
            cata_log!(Info, format!("User {} is already logged in to tenant system (tenant: {})", jwt.get_username(), tenant));
            return Err(Flash::success(Redirect::to(redirect_uri), "Already logged in to tenant system."));
        } else {
            cata_log!(Info, format!("User {} is logged in to vessel system, showing tenant login form (tenant: {})", jwt.get_username(), tenant));

            context_data.insert("jwt_role".to_string(), serde_json::Value::String(jwt.get_role().to_string()));
            context_data.insert("jwt_auth_system".to_string(), serde_json::Value::String(format!("{:?}", jwt.get_auth_system()).to_lowercase()));
            if let Some(tenant) = jwt.get_tenant_name() {
                context_data.insert("jwt_tenant".to_string(), serde_json::Value::String(tenant.clone()));
            }

            true
        }
    } else {
        cata_log!(Info, format!("No JWT, showing normal login page (tenant: {})", tenant));
        true
    };

    if show_login {
        cata_log!(Info, format!("Rendering login page for tenant: {}", tenant));
        let tenant_data = TenantData::new(tenant, context_data);
        Ok(app_context.render_with("auth/login", tenant_data))
    } else {
        let error = MeltDown::new(MeltType::Unknown, "Unexpected authentication state");
        Err(Flash::error(Redirect::to(uri!(get_login(tenant))), error.user_message()))
    }
}

#[get("/<tenant>")]
pub async fn get_home(tenant: &str, app_context: AppContext<'_>, jwt: Option<JWT>) -> Result<Template, Flash<Redirect>> {
    match Vessel::tenant_exists(tenant).await {
        Ok(exists) => {
            if !exists {
                cata_log!(Warning, format!("Attempted to access non-existent tenant: {}", tenant));
                let error = MeltDown::new(MeltType::NotFound, "Tenant not found");
                return Err(Flash::error(Redirect::to("/vessel/auth/login"), error.user_message()));
            }
        }
        Err(e) => {
            cata_log!(Error, format!("Error checking tenant existence: {}", e.log_message()));
            let error = MeltDown::new(MeltType::DatabaseError, "Database error");
            return Err(Flash::error(Redirect::to("/vessel/auth/login"), error.user_message()));
        }
    }

    let mut context_data = serde_json::Map::new();

    context_data.insert("title".to_string(), serde_json::Value::String(format!("Welcome to {}", tenant)));

    let show_home = if let Some(ref jwt) = jwt {
        if jwt.is_tenant_auth() {
            let is_admin = jwt.is_admin();
            let redirect_uri = if is_admin { uri!(admin::get_admin_dashboard(tenant)) } else { uri!(user::get_user_dashboard(tenant)) };
            cata_log!(Info, format!("User {} is already logged in to tenant system, redirecting to dashboard (tenant: {})", jwt.get_username(), tenant));
            return Err(Flash::success(Redirect::to(redirect_uri), "Already logged in to tenant system."));
        } else {
            cata_log!(Info, format!("User {} is logged in to vessel system, showing tenant home page (tenant: {})", jwt.get_username(), tenant));
            context_data.insert("jwt_role".to_string(), serde_json::Value::String(jwt.get_role().to_string()));
            context_data.insert("jwt_auth_system".to_string(), serde_json::Value::String(format!("{:?}", jwt.get_auth_system()).to_lowercase()));
            if let Some(tenant) = jwt.get_tenant_name() {
                context_data.insert("jwt_tenant".to_string(), serde_json::Value::String(tenant.clone()));
            }

            true
        }
    } else {
        true
    };

    if show_home {
        let tenant_data = TenantData::new(tenant, context_data);
        Ok(app_context.render_with("index", tenant_data))
    } else {
        let error = MeltDown::new(MeltType::Unknown, "Unexpected authentication state");
        Err(Flash::error(Redirect::to(uri!(get_login(tenant))), error.user_message()))
    }
}

#[post("/<tenant>/auth/refresh")]
async fn refresh_token(tenant: &str, cookies: &CookieJar<'_>) -> Result<(), Flash<Redirect>> {
    let refresh_token = match cookies.get("refresh_token") {
        Some(cookie) => cookie.value().to_string(),
        None => {
            cata_log!(Warning, format!("Refresh attempt without refresh token (tenant: {})", tenant));
            return Err(Flash::error(Redirect::to(uri!(get_login(tenant))), "Session expired. Please log in again."));
        }
    };

    let token_info = match crate::services::default::jwt_service::validate_refresh_token(&refresh_token) {
        Ok(info) => info,
        Err(error) => {
            cata_log!(Warning, format!("Invalid refresh token (tenant: {}): {}", tenant, error.log_message()));

            cookies.remove(Cookie::new("access_token", ""));
            cookies.remove(Cookie::new("refresh_token", ""));
            cookies.remove(Cookie::new("user_id", ""));

            return Err(Flash::error(Redirect::to(uri!(get_login(tenant))), "Session expired. Please log in again."));
        }
    };

    let user_id = token_info.user_id;

    crate::services::default::token_registry::mark_refresh_token_used(tenant, user_id, &token_info.jti);

    let user = match Users::get_user_by_id(user_id, tenant).await {
        Ok(user) => user,
        Err(error) => {
            cata_log!(Error, format!("Failed to get user {} (tenant: {}): {}", user_id, tenant, error.log_message()));
            return Err(Flash::error(Redirect::to(uri!(get_login(tenant))), "User account issue. Please log in again."));
        }
    };

    crate::services::default::jwt_service::set_current_tenant(tenant);

    let token_pair = match crate::services::default::jwt_service::generate_token_pair(&user, token_info.remember, token_info.device_info) {
        Ok(pair) => pair,
        Err(error) => {
            cata_log!(Error, format!("Failed to generate new tokens (tenant: {}): {}", tenant, error.log_message()));
            return Err(Flash::error(Redirect::to(uri!(get_login(tenant))), "Authentication error. Please log in again."));
        }
    };

    cookies.add(Cookie::build(Cookie::new("access_token", token_pair.access_token)).http_only(true).secure(true).build());
    cookies.add(Cookie::build(Cookie::new("refresh_token", token_pair.refresh_token)).http_only(true).secure(true).build());
    cookies.add(Cookie::build(Cookie::new("user_id", user_id.to_string())).http_only(true).secure(true).build());

    cata_log!(Info, format!("Refreshed tokens for user {} (tenant: {})", user_id, tenant));

    Ok(())
}

pub fn routes() -> Vec<Route> {
    routes![get_home, get_login, get_logout, get_register, post_login, post_register, refresh_token]
}
