use rocket::{get, http::Status, post, routes, Route};

use crate::{cata_log, middleware::*, services::*, structs::*};

#[post("/<tenant>/admin/partials/create_duplicate_admin")]
pub async fn create_duplicate_admin(tenant: &str) -> HtmxResult {
    let register_form = RegisterForm {
        username: "admin".to_string(),
        email: "duplicate@admin.com".to_string(),
        first_name: "Duplicate".to_string(),
        last_name: "Admin".to_string(),
        password: "password123".to_string(),
        confirm_password: "password123".to_string(),
        authenticity_token: "valid-token".to_string(),
    };

    match Users::register_user(register_form, tenant).await {
        Ok(_) => Ok(HtmxSuccess::with_notification("Admin created successfully")),
        Err(error) => Err(HtmxError::with_notification(error.status_code(), error.user_message())),
    }
}

#[post("/<tenant>/admin/partials/test_notification")]
pub async fn test_notification(tenant: &str) -> HtmxResult {
    Ok(HtmxSuccess::with_notification(format!(
        "This is a test notification message for tenant: {} that will be displayed in a notification div",
        tenant
    )))
}

#[post("/<tenant>/admin/partials/test_content")]
pub async fn test_content(tenant: &str) -> HtmxResult {
    Ok(HtmxSuccess::with_content(format!(
        "<div class='card-panel teal lighten-2 white-text'><i class='material-icons left'>check_circle</i>This is rendered HTML content for tenant: {}</div>",
        tenant
    )))
}

#[post("/<tenant>/admin/partials/test_error_with_notification")]
pub async fn test_error_with_notification(tenant: &str) -> HtmxResult {
    Err(HtmxError::with_notification(
        Status::BadRequest,
        format!("This error includes a notification for tenant: {} that can be shown in a toast", tenant),
    ))
}

#[post("/<tenant>/admin/partials/test_warning")]
pub async fn test_warning(tenant: &str) -> HtmxResult {
    Ok(HtmxWarning::with_notification(format!(
        "This is a warning notification message for tenant: {} that will be displayed as a yellow toast",
        tenant
    )))
}

#[post("/<tenant>/admin/partials/test_info")]
pub async fn test_info(tenant: &str) -> HtmxResult {
    Ok(HtmxInfo::with_notification(format!("This is an information message for tenant: {} that will be displayed as a blue toast", tenant)))
}

pub fn admin_partial_routes() -> Vec<Route> {
    routes![create_duplicate_admin, test_content, test_error_with_notification, test_info, test_notification, test_warning,]
}
