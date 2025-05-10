use std::env;

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header as JWTHeader, Validation};
use uuid::Uuid;

use crate::{
    bootstrap::*,
    cata_log,
    meltdown::*,
    middleware::jwt::{Claims, TokenType},
    services::*,
    structs::*,
};

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

    let version = token_registry::get_token_version(user.id);

    let claims = Claims {
        sub: user.id.to_string(),
        exp: expiration,
        jti,
        iat: issued_at,
        nbf: issued_at,
        ver: version,
        remember: false,
        role: user.role.clone(),
        username: user.username.clone(),
        token_type: TokenType::Access,
        refresh_jti,
        device_info,
    };

    let secret = env::var("JWT_SECRET").map_err(|e| MeltDown::new(MeltType::ConfigurationError, format!("JWT_SECRET not set: {}", e)))?;

    encode(&JWTHeader::default(), &claims, &EncodingKey::from_secret(secret.as_ref())).map(|token| (token, claims)).map_err(|e| {
        let error = MeltDown::from(e);
        cata_log!(Error, error.log_message());
        error
    })
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

    let version = crate::services::default::token_registry::get_token_version(user.id);

    let claims = Claims {
        sub: user.id.to_string(),
        exp: expiration,
        jti,
        iat: issued_at,
        nbf: issued_at,
        ver: version,
        remember,
        role: user.role.clone(),
        username: user.username.clone(),
        token_type: TokenType::Refresh,
        refresh_jti: None,
        device_info,
    };

    let secret = env::var("JWT_SECRET").map_err(|e| MeltDown::new(MeltType::ConfigurationError, format!("JWT_SECRET not set: {}", e)))?;

    encode(&JWTHeader::default(), &claims, &EncodingKey::from_secret(secret.as_ref())).map(|token| (token, claims)).map_err(|e| {
        let error = MeltDown::from(e);
        cata_log!(Error, error.log_message());
        error
    })
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

pub fn generate_token(user: &Users, remember: bool) -> Result<(String, Claims), MeltDown> {
    let jwt_settings = get_jwt_settings();

    let expiry_duration = if remember {
        Duration::days(jwt_settings.token_expiry_days_remember as i64)
    } else {
        Duration::hours(jwt_settings.token_expiry_hours as i64)
    };

    let now = Utc::now();
    let expiration = now.checked_add_signed(expiry_duration).unwrap_or(now).timestamp() as usize;
    let issued_at = now.timestamp() as usize;

    let jti = Uuid::new_v4().to_string();

    let version = crate::services::default::token_registry::get_token_version(user.id);

    let claims = Claims {
        sub: user.id.to_string(),
        exp: expiration,
        jti,
        iat: issued_at,
        nbf: issued_at,
        ver: version,
        remember,
        role: user.role.clone(),
        username: user.username.clone(),
        token_type: TokenType::Access,
        refresh_jti: None,
        device_info: None,
    };

    let secret = env::var("JWT_SECRET").map_err(|e| MeltDown::new(MeltType::ConfigurationError, format!("JWT_SECRET not set: {}", e)))?;

    encode(&JWTHeader::default(), &claims, &EncodingKey::from_secret(secret.as_ref())).map(|token| (token, claims)).map_err(|e| {
        let error = MeltDown::from(e);
        cata_log!(Error, error.log_message());
        error
    })
}

pub fn validate_token(token: &str) -> Result<Claims, MeltDown> {
    let secret = env::var("JWT_SECRET").map_err(|e| MeltDown::new(MeltType::ConfigurationError, format!("JWT_SECRET not set: {}", e)))?;

    let jwt_settings = get_jwt_settings();

    let mut validation = Validation::default();
    validation.leeway = jwt_settings.token_leeway_secs as u64;

    let claims = decode::<Claims>(token, &DecodingKey::from_secret(secret.as_ref()), &validation).map(|token_data| token_data.claims).map_err(|e| {
        let error = MeltDown::from(e);
        cata_log!(Warning, error.log_message());
        error
    })?;

    if let Ok(user_id) = claims.sub.parse::<i32>() {
        if !crate::services::default::token_registry::is_token_valid(user_id, claims.ver) {
            return Err(MeltDown::new(MeltType::InvalidToken, format!("Token version {} is no longer valid for user {}", claims.ver, claims.sub)));
        }
    }

    Ok(claims)
}

pub fn validate_refresh_token(token: &str) -> Result<RefreshTokenInfo, MeltDown> {
    let claims = validate_token(token)?;

    if claims.token_type != TokenType::Refresh {
        return Err(MeltDown::new(MeltType::InvalidToken, "Token is not a refresh token"));
    }

    let user_id = claims.sub.parse::<i32>().map_err(|_| MeltDown::new(MeltType::InvalidToken, "Invalid user ID in refresh token"))?;

    Ok(RefreshTokenInfo {
        jti: claims.jti,
        user_id,
        token_version: claims.ver,
        remember: claims.remember,
        device_info: claims.device_info,
    })
}

pub fn refresh_token_if_needed(claims: &mut Claims, threshold_seconds: usize) -> Result<Option<String>, MeltDown> {
    let now_ts = Utc::now().timestamp() as usize;
    let remaining = claims.exp.saturating_sub(now_ts);

    if remaining < threshold_seconds {
        let jwt_settings = get_jwt_settings();

        let extension = if claims.remember {
            Duration::days(jwt_settings.token_expiry_days_remember as i64)
        } else {
            Duration::hours(jwt_settings.token_expiry_hours as i64)
        };

        let now = Utc::now();
        let new_exp = (now + extension).timestamp() as usize;
        claims.exp = new_exp;
        claims.iat = now.timestamp() as usize;

        let secret = env::var("JWT_SECRET").map_err(|e| MeltDown::new(MeltType::ConfigurationError, format!("JWT_SECRET not set: {}", e)))?;

        let new_token = encode(&JWTHeader::default(), &claims, &EncodingKey::from_secret(secret.as_ref())).map_err(|e| {
            cata_log!(Error, format!("Failed to regenerate JWT: {}", e));
            MeltDown::from(e)
        })?;

        cata_log!(Debug, format!("Regenerating JWT for user {}: new expiry at {} (in {}s)", claims.sub, new_exp, new_exp.saturating_sub(now_ts)));

        Ok(Some(new_token))
    } else {
        Ok(None)
    }
}
