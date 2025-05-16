use std::{cell::RefCell, env, thread};

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header as JWTHeader, Validation};
use uuid::Uuid;

use crate::{
    bootstrap::*,
    cata_log,
    meltdown::*,
    middleware::jwt::{AuthSystem, Claims, TokenType},
    services::*,
    structs::*,
    vessel::structs::Vessel,
};

thread_local! {
    pub static CURRENT_TENANT: RefCell<Option<String>> = RefCell::new(None);
}

pub fn set_current_tenant(tenant: &str) {
    CURRENT_TENANT.with(|current| {
        *current.borrow_mut() = Some(tenant.to_string());
    });
}

pub fn get_current_tenant() -> Option<String> {
    CURRENT_TENANT.with(|current| current.borrow().clone())
}

#[derive(Debug, Clone)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub access_claims: Claims,
    pub refresh_claims: Claims,
}

#[derive(Debug)]
pub struct RefreshTokenInfo {
    pub jti: String,
    pub user_id: i32,
    pub token_version: u32,
    pub remember: bool,
    pub device_info: Option<String>,
}

pub fn get_jwt_settings() -> JwtSettings {
    match APP_CONFIG.get() {
        Some(config) => config.settings.jwt.clone(),
        None => {
            cata_log!(Warning, "Using default JWT settings as APP_CONFIG was not initialized");
            JwtSettings::default()
        }
    }
}

pub fn generate_access_token(user: &Users, refresh_jti: Option<String>, device_info: Option<String>) -> Result<(String, Claims), MeltDown> {
    let jwt_settings = get_jwt_settings();

    let expiry_duration = Duration::minutes(jwt_settings.access_token_expiry_mins as i64);

    let now = Utc::now();
    let expiration = now.checked_add_signed(expiry_duration).unwrap_or(now).timestamp() as usize;
    let issued_at = now.timestamp() as usize;

    let jti = Uuid::new_v4().to_string();

    let tenant_name = get_current_tenant().unwrap_or_else(|| "postgres".to_string());

    let token_version = token_registry::get_token_version(&tenant_name, user.id);

    token_registry::register_user(&tenant_name, user.id);

    let claims = Claims {
        sub: user.id.to_string(),
        username: user.username.clone(),
        role: user.role.clone(),
        exp: expiration,
        iat: issued_at,
        nbf: issued_at,
        jti,
        token_type: TokenType::Access,
        ver: token_version,
        remember: false,
        refresh_jti,
        device_info,
        tenant_name: Some(tenant_name),
        auth_system: AuthSystem::Tenant,
    };

    let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "your-256-bit-secret".to_string());

    match encode(&JWTHeader::default(), &claims, &EncodingKey::from_secret(secret.as_bytes())) {
        Ok(token) => Ok((token, claims)),
        Err(e) => {
            let error_message = format!("Error encoding JWT: {}", e);
            cata_log!(Error, &error_message);
            Err(MeltDown::new(MeltType::Unknown, "Failed to generate JWT token").with_context("error", &error_message))
        }
    }
}

pub fn generate_access_token_for_vessel(vessel: &Vessel, refresh_jti: Option<String>, device_info: Option<String>) -> Result<(String, Claims), MeltDown> {
    let jwt_settings = get_jwt_settings();

    let expiry_duration = Duration::minutes(jwt_settings.access_token_expiry_mins as i64);

    let now = Utc::now();
    let expiration = now.checked_add_signed(expiry_duration).unwrap_or(now).timestamp() as usize;
    let issued_at = now.timestamp() as usize;

    let jti = Uuid::new_v4().to_string();

    let tenant_name = vessel.name.clone();
    let user_id = vessel.id;

    let token_version = token_registry::get_token_version(&tenant_name, user_id);

    token_registry::register_user(&tenant_name, user_id);

    let claims = Claims {
        sub: vessel.id.to_string(),
        username: vessel.username.clone(),
        role: "vessel".to_string(),
        exp: expiration,
        iat: issued_at,
        nbf: issued_at,
        jti,
        token_type: TokenType::Access,
        ver: token_version,
        remember: false,
        refresh_jti,
        device_info,
        tenant_name: Some(tenant_name),
        auth_system: AuthSystem::Vessel,
    };

    let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "your-256-bit-secret".to_string());

    match encode(&JWTHeader::default(), &claims, &EncodingKey::from_secret(secret.as_bytes())) {
        Ok(token) => Ok((token, claims)),
        Err(e) => {
            let error_message = format!("Error encoding JWT: {}", e);
            cata_log!(Error, &error_message);
            Err(MeltDown::new(MeltType::Unknown, "Failed to generate JWT token").with_context("error", &error_message))
        }
    }
}

pub fn generate_refresh_token(user: &Users, remember: bool, device_info: Option<String>) -> Result<(String, Claims), MeltDown> {
    let jwt_settings = get_jwt_settings();

    let expiry_duration = if remember {
        Duration::days(jwt_settings.refresh_token_expiry_days_remember as i64)
    } else {
        Duration::days(jwt_settings.refresh_token_expiry_days as i64)
    };

    let now = Utc::now();
    let expiration = now.checked_add_signed(expiry_duration).unwrap_or(now).timestamp() as usize;
    let issued_at = now.timestamp() as usize;

    let jti = Uuid::new_v4().to_string();

    let tenant_name = get_current_tenant().unwrap_or_else(|| "postgres".to_string());

    let token_version = token_registry::get_token_version(&tenant_name, user.id);

    token_registry::register_user(&tenant_name, user.id);

    let claims = Claims {
        sub: user.id.to_string(),
        username: user.username.clone(),
        role: user.role.clone(),
        exp: expiration,
        iat: issued_at,
        nbf: issued_at,
        jti,
        token_type: TokenType::Refresh,
        ver: token_version,
        remember,
        refresh_jti: None,
        device_info,
        tenant_name: Some(tenant_name),
        auth_system: AuthSystem::Tenant,
    };

    let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "your-256-bit-secret".to_string());

    match encode(&JWTHeader::default(), &claims, &EncodingKey::from_secret(secret.as_bytes())) {
        Ok(token) => Ok((token, claims)),
        Err(e) => {
            let error_message = format!("Error encoding JWT: {}", e);
            cata_log!(Error, &error_message);
            Err(MeltDown::new(MeltType::Unknown, "Failed to generate JWT token").with_context("error", &error_message))
        }
    }
}

pub fn generate_refresh_token_for_vessel(vessel: &Vessel, remember: bool, device_info: Option<String>) -> Result<(String, Claims), MeltDown> {
    let jwt_settings = get_jwt_settings();

    let expiry_duration = if remember {
        Duration::days(jwt_settings.refresh_token_expiry_days_remember as i64)
    } else {
        Duration::days(jwt_settings.refresh_token_expiry_days as i64)
    };

    let now = Utc::now();
    let expiration = now.checked_add_signed(expiry_duration).unwrap_or(now).timestamp() as usize;
    let issued_at = now.timestamp() as usize;

    let jti = Uuid::new_v4().to_string();

    let tenant_name = vessel.name.clone();
    let user_id = vessel.id;

    let token_version = token_registry::get_token_version(&tenant_name, user_id);

    token_registry::register_user(&tenant_name, user_id);

    let claims = Claims {
        sub: vessel.id.to_string(),
        username: vessel.username.clone(),
        role: "vessel".to_string(),
        exp: expiration,
        iat: issued_at,
        nbf: issued_at,
        jti,
        token_type: TokenType::Refresh,
        ver: token_version,
        remember,
        refresh_jti: None,
        device_info,
        tenant_name: Some(tenant_name),
        auth_system: AuthSystem::Vessel,
    };

    let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "your-256-bit-secret".to_string());

    match encode(&JWTHeader::default(), &claims, &EncodingKey::from_secret(secret.as_bytes())) {
        Ok(token) => Ok((token, claims)),
        Err(e) => {
            let error_message = format!("Error encoding JWT: {}", e);
            cata_log!(Error, &error_message);
            Err(MeltDown::new(MeltType::Unknown, "Failed to generate JWT token").with_context("error", &error_message))
        }
    }
}

pub fn generate_token_pair(user: &Users, remember: bool, device_info: Option<String>) -> Result<TokenPair, MeltDown> {
    let (refresh_token, refresh_claims) = generate_refresh_token(user, remember, device_info.clone())?;
    let (access_token, access_claims) = generate_access_token(user, Some(refresh_claims.jti.clone()), device_info)?;

    Ok(TokenPair {
        access_token,
        refresh_token,
        access_claims,
        refresh_claims,
    })
}

pub fn generate_token_pair_for_vessel(vessel: &Vessel, remember: bool, device_info: Option<String>) -> Result<TokenPair, MeltDown> {
    let (refresh_token, refresh_claims) = generate_refresh_token_for_vessel(vessel, remember, device_info.clone())?;
    let (access_token, access_claims) = generate_access_token_for_vessel(vessel, Some(refresh_claims.jti.clone()), device_info)?;

    Ok(TokenPair {
        access_token,
        refresh_token,
        access_claims,
        refresh_claims,
    })
}

pub fn validate_token(token: &str) -> Result<Claims, MeltDown> {
    let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "your-256-bit-secret".to_string());

    let mut validation = Validation::default();
    validation.validate_exp = true;
    validation.validate_nbf = false;
    validation.required_spec_claims = vec!["exp".to_string(), "iat".to_string(), "jti".to_string()].into_iter().collect();
    validation.leeway = 0;

    match decode::<Claims>(token, &DecodingKey::from_secret(secret.as_bytes()), &validation) {
        Ok(token_data) => {
            let claims = token_data.claims;

            let tenant_name = claims.tenant_name.clone().unwrap_or_else(|| "postgres".to_string());
            let user_id = claims.sub.parse::<i32>().map_err(|_| MeltDown::new(MeltType::ValidationFailed, "Invalid user ID in token"))?;

            if claims.token_type == TokenType::Access {
                let current_version = token_registry::get_token_version(&tenant_name, user_id);
                if claims.ver < current_version {
                    return Err(MeltDown::new(MeltType::TokenExpired, "Token version is outdated, please login again"));
                }
            }

            Ok(claims)
        }
        Err(e) => {
            let error_message = format!("Error validating JWT: {}", e);
            cata_log!(Warning, &error_message);
            Err(MeltDown::new(MeltType::Unauthorized, "Invalid token").with_context("error", &error_message))
        }
    }
}

pub fn validate_refresh_token(token: &str) -> Result<RefreshTokenInfo, MeltDown> {
    let claims = validate_token(token)?;

    if claims.token_type != TokenType::Refresh {
        return Err(MeltDown::new(MeltType::Unauthorized, "Invalid token type"));
    }

    let user_id = claims.sub.parse::<i32>().map_err(|_| MeltDown::new(MeltType::ValidationFailed, "Invalid user ID in token"))?;

    let tenant_name = claims.tenant_name.clone().unwrap_or_else(|| "postgres".to_string());

    if token_registry::is_refresh_token_used(&tenant_name, user_id, &claims.jti) {
        return Err(MeltDown::new(MeltType::Unauthorized, "Refresh token has already been used, please login again"));
    }

    let current_version = token_registry::get_token_version(&tenant_name, user_id);
    if claims.ver < current_version {
        return Err(MeltDown::new(MeltType::TokenExpired, "Token version is outdated, please login again"));
    }

    Ok(RefreshTokenInfo {
        jti: claims.jti,
        user_id,
        token_version: claims.ver,
        remember: claims.remember,
        device_info: claims.device_info,
    })
}
