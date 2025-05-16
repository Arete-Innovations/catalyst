use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::Utc;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};

use crate::{
    cata_log,
    meltdown::*,
    services::default::jwt_service,
    vessel::{
        database::{db::establish_connection, schema::vessels},
        structs::{NewVessel, Vessel, VesselLoginForm, VesselRegisterForm, VesselResponse},
    },
};

impl Vessel {
    pub async fn create(vessel: NewVessel) -> Result<Vessel, MeltDown> {
        let mut conn = establish_connection().await?;

        match diesel::insert_into(vessels::table).values(vessel).returning(Vessel::as_returning()).get_result(&mut conn).await {
            Ok(vessel) => Ok(vessel),
            Err(e) => {
                let error_message = format!("Error creating vessel: {}", e);
                cata_log!(Error, &error_message);
                Err(MeltDown::new(MeltType::DatabaseError, "Failed to create vessel").with_context("error", &error_message))
            }
        }
    }

    pub async fn find_by_username(username: &str) -> Result<Option<Vessel>, MeltDown> {
        let mut conn = establish_connection().await?;

        match vessels::table.filter(vessels::username.eq(username)).select(Vessel::as_select()).first(&mut conn).await.optional() {
            Ok(vessel) => Ok(vessel),
            Err(e) => {
                let error_message = format!("Error finding vessel by username: {}", e);
                cata_log!(Error, &error_message);
                Err(MeltDown::new(MeltType::DatabaseError, "Failed to find vessel by username").with_context("error", &error_message))
            }
        }
    }

    pub async fn find_by_id(id: i32) -> Result<Option<Vessel>, MeltDown> {
        let mut conn = establish_connection().await?;

        match vessels::table.find(id).select(Vessel::as_select()).first(&mut conn).await.optional() {
            Ok(vessel) => Ok(vessel),
            Err(e) => {
                let error_message = format!("Error finding vessel by id: {}", e);
                cata_log!(Error, &error_message);
                Err(MeltDown::new(MeltType::DatabaseError, "Failed to find vessel by id").with_context("error", &error_message))
            }
        }
    }

    pub async fn find_by_name(name: &str) -> Result<Option<Vessel>, MeltDown> {
        let mut conn = establish_connection().await?;

        match vessels::table.filter(vessels::name.eq(name)).select(Vessel::as_select()).first(&mut conn).await.optional() {
            Ok(vessel) => Ok(vessel),
            Err(e) => {
                let error_message = format!("Error finding vessel by name: {}", e);
                cata_log!(Error, &error_message);
                Err(MeltDown::new(MeltType::DatabaseError, "Failed to find vessel by name").with_context("error", &error_message))
            }
        }
    }

    pub async fn tenant_exists(tenant_name: &str) -> Result<bool, MeltDown> {
        let result = Self::find_by_name(tenant_name).await?;
        Ok(result.is_some())
    }

    pub async fn verify_password(&self, password: &str) -> Result<bool, MeltDown> {
        cata_log!(Info, format!("Verifying password for vessel: {}", self.username));

        let hash_prefix = self.password_hash.chars().take(10).collect::<String>();
        cata_log!(Info, format!("Stored password hash prefix: {}...", hash_prefix));

        match verify(password, &self.password_hash) {
            Ok(valid) => {
                if valid {
                    cata_log!(Info, "Password hash verification successful");
                } else {
                    cata_log!(Warning, "Password hash verification failed - hash mismatch");
                }
                Ok(valid)
            }
            Err(e) => {
                let error_message = format!("Error verifying password: {}", e);
                cata_log!(Error, &error_message);
                Err(MeltDown::new(MeltType::ValidationFailed, "Failed to verify password").with_context("error", &error_message))
            }
        }
    }

    pub async fn login_user(login_form: VesselLoginForm) -> Result<(Vessel, jwt_service::TokenPair), MeltDown> {
        let vessel = match Self::find_by_username(&login_form.username).await {
            Ok(Some(vessel)) => vessel,
            Ok(None) => {
                cata_log!(Warning, format!("Login attempt with invalid username: {}", login_form.username));
                return Err(MeltDown::invalid_credentials());
            }
            Err(e) => {
                cata_log!(Error, format!("Database error during login: {}", e.log_message()));
                return Err(MeltDown::new(MeltType::DatabaseError, "Database error. Please try again later."));
            }
        };

        let pw_len = login_form.password.len();
        let pw_first = if pw_len > 0 { login_form.password.chars().next().unwrap() } else { '?' };
        let pw_last = if pw_len > 1 { login_form.password.chars().last().unwrap() } else { '?' };
        cata_log!(Info, format!("Password input info: length={}, first={}, last={}", pw_len, pw_first, pw_last));

        let password_match = match vessel.verify_password(&login_form.password).await {
            Ok(is_match) => {
                if is_match {
                    cata_log!(Info, format!("Password verification successful for vessel: {}", login_form.username));
                    true
                } else {
                    cata_log!(Warning, format!("Password verification failed for vessel: {} - passwords don't match", login_form.username));
                    false
                }
            }
            Err(e) => {
                cata_log!(Error, format!("Password verification error for vessel: {} - {}", login_form.username, e.log_message()));
                return Err(e);
            }
        };

        if !password_match {
            cata_log!(Warning, format!("Failed login attempt for vessel: {}", login_form.username));
            return Err(MeltDown::invalid_credentials());
        }

        let remember = login_form.remember_me.unwrap_or(false);
        let device_info = Some(format!("Vessel login at {}", Utc::now().to_rfc3339()));

        let token_pair = jwt_service::generate_token_pair_for_vessel(&vessel, remember, device_info)?;

        cata_log!(Info, format!("Vessel {} logged in successfully", vessel.username));

        Ok((vessel, token_pair))
    }

    pub async fn register_user(register_form: VesselRegisterForm) -> Result<Vessel, MeltDown> {
        if register_form.password != register_form.confirm_password {
            return Err(MeltDown::new(MeltType::ValidationFailed, "Passwords do not match."));
        }

        if register_form.name.is_empty() {
            return Err(MeltDown::new(MeltType::ValidationFailed, "Vessel name cannot be empty."));
        }

        if !register_form.name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err(MeltDown::new(MeltType::ValidationFailed, "Vessel name can only contain letters, numbers, and underscores."));
        }

        if register_form.name.chars().next().map_or(false, |c| c.is_numeric()) {
            return Err(MeltDown::new(MeltType::ValidationFailed, "Vessel name must start with a letter or underscore."));
        }

        match Self::find_by_username(&register_form.username).await {
            Ok(Some(_)) => {
                return Err(MeltDown::new(MeltType::ValidationFailed, "A vessel with this username already exists."));
            }
            Ok(None) => {}
            Err(e) => {
                cata_log!(Error, format!("Database error checking username: {}", e.log_message()));
                return Err(MeltDown::new(MeltType::DatabaseError, "Database error. Please try again later."));
            }
        }

        let pw_len = register_form.password.len();
        let pw_first = if pw_len > 0 { register_form.password.chars().next().unwrap() } else { '?' };
        let pw_last = if pw_len > 1 { register_form.password.chars().last().unwrap() } else { '?' };
        cata_log!(Info, format!("Registration password info: length={}, first={}, last={}", pw_len, pw_first, pw_last));

        let password_hash = match hash(&register_form.password, DEFAULT_COST) {
            Ok(hashed) => {
                let hash_prefix = hashed.chars().take(10).collect::<String>();
                cata_log!(Info, format!("Generated password hash prefix: {}...", hash_prefix));
                hashed
            }
            Err(e) => {
                let error_message = format!("Error hashing password: {}", e);
                cata_log!(Error, &error_message);
                return Err(MeltDown::new(MeltType::ValidationFailed, "Failed to hash password").with_context("error", &error_message));
            }
        };

        let new_vessel = NewVessel {
            name: register_form.name.clone(),
            display_name: register_form.display_name,
            username: register_form.username,
            email: register_form.email,
            password_hash,
            first_name: register_form.first_name,
            last_name: register_form.last_name,
        };

        match Self::create(new_vessel).await {
            Ok(vessel) => {
                cata_log!(Info, "Vessel registered successfully");

                let hash_prefix = vessel.password_hash.chars().take(10).collect::<String>();
                cata_log!(Info, format!("Stored password hash prefix: {}...", hash_prefix));

                cata_log!(Info, format!("Provisioning database for tenant: {}", vessel.name));

                match crate::vessel::database::provision_vessel_database(&vessel.name, &vessel.username, &vessel.email, &vessel.password_hash, &vessel.display_name).await {
                    Ok(_) => {
                        cata_log!(Info, format!("Successfully provisioned database for tenant: {}", vessel.name));
                    }
                    Err(e) => {
                        cata_log!(Warning, format!("Failed to provision database for tenant: {} - {}", vessel.name, e.log_message()));
                    }
                }

                Ok(vessel)
            }
            Err(err) => {
                cata_log!(Error, format!("Registration error: {}", err.log_message()));
                Err(err)
            }
        }
    }

    pub async fn refresh_user_token(refresh_token: &str) -> Result<(Vessel, jwt_service::TokenPair), MeltDown> {
        let token_info = jwt_service::validate_refresh_token(refresh_token)?;
        let user_id = token_info.user_id;

        let vessel = match Self::find_by_id(user_id as i32).await {
            Ok(Some(vessel)) => vessel,
            Ok(None) => {
                cata_log!(Error, format!("Failed to get vessel {}: Vessel not found", user_id));
                return Err(MeltDown::new(MeltType::NotFound, "Vessel account issue. Please log in again."));
            }
            Err(error) => {
                cata_log!(Error, format!("Failed to get vessel {}: {}", user_id, error.log_message()));
                return Err(MeltDown::new(MeltType::DatabaseError, "Database error. Please log in again."));
            }
        };

        let token_pair = jwt_service::generate_token_pair_for_vessel(&vessel, token_info.remember, token_info.device_info)?;

        cata_log!(Info, format!("Refreshed tokens for vessel {}", user_id));

        Ok((vessel, token_pair))
    }
}
