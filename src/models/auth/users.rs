use bcrypt::{hash, verify};
use diesel::prelude::*;
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection, RunQueryDsl};

use crate::{
    database::{
        db::{establish_connection, establish_connection_with_tenant},
        schema::users::dsl::{self as user_dsl},
    },
    meltdown::*,
    structs::*,
};

impl Users {
    pub async fn count_active_users(tenant_name: &str) -> Result<i64, MeltDown> {
        let mut conn = establish_connection_with_tenant(tenant_name).await?;

        user_dsl::users
            .filter(user_dsl::active.eq(true))
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .map_err(|e| MeltDown::from(e).with_context("operation", "count_active_users"))
    }

    pub async fn verify_password(&self, password: String) -> Result<bool, MeltDown> {
        let password_hash = self.password_hash.clone();

        tokio::task::spawn_blocking(move || match verify(&password, &password_hash) {
            Ok(result) => {
                if !result {
                    return Err(MeltDown::invalid_credentials().with_context("operation", "password_verification"));
                }
                Ok(result)
            }
            Err(e) => Err(MeltDown::from(e).with_context("operation", "password_verification")),
        })
        .await
        .map_err(|e| MeltDown::new(MeltType::Unknown, format!("Task join error: {}", e)))?
    }

    pub async fn set_password(&mut self, password: String) -> Result<(), MeltDown> {
        let hashed = tokio::task::spawn_blocking(move || hash(&password, bcrypt::DEFAULT_COST).map_err(|e| MeltDown::from(e).with_context("operation", "password_hashing")))
            .await
            .map_err(|e| MeltDown::new(MeltType::Unknown, format!("Task join error: {}", e)))??;

        self.password_hash = hashed;
        Ok(())
    }

    pub async fn get_all_users(tenant_name: &str) -> Result<Vec<Users>, MeltDown> {
        let mut conn = establish_connection_with_tenant(tenant_name).await?;

        user_dsl::users
            .filter(user_dsl::role.ne("dev"))
            .load::<Users>(&mut conn)
            .await
            .map_err(|e| MeltDown::from(e).with_context("operation", "get_all_users"))
    }

    pub async fn is_admin(id: i32, tenant_name: &str) -> Result<bool, MeltDown> {
        let user = Self::get_user_by_id(id, tenant_name).await?;
        Ok(user.role == "admin")
    }

    pub async fn get_all_users_active(tenant_name: &str) -> Result<Vec<Users>, MeltDown> {
        let mut conn = establish_connection_with_tenant(tenant_name).await?;

        user_dsl::users
            .filter(user_dsl::role.ne("dev"))
            .filter(user_dsl::active.eq(true))
            .order(user_dsl::id.asc())
            .load::<Users>(&mut conn)
            .await
            .map_err(|e| MeltDown::from(e).with_context("operation", "get_all_users_active"))
    }

    pub async fn search_users(query: &str, tenant_name: &str) -> Result<Vec<Users>, MeltDown> {
        let query_string = query.to_string();
        let mut conn = establish_connection_with_tenant(tenant_name).await?;
        let query = format!("%{}%", query_string.to_lowercase());

        user_dsl::users
            .filter(
                user_dsl::active.eq(true).and(
                    user_dsl::username
                        .ilike(&query)
                        .or(user_dsl::first_name.ilike(&query))
                        .or(user_dsl::last_name.ilike(&query))
                        .or(user_dsl::email.nullable().ilike(&query)),
                ),
            )
            .order(user_dsl::username.asc())
            .load::<Users>(&mut conn)
            .await
            .map_err(|e| MeltDown::from(e).with_context("operation", "search_users").with_context("query", query))
    }

    pub async fn username_exists(username: String, tenant_name: &str) -> Result<bool, MeltDown> {
        let mut conn = establish_connection_with_tenant(tenant_name).await?;

        user_dsl::users
            .filter(user_dsl::username.eq(&username))
            .first::<Users>(&mut conn)
            .await
            .optional()
            .map_err(|e| MeltDown::from(e).with_context("operation", "username_exists").with_context("username", username.clone()))
            .map(|result| result.is_some())
    }

    pub async fn is_admin_by_id(id: i32, tenant_name: &str) -> Result<bool, MeltDown> {
        let user = Self::get_user_by_id(id, tenant_name).await?;
        Ok(user.role == "admin")
    }

    pub async fn register_user(register: RegisterForm, tenant_name: &str) -> Result<(), MeltDown> {
        let mut conn = establish_connection_with_tenant(tenant_name).await?;

        let password_hash = tokio::task::spawn_blocking(move || hash(&register.password, bcrypt::DEFAULT_COST).map_err(|e| MeltDown::from(e).with_context("operation", "password_hashing")))
            .await
            .map_err(|e| MeltDown::new(MeltType::Unknown, format!("Task join error: {}", e)))??;

        conn.transaction::<_, MeltDown, _>(|conn| {
            async move {
                let new_user = NewUser {
                    username: register.username.to_string(),
                    first_name: register.first_name.to_string(),
                    last_name: register.last_name.to_string(),
                    email: Some(register.email.to_string()),
                    password_hash,
                    role: "user".to_string(),
                };

                diesel::insert_into(user_dsl::users)
                    .values(&new_user)
                    .execute(conn)
                    .await
                    .map_err(|e| MeltDown::from(e).with_context("operation", "user_registration"))?;

                Ok(())
            }
            .scope_boxed()
        })
        .await
    }

    pub async fn get_user_by_id(id: i32, tenant_name: &str) -> Result<Users, MeltDown> {
        crate::services::default::jwt_service::set_current_tenant(tenant_name);

        let mut conn = establish_connection_with_tenant(tenant_name).await?;

        user_dsl::users.filter(user_dsl::id.eq(id)).first::<Users>(&mut conn).await.map_err(|e| {
            let mut error = MeltDown::from(e);
            error = error.with_context("operation", "get_user_by_id").with_context("user_id", id.to_string());

            error
        })
    }

    pub async fn get_user_by_username(username: String, tenant_name: &str) -> Result<Users, MeltDown> {
        crate::services::default::jwt_service::set_current_tenant(tenant_name);

        let mut conn = establish_connection_with_tenant(tenant_name).await?;

        user_dsl::users
            .filter(user_dsl::username.eq(&username))
            .filter(user_dsl::active.eq(true))
            .first::<Users>(&mut conn)
            .await
            .map_err(|e| {
                let mut error = MeltDown::from(e);
                error = error.with_context("operation", "get_user_by_username").with_context("username", username.clone());

                if matches!(error.melt_type, MeltType::RecordNotFound) {
                    error = error.with_user_message("Invalid username or password");
                }

                error
            })
    }

    pub async fn get_id_by_username(username: String, tenant_name: &str) -> Result<i32, MeltDown> {
        let mut conn = establish_connection_with_tenant(tenant_name).await?;

        user_dsl::users.filter(user_dsl::username.eq(&username)).select(user_dsl::id).first::<i32>(&mut conn).await.map_err(|e| {
            let mut error = MeltDown::from(e);
            error = error.with_context("operation", "get_id_by_username").with_context("username", username.clone());
            error
        })
    }

    pub async fn activate_user(&mut self, tenant_name: &str) -> Result<(), MeltDown> {
        let user_id = self.id;
        let mut conn = establish_connection_with_tenant(tenant_name).await?;

        let _ = user_dsl::users
            .filter(user_dsl::id.eq(user_id))
            .first::<Users>(&mut conn)
            .await
            .map_err(|e| MeltDown::from(e).with_context("operation", "activate_user_check").with_context("id", user_id.to_string()))?;

        diesel::update(user_dsl::users.filter(user_dsl::id.eq(user_id)))
            .set(user_dsl::active.eq(true))
            .execute(&mut conn)
            .await
            .map_err(|e| MeltDown::from(e).with_context("operation", "activate_user").with_context("id", user_id.to_string()))?;

        self.active = true;

        Ok(())
    }

    pub async fn deactivate_user(id: i32, tenant_name: &str) -> Result<(), MeltDown> {
        let mut conn = establish_connection_with_tenant(tenant_name).await?;

        let _ = user_dsl::users
            .filter(user_dsl::id.eq(id))
            .first::<Users>(&mut conn)
            .await
            .map_err(|e| MeltDown::from(e).with_context("operation", "deactivate_user_check").with_context("id", id.to_string()))?;

        diesel::update(user_dsl::users.filter(user_dsl::id.eq(id)))
            .set(user_dsl::active.eq(false))
            .execute(&mut conn)
            .await
            .map_err(|e| MeltDown::from(e).with_context("operation", "deactivate_user").with_context("id", id.to_string()))?;

        Ok(())
    }

    pub async fn change_password_by_id(id: i32, new_password: &str, tenant_name: &str) -> Result<(), MeltDown> {
        let password_string = new_password.to_string();
        let mut conn = establish_connection_with_tenant(tenant_name).await?;

        let password_hash = tokio::task::spawn_blocking(move || hash(&password_string, bcrypt::DEFAULT_COST).map_err(|e| MeltDown::from(e).with_context("operation", "password_hashing")))
            .await
            .map_err(|e| MeltDown::new(MeltType::Unknown, format!("Task join error: {}", e)))??;

        let mut user = user_dsl::users
            .filter(user_dsl::id.eq(id))
            .first::<Users>(&mut conn)
            .await
            .map_err(|e| MeltDown::from(e).with_context("operation", "change_password_check").with_context("id", id.to_string()))?;

        user.password_hash = password_hash.clone();
        user.should_change_password = false;

        diesel::update(user_dsl::users.filter(user_dsl::id.eq(id)))
            .set((user_dsl::password_hash.eq(&user.password_hash), user_dsl::should_change_password.eq(user.should_change_password)))
            .execute(&mut conn)
            .await
            .map_err(|e| MeltDown::from(e).with_context("operation", "change_password").with_context("id", id.to_string()))?;

        Ok(())
    }

    pub async fn reset_password_by_id(id: i32, new_password: &str, tenant_name: &str) -> Result<(), MeltDown> {
        let password_string = new_password.to_string();
        let mut conn = establish_connection_with_tenant(tenant_name).await?;

        let password_hash = tokio::task::spawn_blocking(move || hash(&password_string, bcrypt::DEFAULT_COST).map_err(|e| MeltDown::from(e).with_context("operation", "password_hashing")))
            .await
            .map_err(|e| MeltDown::new(MeltType::Unknown, format!("Task join error: {}", e)))??;

        let mut user = user_dsl::users
            .filter(user_dsl::id.eq(id))
            .first::<Users>(&mut conn)
            .await
            .map_err(|e| MeltDown::from(e).with_context("operation", "reset_password_check").with_context("id", id.to_string()))?;

        user.password_hash = password_hash.clone();
        user.should_change_password = true;

        diesel::update(user_dsl::users.filter(user_dsl::id.eq(id)))
            .set((user_dsl::password_hash.eq(&user.password_hash), user_dsl::should_change_password.eq(user.should_change_password)))
            .execute(&mut conn)
            .await
            .map_err(|e| MeltDown::from(e).with_context("operation", "reset_password").with_context("id", id.to_string()))?;

        Ok(())
    }

    pub async fn update_profile(&mut self, first_name: &str, last_name: &str, email: Option<&str>, tenant_name: &str) -> Result<(), MeltDown> {
        let user_id = self.id;
        let first_name_string = first_name.to_string();
        let last_name_string = last_name.to_string();
        let email_option = email.map(|e| e.to_string());

        let updated_at = chrono::Utc::now().timestamp();
        let mut conn = establish_connection_with_tenant(tenant_name).await?;

        let user = user_dsl::users
            .filter(user_dsl::id.eq(user_id))
            .first::<Users>(&mut conn)
            .await
            .map_err(|e| MeltDown::from(e).with_context("operation", "update_profile_check").with_context("id", user_id.to_string()))?;

        if !user.active {
            return Err(MeltDown::validation_failed("User account is not active"));
        }

        let result = diesel::update(user_dsl::users.filter(user_dsl::id.eq(user_id)))
            .set((
                user_dsl::first_name.eq(&first_name_string),
                user_dsl::last_name.eq(&last_name_string),
                user_dsl::email.eq(&email_option),
                user_dsl::updated_at.eq(updated_at),
            ))
            .execute(&mut conn)
            .await
            .map_err(|e| MeltDown::from(e).with_context("operation", "update_profile").with_context("id", user_id.to_string()));

        if result.is_ok() {
            self.first_name = first_name_string;
            self.last_name = last_name_string;
            self.email = email_option;
            self.updated_at = updated_at;
        }

        result.map(|_| ())
    }
}
