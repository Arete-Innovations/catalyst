use crate::database::db::establish_connection;
use crate::database::schema::users::dsl::{self as user_dsl};
use crate::meltdown::*;
use crate::structs::*;
use bcrypt::{hash, verify};
use diesel::prelude::*;
use diesel::Connection;
use tokio::task;

impl Users {
    pub async fn count_active_users() -> Result<i64, MeltDown> {
        task::spawn_blocking(move || {
            let mut conn = establish_connection();

            user_dsl::users
                .filter(user_dsl::active.eq(true))
                .count()
                .get_result::<i64>(&mut conn)
                .map_err(|e| MeltDown::from(e).with_context("operation", "count_active_users"))
        })
        .await
        .map_err(|e| MeltDown::new(MeltType::Unknown, format!("Task join error: {}", e)))?
    }

    pub async fn verify_password(&self, password: String) -> Result<bool, MeltDown> {
        let password_hash = self.password_hash.clone();

        task::spawn_blocking(move || {
            let result = verify(&password, &password_hash)?;

            if !result {
                return Err(MeltDown::invalid_credentials().with_context("operation", "password_verification"));
            }

            Ok(result)
        })
        .await
        .map_err(|e| MeltDown::new(MeltType::Unknown, format!("Task join error: {}", e)))?
    }

    pub async fn set_password(&mut self, password: String) -> Result<(), MeltDown> {
        let hashed = task::spawn_blocking(move || hash(&password, bcrypt::DEFAULT_COST).map_err(|e| MeltDown::from(e).with_context("operation", "password_hashing")))
            .await
            .map_err(|e| MeltDown::new(MeltType::Unknown, format!("Task join error: {}", e)))??;

        self.password_hash = hashed;
        Ok(())
    }

    pub async fn get_all_users() -> Result<Vec<Users>, MeltDown> {
        task::spawn_blocking(move || {
            let mut conn = establish_connection();

            user_dsl::users
                .filter(user_dsl::role.ne("dev"))
                .load::<Users>(&mut conn)
                .map_err(|e| MeltDown::from(e).with_context("operation", "get_all_users"))
        })
        .await
        .map_err(|e| MeltDown::new(MeltType::Unknown, format!("Task join error: {}", e)))?
    }

    pub async fn is_admin(id: i32) -> Result<bool, MeltDown> {
        let user = Self::get_user_by_id(id).await?;
        Ok(user.role == "admin")
    }

    pub async fn get_all_users_active() -> Result<Vec<Users>, MeltDown> {
        task::spawn_blocking(move || {
            let mut conn = establish_connection();

            user_dsl::users
                .filter(user_dsl::role.ne("dev"))
                .filter(user_dsl::active.eq(true))
                .order(user_dsl::id.asc())
                .load::<Users>(&mut conn)
                .map_err(|e| MeltDown::from(e).with_context("operation", "get_all_users_active"))
        })
        .await
        .map_err(|e| MeltDown::new(MeltType::Unknown, format!("Task join error: {}", e)))?
    }

    pub async fn search_users(query: &str) -> Result<Vec<Users>, MeltDown> {
        let query_string = query.to_string();

        task::spawn_blocking(move || {
            let mut conn = establish_connection();
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
                .map_err(|e| MeltDown::from(e).with_context("operation", "search_users").with_context("query", query))
        })
        .await
        .map_err(|e| MeltDown::new(MeltType::Unknown, format!("Task join error: {}", e)))?
    }

    pub async fn username_exists(username: String) -> Result<bool, MeltDown> {
        task::spawn_blocking(move || {
            let mut conn = establish_connection();

            user_dsl::users
                .filter(user_dsl::username.eq(&username))
                .first::<Users>(&mut conn)
                .optional()
                .map_err(|e| MeltDown::from(e).with_context("operation", "username_exists").with_context("username", username.clone()))
                .map(|result| result.is_some())
        })
        .await
        .map_err(|e| MeltDown::new(MeltType::Unknown, format!("Task join error: {}", e)))?
    }

    pub async fn is_admin_by_id(id: i32) -> Result<bool, MeltDown> {
        let user = Self::get_user_by_id(id).await?;
        Ok(user.role == "admin")
    }

    pub async fn register_user(register: RegisterForm) -> Result<(), MeltDown> {
        task::spawn_blocking(move || {
            let mut conn = establish_connection();

            conn.transaction::<_, MeltDown, _>(|conn| {
                let password_hash = hash(&register.password, bcrypt::DEFAULT_COST)?;

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
                    .map_err(|e| MeltDown::from(e).with_context("operation", "user_registration"))?;

                Ok(())
            })
        })
        .await
        .map_err(|e| MeltDown::new(MeltType::Unknown, format!("Task join error: {}", e)))?
    }

    pub async fn get_user_by_id(id: i32) -> Result<Users, MeltDown> {
        task::spawn_blocking(move || {
            let mut conn = establish_connection();

            user_dsl::users.filter(user_dsl::id.eq(id)).first::<Users>(&mut conn).map_err(|e| {
                let mut error = MeltDown::from(e);
                error = error.with_context("operation", "get_user_by_id").with_context("user_id", id.to_string());

                error
            })
        })
        .await
        .map_err(|e| MeltDown::new(MeltType::Unknown, format!("Task join error: {}", e)))?
    }

    pub async fn get_user_by_username(username: String) -> Result<Users, MeltDown> {
        task::spawn_blocking(move || {
            let mut conn = establish_connection();

            user_dsl::users.filter(user_dsl::username.eq(&username)).filter(user_dsl::active.eq(true)).first::<Users>(&mut conn).map_err(|e| {
                let mut error = MeltDown::from(e);
                error = error.with_context("operation", "get_user_by_username").with_context("username", username.clone());

                if matches!(error.melt_type, MeltType::RecordNotFound) {
                    error = error.with_user_message("Invalid username or password");
                }

                error
            })
        })
        .await
        .map_err(|e| MeltDown::new(MeltType::Unknown, format!("Task join error: {}", e)))?
    }

    pub async fn get_id_by_username(username: String) -> Result<i32, MeltDown> {
        task::spawn_blocking(move || {
            let mut conn = establish_connection();

            user_dsl::users.filter(user_dsl::username.eq(&username)).select(user_dsl::id).first::<i32>(&mut conn).map_err(|e| {
                let mut error = MeltDown::from(e);
                error = error.with_context("operation", "get_id_by_username").with_context("username", username.clone());
                error
            })
        })
        .await
        .map_err(|e| MeltDown::new(MeltType::Unknown, format!("Task join error: {}", e)))?
    }

    pub async fn activate_user(&mut self) -> Result<(), MeltDown> {
        let user_id = self.id;

        task::spawn_blocking(move || {
            let mut conn = establish_connection();

            conn.transaction::<_, MeltDown, _>(|conn| {
                let _ = user_dsl::users.filter(user_dsl::id.eq(user_id)).first::<Users>(conn)?;

                diesel::update(user_dsl::users.filter(user_dsl::id.eq(user_id))).set(user_dsl::active.eq(true)).execute(conn)?;

                Ok(())
            })
        })
        .await
        .map_err(|e| MeltDown::new(MeltType::Unknown, format!("Task join error: {}", e)))?;

        self.active = true;

        Ok(())
    }

    pub async fn deactivate_user(id: i32) -> Result<(), MeltDown> {
        task::spawn_blocking(move || {
            let mut conn = establish_connection();

            conn.transaction::<_, MeltDown, _>(|conn| {
                let _ = user_dsl::users.filter(user_dsl::id.eq(id)).first::<Users>(conn)?;

                diesel::update(user_dsl::users.filter(user_dsl::id.eq(id))).set(user_dsl::active.eq(false)).execute(conn)?;

                Ok(())
            })
        })
        .await
        .map_err(|e| MeltDown::new(MeltType::Unknown, format!("Task join error: {}", e)))?;

        Ok(())
    }

    pub async fn change_password_by_id(id: i32, new_password: &str) -> Result<(), MeltDown> {
        let password_string = new_password.to_string();

        task::spawn_blocking(move || {
            let mut conn = establish_connection();

            conn.transaction::<_, MeltDown, _>(|conn| {
                let mut user = user_dsl::users.filter(user_dsl::id.eq(id)).first::<Users>(conn)?;

                let password_hash = hash(&password_string, bcrypt::DEFAULT_COST)?;

                user.password_hash = password_hash;
                user.should_change_password = false;

                diesel::update(user_dsl::users.filter(user_dsl::id.eq(id)))
                    .set((user_dsl::password_hash.eq(&user.password_hash), user_dsl::should_change_password.eq(user.should_change_password)))
                    .execute(conn)?;

                Ok(())
            })
        })
        .await
        .map_err(|e| MeltDown::new(MeltType::Unknown, format!("Task join error: {}", e)))?;

        Ok(())
    }

    pub async fn reset_password_by_id(id: i32, new_password: &str) -> Result<(), MeltDown> {
        let password_string = new_password.to_string();

        task::spawn_blocking(move || {
            let mut conn = establish_connection();

            conn.transaction::<_, MeltDown, _>(|conn| {
                let mut user = user_dsl::users.filter(user_dsl::id.eq(id)).first::<Users>(conn)?;

                let password_hash = hash(&password_string, bcrypt::DEFAULT_COST)?;

                user.password_hash = password_hash;
                user.should_change_password = true;

                diesel::update(user_dsl::users.filter(user_dsl::id.eq(id)))
                    .set((user_dsl::password_hash.eq(&user.password_hash), user_dsl::should_change_password.eq(user.should_change_password)))
                    .execute(conn)?;

                Ok(())
            })
        })
        .await
        .map_err(|e| MeltDown::new(MeltType::Unknown, format!("Task join error: {}", e)))?;

        Ok(())
    }

    pub async fn update_profile(&mut self, first_name: &str, last_name: &str, email: Option<&str>) -> Result<(), MeltDown> {
        let user_id = self.id;
        let first_name_string = first_name.to_string();
        let last_name_string = last_name.to_string();
        let email_option = email.map(|e| e.to_string());

        self.first_name = first_name_string.clone();
        self.last_name = last_name_string.clone();
        self.email = email_option.clone();
        self.updated_at = chrono::Utc::now().timestamp();

        let updated_at = self.updated_at;

        task::spawn_blocking(move || {
            let mut conn = establish_connection();

            conn.transaction::<_, MeltDown, _>(|conn| {
                let user = user_dsl::users.filter(user_dsl::id.eq(user_id)).first::<Users>(conn)?;

                if !user.active {
                    return Err(MeltDown::validation_failed("User account is not active"));
                }

                diesel::update(user_dsl::users.filter(user_dsl::id.eq(user_id)))
                    .set((
                        user_dsl::first_name.eq(&first_name_string),
                        user_dsl::last_name.eq(&last_name_string),
                        user_dsl::email.eq(&email_option),
                        user_dsl::updated_at.eq(updated_at),
                    ))
                    .execute(conn)?;

                Ok(())
            })
        })
        .await
        .map_err(|e| MeltDown::new(MeltType::Unknown, format!("Task join error: {}", e)))?;

        Ok(())
    }
}

