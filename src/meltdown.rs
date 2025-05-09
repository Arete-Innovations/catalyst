use std::{fmt, io::Error as IoError};

use diesel::result::{DatabaseErrorKind, Error as DieselError};
use rocket::{
    http::Status,
    response::{Flash, Redirect},
    serde::json::Json,
    uri,
};
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Clone, PartialEq)]
pub enum MeltType {
    DatabaseConnection,
    DatabaseError,
    RecordNotFound,
    UniqueViolation,
    ForeignKeyViolation,
    CheckViolation,
    NotNullViolation,

    InvalidCredentials,
    ExpiredToken,
    InvalidToken,
    MissingToken,
    InsufficientPermissions,

    ValidationFailed,
    InvalidInput,
    MissingField,

    FileNotFound,
    FilePermissionDenied,
    FileOperationFailed,

    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    MethodNotAllowed,

    TemplateRenderFailed,

    SerializationFailed,
    DeserializationFailed,
    ConfigurationError,
    EnvironmentError,
    ExternalServiceError,

    Unknown,
}

#[derive(Debug)]
pub struct MeltDown {
    pub melt_type: MeltType,
    pub details: String,
    pub user_message: Option<String>,
    pub context: Option<std::collections::HashMap<String, String>>,
}

impl MeltDown {
    pub fn new(melt_type: MeltType, details: impl Into<String>) -> Self {
        Self {
            melt_type,
            details: details.into(),
            user_message: None,
            context: None,
        }
    }

    pub fn with_user_message(mut self, message: impl Into<String>) -> Self {
        self.user_message = Some(message.into());
        self
    }

    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        if self.context.is_none() {
            self.context = Some(std::collections::HashMap::new());
        }

        if let Some(context) = &mut self.context {
            context.insert(key.into(), value.into());
        }

        self
    }

    pub fn user_message(&self) -> String {
        if let Some(msg) = &self.user_message {
            return msg.clone();
        }

        match self.melt_type {
            MeltType::DatabaseConnection => "Unable to connect to database. Please try again later.".to_string(),
            MeltType::DatabaseError => "A database error occurred. Please try again later.".to_string(),
            MeltType::RecordNotFound => format!("{} not found.", self.details),
            MeltType::UniqueViolation => format!("{} already exists.", self.details),
            MeltType::ForeignKeyViolation => "Referenced data does not exist.".to_string(),
            MeltType::CheckViolation => "Data validation constraints were not met.".to_string(),
            MeltType::NotNullViolation => format!("{} is required.", self.details),

            MeltType::InvalidCredentials => "Invalid username or password.".to_string(),
            MeltType::ExpiredToken => "Your session has expired. Please login again.".to_string(),
            MeltType::InvalidToken => "Invalid authentication token.".to_string(),
            MeltType::MissingToken => "Authentication required.".to_string(),
            MeltType::InsufficientPermissions => "You don't have permission to perform this action.".to_string(),

            MeltType::ValidationFailed => {
                if self.details.is_empty() {
                    "Validation failed".to_string()
                } else {
                    format!("Validation failed: {}", self.details)
                }
            }
            MeltType::InvalidInput => format!("Invalid input: {}", self.details),
            MeltType::MissingField => format!("{} is required.", self.details),

            MeltType::FileNotFound => format!("File not found: {}", self.details),
            MeltType::FilePermissionDenied => "Permission denied accessing file.".to_string(),
            MeltType::FileOperationFailed => "File operation failed.".to_string(),

            MeltType::BadRequest => format!("Bad request: {}", self.details),
            MeltType::Unauthorized => format!("Unauthorized: {}", self.details),
            MeltType::Forbidden => format!("Forbidden: {}", self.details),
            MeltType::NotFound => format!("{} not found.", self.details),
            MeltType::MethodNotAllowed => format!("Method {} not allowed.", self.details),

            MeltType::TemplateRenderFailed => "Unable to render page.".to_string(),

            MeltType::SerializationFailed => "Data processing error.".to_string(),
            MeltType::DeserializationFailed => "Data processing error.".to_string(),
            MeltType::ConfigurationError => "Application configuration error.".to_string(),
            MeltType::EnvironmentError => "Environment setup error.".to_string(),
            MeltType::ExternalServiceError => "External service error.".to_string(),

            MeltType::Unknown => "An unexpected error occurred.".to_string(),
        }
    }

    pub fn log_message(&self) -> String {
        let mut message = format!("[{}] {}", self.melt_type_str(), self.details);

        if let Some(context) = &self.context {
            for (key, value) in context {
                message.push_str(&format!(" | {}={}", key, value));
            }
        }

        message
    }

    fn melt_type_str(&self) -> &'static str {
        match self.melt_type {
            MeltType::DatabaseConnection => "DatabaseConnection",
            MeltType::DatabaseError => "DatabaseError",
            MeltType::RecordNotFound => "RecordNotFound",
            MeltType::UniqueViolation => "UniqueViolation",
            MeltType::ForeignKeyViolation => "ForeignKeyViolation",
            MeltType::CheckViolation => "CheckViolation",
            MeltType::NotNullViolation => "NotNullViolation",
            MeltType::InvalidCredentials => "InvalidCredentials",
            MeltType::ExpiredToken => "ExpiredToken",
            MeltType::InvalidToken => "InvalidToken",
            MeltType::MissingToken => "MissingToken",
            MeltType::InsufficientPermissions => "InsufficientPermissions",
            MeltType::ValidationFailed => "ValidationFailed",
            MeltType::InvalidInput => "InvalidInput",
            MeltType::MissingField => "MissingField",
            MeltType::FileNotFound => "FileNotFound",
            MeltType::FilePermissionDenied => "FilePermissionDenied",
            MeltType::FileOperationFailed => "FileOperationFailed",
            MeltType::BadRequest => "BadRequest",
            MeltType::Unauthorized => "Unauthorized",
            MeltType::Forbidden => "Forbidden",
            MeltType::NotFound => "NotFound",
            MeltType::MethodNotAllowed => "MethodNotAllowed",
            MeltType::TemplateRenderFailed => "TemplateRenderFailed",
            MeltType::SerializationFailed => "SerializationFailed",
            MeltType::DeserializationFailed => "DeserializationFailed",
            MeltType::ConfigurationError => "ConfigurationError",
            MeltType::EnvironmentError => "EnvironmentError",
            MeltType::ExternalServiceError => "ExternalServiceError",
            MeltType::Unknown => "Unknown",
        }
    }

    pub fn status_code(&self) -> Status {
        match self.melt_type {
            MeltType::InvalidCredentials => Status::Unauthorized,
            MeltType::ExpiredToken => Status::Unauthorized,
            MeltType::InvalidToken => Status::Unauthorized,
            MeltType::MissingToken => Status::Unauthorized,
            MeltType::InsufficientPermissions => Status::Forbidden,
            MeltType::ValidationFailed => Status::BadRequest,
            MeltType::InvalidInput => Status::BadRequest,
            MeltType::MissingField => Status::BadRequest,
            MeltType::BadRequest => Status::BadRequest,
            MeltType::Unauthorized => Status::Unauthorized,
            MeltType::Forbidden => Status::Forbidden,
            MeltType::NotFound => Status::NotFound,
            MeltType::MethodNotAllowed => Status::MethodNotAllowed,
            MeltType::RecordNotFound => Status::NotFound,

            MeltType::DatabaseConnection => Status::InternalServerError,
            MeltType::DatabaseError => Status::InternalServerError,
            MeltType::UniqueViolation => Status::Conflict,
            MeltType::ForeignKeyViolation => Status::Conflict,
            MeltType::CheckViolation => Status::BadRequest,
            MeltType::NotNullViolation => Status::BadRequest,
            MeltType::FileNotFound => Status::NotFound,
            MeltType::FilePermissionDenied => Status::Forbidden,
            MeltType::FileOperationFailed => Status::InternalServerError,
            MeltType::TemplateRenderFailed => Status::InternalServerError,
            MeltType::SerializationFailed => Status::InternalServerError,
            MeltType::DeserializationFailed => Status::InternalServerError,
            MeltType::ConfigurationError => Status::InternalServerError,
            MeltType::EnvironmentError => Status::InternalServerError,
            MeltType::ExternalServiceError => Status::ServiceUnavailable,
            MeltType::Unknown => Status::InternalServerError,
        }
    }

    pub fn log(&self) {
        use crate::cata_log;

        match self.status_code().code {
            400..=499 => cata_log!(Warning, self.log_message()),
            _ => cata_log!(Error, self.log_message()),
        }
    }
}

impl fmt::Display for MeltDown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.user_message())
    }
}

impl std::error::Error for MeltDown {
}

impl From<bcrypt::BcryptError> for MeltDown {
    fn from(err: bcrypt::BcryptError) -> Self {
        MeltDown::new(MeltType::ConfigurationError, format!("Password hashing error: {}", err))
    }
}

impl From<jsonwebtoken::errors::Error> for MeltDown {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        match err.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => MeltDown::new(MeltType::ExpiredToken, format!("JWT token expired: {}", err)),
            jsonwebtoken::errors::ErrorKind::InvalidToken => MeltDown::new(MeltType::InvalidToken, format!("Invalid JWT token: {}", err)),
            jsonwebtoken::errors::ErrorKind::InvalidSignature => MeltDown::new(MeltType::InvalidToken, format!("Invalid JWT signature: {}", err)),

            jsonwebtoken::errors::ErrorKind::InvalidKeyFormat => MeltDown::new(MeltType::ConfigurationError, format!("Invalid JWT key format: {}", err)),
            jsonwebtoken::errors::ErrorKind::Base64(_) => MeltDown::new(MeltType::ConfigurationError, format!("JWT Base64 error: {}", err)),
            jsonwebtoken::errors::ErrorKind::Json(_) => MeltDown::new(MeltType::SerializationFailed, format!("JWT serialization error: {}", err)),
            jsonwebtoken::errors::ErrorKind::Utf8(_) => MeltDown::new(MeltType::SerializationFailed, format!("JWT UTF-8 encoding error: {}", err)),

            _ => MeltDown::new(MeltType::ExternalServiceError, format!("JWT operation failed: {}", err)),
        }
    }
}

impl From<std::env::VarError> for MeltDown {
    fn from(err: std::env::VarError) -> Self {
        MeltDown::new(MeltType::EnvironmentError, format!("Environment variable error: {}", err))
    }
}

impl From<DieselError> for MeltDown {
    fn from(err: DieselError) -> Self {
        match err {
            DieselError::DatabaseError(kind, ref info) => match kind {
                DatabaseErrorKind::UniqueViolation => {
                    let field = if let Some(constraint) = info.constraint_name() {
                        if constraint.contains("username") || constraint.contains("users_username_key") {
                            "Username"
                        } else if constraint.contains("email") || constraint.contains("users_email_key") {
                            "Email"
                        } else if constraint.contains("api_key") || constraint.contains("api_keys_key_key") {
                            "API Key"
                        } else {
                            "This value"
                        }
                    } else {
                        "This value"
                    };

                    let mut error = MeltDown::new(MeltType::UniqueViolation, field);
                    if let Some(constraint) = info.constraint_name() {
                        error = error.with_context("constraint", constraint);
                    }
                    error
                }
                DatabaseErrorKind::ForeignKeyViolation => {
                    let mut error = MeltDown::new(MeltType::ForeignKeyViolation, "Related record not found");
                    if let Some(constraint) = info.constraint_name() {
                        error = error.with_context("constraint", constraint);
                    }
                    error
                }
                DatabaseErrorKind::CheckViolation => {
                    let mut error = MeltDown::new(MeltType::CheckViolation, "Check constraint failed");
                    if let Some(constraint) = info.constraint_name() {
                        error = error.with_context("constraint", constraint);
                    }
                    error
                }
                DatabaseErrorKind::NotNullViolation => {
                    let column = info.column_name().unwrap_or("Unknown field").to_string();
                    let mut error = MeltDown::new(MeltType::NotNullViolation, column);
                    if let Some(table) = info.table_name() {
                        error = error.with_context("table", table);
                    }
                    error
                }
                _ => MeltDown::new(MeltType::DatabaseConnection, format!("Database error: {:?}", err)),
            },
            DieselError::NotFound => MeltDown::new(MeltType::RecordNotFound, "Record"),
            DieselError::RollbackTransaction => MeltDown::new(MeltType::DatabaseConnection, "Transaction rolled back"),
            DieselError::AlreadyInTransaction => MeltDown::new(MeltType::DatabaseConnection, "Already in transaction"),
            DieselError::QueryBuilderError(e) => MeltDown::new(MeltType::DatabaseConnection, format!("Query builder error: {}", e)),
            DieselError::DeserializationError(e) => MeltDown::new(MeltType::DeserializationFailed, format!("Failed to deserialize result: {}", e)),
            DieselError::SerializationError(e) => MeltDown::new(MeltType::SerializationFailed, format!("Failed to serialize data: {}", e)),
            _ => MeltDown::new(MeltType::DatabaseConnection, format!("Database error: {:?}", err)),
        }
    }
}

impl From<IoError> for MeltDown {
    fn from(err: IoError) -> Self {
        use std::io::ErrorKind;

        match err.kind() {
            ErrorKind::NotFound => MeltDown::new(MeltType::FileNotFound, err.to_string()),
            ErrorKind::PermissionDenied => MeltDown::new(MeltType::FilePermissionDenied, err.to_string()),
            _ => MeltDown::new(MeltType::FileOperationFailed, err.to_string()),
        }
    }
}

impl From<MeltDown> for Flash<Redirect> {
    fn from(error: MeltDown) -> Self {
        error.log();

        Flash::error(Redirect::to(uri!("/auth/login")), error.user_message())
    }
}

impl<'r> From<MeltDown> for Json<serde_json::Value> {
    fn from(error: MeltDown) -> Self {
        error.log();

        let mut response = json!({
            "error": {
                "code": error.status_code().code,
                "type": error.melt_type_str(),
                "message": error.user_message()
            }
        });

        if let Some(context) = &error.context {
            response["error"]["context"] = json!(context);
        }

        Json(response)
    }
}

#[derive(Serialize)]
pub struct ApiError {
    pub error: ApiErrorDetail,
}

#[derive(Serialize)]
pub struct ApiErrorDetail {
    pub code: u16,
    pub melt_type: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<std::collections::HashMap<String, String>>,
}

impl From<MeltDown> for ApiError {
    fn from(error: MeltDown) -> Self {
        error.log();

        ApiError {
            error: ApiErrorDetail {
                code: error.status_code().code,
                melt_type: error.melt_type_str().to_string(),
                message: error.user_message(),
                context: error.context.clone(),
            },
        }
    }
}

impl From<&str> for MeltDown {
    fn from(message: &str) -> Self {
        MeltDown::new(MeltType::Unknown, message)
    }
}

impl From<String> for MeltDown {
    fn from(message: String) -> Self {
        MeltDown::new(MeltType::Unknown, message)
    }
}

impl MeltDown {
    pub fn db_connection(details: impl Into<String>) -> Self {
        Self::new(MeltType::DatabaseConnection, details)
    }

    pub fn record_not_found(entity: impl Into<String>) -> Self {
        Self::new(MeltType::RecordNotFound, entity)
    }

    pub fn unique_violation(field: impl Into<String>) -> Self {
        Self::new(MeltType::UniqueViolation, field)
    }

    pub fn invalid_credentials() -> Self {
        Self::new(MeltType::InvalidCredentials, "Invalid username or password")
    }

    pub fn expired_token() -> Self {
        Self::new(MeltType::ExpiredToken, "Token has expired")
    }

    pub fn invalid_token(details: impl Into<String>) -> Self {
        Self::new(MeltType::InvalidToken, details)
    }

    pub fn missing_token() -> Self {
        Self::new(MeltType::MissingToken, "Authentication token is missing")
    }

    pub fn insufficient_permissions() -> Self {
        Self::new(MeltType::InsufficientPermissions, "Insufficient permissions for this action")
    }

    pub fn validation_failed(details: impl Into<String>) -> Self {
        Self::new(MeltType::ValidationFailed, details)
    }

    pub fn invalid_input(details: impl Into<String>) -> Self {
        Self::new(MeltType::InvalidInput, details)
    }

    pub fn missing_field(field: impl Into<String>) -> Self {
        Self::new(MeltType::MissingField, field)
    }
}
