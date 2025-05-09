use rocket::{get, http::Status, post, routes, Route};

use crate::{cata_log, middleware::*, services::*, structs::*};

#[get("/admin/partials/users_table")]
pub async fn users_table() -> HtmxResult {
    match Users::get_all_users_active().await {
        Ok(users) => {
            let table_html = TableBuilder::new(users).with_table_class("striped responsive-table").with_ignore("password_hash should_change_password").build();

            Ok(HtmxSuccess::with_content(table_html))
        }
        Err(error) => {
            cata_log!(Warning, format!("Error fetching users: {}", error.log_message()));
            Err(HtmxError::with_notification(error.status_code(), error.user_message()))
        }
    }
}

#[post("/admin/partials/create_duplicate_admin")]
pub async fn create_duplicate_admin() -> HtmxResult {
    let register_form = RegisterForm {
        username: "admin".to_string(),
        email: "duplicate@admin.com".to_string(),
        first_name: "Duplicate".to_string(),
        last_name: "Admin".to_string(),
        password: "password123".to_string(),
        confirm_password: "password123".to_string(),
        authenticity_token: "valid-token".to_string(),
    };

    match Users::register_user(register_form).await {
        Ok(_) => Ok(HtmxSuccess::with_notification("Admin created successfully")),
        Err(error) => Err(HtmxError::with_notification(error.status_code(), error.user_message())),
    }
}

#[post("/admin/partials/test_notification")]
pub async fn test_notification() -> HtmxResult {
    Ok(HtmxSuccess::with_notification("This is a test notification message that will be displayed in a notification div"))
}

#[post("/admin/partials/test_content")]
pub async fn test_content() -> HtmxResult {
    Ok(HtmxSuccess::with_content(
        "<div class='card-panel teal lighten-2 white-text'><i class='material-icons left'>check_circle</i>This is rendered HTML content</div>",
    ))
}

#[post("/admin/partials/test_error_with_notification")]
pub async fn test_error_with_notification() -> HtmxResult {
    Err(HtmxError::with_notification(Status::BadRequest, "This error includes a notification that can be shown in a toast"))
}

#[post("/admin/partials/test_warning")]
pub async fn test_warning() -> HtmxResult {
    Ok(HtmxWarning::with_notification("This is a warning notification message that will be displayed as a yellow toast"))
}

#[post("/admin/partials/test_info")]
pub async fn test_info() -> HtmxResult {
    Ok(HtmxInfo::with_notification("This is an information message that will be displayed as a blue toast"))
}

pub fn admin_partial_routes() -> Vec<Route> {
    routes![users_table, create_duplicate_admin, test_notification, test_content, test_error_with_notification, test_warning, test_info]
}
