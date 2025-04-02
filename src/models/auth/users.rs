use crate::database::db::establish_connection;
use crate::database::schema::users::dsl::{self as user_dsl};
use crate::structs::*;
use bcrypt::{hash, verify, DEFAULT_COST};
use diesel::prelude::*;
use diesel::result::Error;
use diesel::Connection;

impl Users {
    pub fn verify_password(&self, password: &str) -> bool {
        verify(password, &self.password_hash).unwrap_or(false)
    }

    pub fn set_password(&mut self, password: &str) {
        self.password_hash = hash(password, DEFAULT_COST).unwrap();
    }

    pub fn get_all_users() -> Result<Vec<Users>, &'static str> {
        let mut conn = establish_connection();

        user_dsl::users.filter(user_dsl::role.ne("dev")).load::<Users>(&mut conn).map_err(|_| "Error retrieving users")
    }

    pub fn is_admin(id: i32) -> bool {
        if let Ok(user) = Self::get_user_by_id(id) {
            user.role == "admin"
        } else {
            false
        }
    }

    pub fn get_all_users_active() -> Result<Vec<Users>, &'static str> {
        let mut conn = establish_connection();

        user_dsl::users
            .filter(user_dsl::role.ne("dev"))
            .filter(user_dsl::active.eq(true))
            .order(user_dsl::id.asc())
            .load::<Users>(&mut conn)
            .map_err(|_| "Error retrieving users")
    }

    pub fn search_users(query: &str) -> Result<Vec<Users>, &'static str> {
        let mut conn = establish_connection();
        let query = format!("%{}%", query.to_lowercase());

        user_dsl::users
            .filter(
                user_dsl::active.eq(true).and(
                    user_dsl::username.ilike(&query)
                        .or(user_dsl::first_name.ilike(&query))
                        .or(user_dsl::last_name.ilike(&query))
                        .or(user_dsl::email.nullable().ilike(&query))
                ),
            )
            .order(user_dsl::username.asc())
            .load::<Users>(&mut conn)
            .map_err(|_| "Error searching users")
    }

    pub fn username_exists(username: &str) -> Result<bool, &'static str> {
        let mut conn = establish_connection();

        user_dsl::users
            .filter(user_dsl::username.eq(username))
            .first::<Users>(&mut conn)
            .optional()
            .map(|opt| opt.is_some())
            .map_err(|_| "Database error.")
    }

    pub fn is_admin_by_id(id: i32) -> bool {
        if let Ok(user) = Self::get_user_by_id(id) {
            user.role == "admin"
        } else {
            false
        }
    }

    pub fn register_user(register: RegisterForm) -> Result<(), &'static str> {
        // Validate passwords outside the transaction
        if register.password != register.confirm_password {
            return Err("Passwords don't match.");
        }

        let mut conn = establish_connection();
        
        // Start a transaction
        conn.transaction(|conn| {
            // Check username existence within the transaction
            let username_exists = user_dsl::users
                .filter(user_dsl::username.eq(&register.username))
                .first::<Users>(conn)
                .optional()
                .map(|opt| opt.is_some())
                .map_err(|_| Error::RollbackTransaction)?;
                
            if username_exists {
                return Err(Error::RollbackTransaction);
            }
            
            let password_hash = match hash(&register.password, DEFAULT_COST) {
                Ok(hash) => hash,
                Err(_) => return Err(Error::RollbackTransaction),
            };
            
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
                .map_err(|_| Error::RollbackTransaction)?;
                
            Ok(())
        })
        .map_err(|e: diesel::result::Error| {
            // Handle error types
            match e {
                diesel::result::Error::DatabaseError(diesel::result::DatabaseErrorKind::UniqueViolation, _) => {
                    "Username already taken."
                }
                _ => "Error saving new user"
            }
        })
    }

    pub fn get_user_by_id(id: i32) -> Result<Users, &'static str> {
        let mut conn = establish_connection();

        match user_dsl::users.filter(user_dsl::id.eq(id)).first::<Users>(&mut conn) {
            Ok(user) => Ok(user),
            Err(_) => Err("Users not found"),
        }
    }

    pub fn get_user_by_username(username: &str) -> Result<Users, &'static str> {
        let mut conn = establish_connection();
        user_dsl::users
            .filter(user_dsl::username.eq(username))
            .filter(user_dsl::active.eq(true))
            .first::<Users>(&mut conn)
            .map_err(|_| "Users not found")
    }

    pub fn get_id_by_username(username: &str) -> Result<i32, &'static str> {
        let mut conn = establish_connection();

        user_dsl::users.filter(user_dsl::username.eq(username)).select(user_dsl::id).first::<i32>(&mut conn).map_err(|_| "Users not found")
    }

    pub fn activate_user(&mut self) -> Result<(), &'static str> {
        let mut conn = establish_connection();
        let user_id = self.id; // Store ID for use in closure
        
        conn.transaction(|conn| {
            // Get the latest user data inside the transaction
            let _ = user_dsl::users
                .filter(user_dsl::id.eq(user_id))
                .first::<Users>(conn)
                .map_err(|_| Error::RollbackTransaction)?;
            
            // Update active status
            diesel::update(user_dsl::users.filter(user_dsl::id.eq(user_id)))
                .set(user_dsl::active.eq(true))
                .execute(conn)
                .map_err(|_| Error::RollbackTransaction)?;
                
            Ok(())
        })
        .map_err(|_: diesel::result::Error| "Error activating user")?;
        
        // Update local state
        self.active = true;
        
        Ok(())
    }

    pub fn deactivate_user(id: i32) -> Result<(), &'static str> {
        let mut conn = establish_connection();

        conn.transaction(|conn| {
            // Get the latest user data inside the transaction
            let _ = user_dsl::users
                .filter(user_dsl::id.eq(id))
                .first::<Users>(conn)
                .map_err(|_| Error::RollbackTransaction)?;
                
            // Update active status
            diesel::update(user_dsl::users.filter(user_dsl::id.eq(id)))
                .set(user_dsl::active.eq(false))
                .execute(conn)
                .map_err(|_| Error::RollbackTransaction)?;
                
            Ok(())
        })
        .map_err(|_: diesel::result::Error| "Error deactivating user")
    }

    pub fn change_password_by_id(id: i32, new_password: &str) -> Result<(), &'static str> {
        let mut conn = establish_connection();
        
        conn.transaction(|conn| {
            // Get user inside transaction to prevent race conditions
            let mut user = user_dsl::users
                .filter(user_dsl::id.eq(id))
                .first::<Users>(conn)
                .map_err(|_| Error::RollbackTransaction)?;
                
            // Generate new password hash
            let password_hash = hash(new_password, DEFAULT_COST)
                .map_err(|_| Error::RollbackTransaction)?;
                
            user.password_hash = password_hash;
            user.should_change_password = false;

            // Update user in database within the same transaction
            diesel::update(user_dsl::users.filter(user_dsl::id.eq(id)))
                .set((
                    user_dsl::password_hash.eq(&user.password_hash), 
                    user_dsl::should_change_password.eq(user.should_change_password)
                ))
                .execute(conn)
                .map_err(|_| Error::RollbackTransaction)?;

            Ok(())
        })
        .map_err(|_: diesel::result::Error| "Error changing password")
    }

    pub fn reset_password_by_id(id: i32, new_password: &str) -> Result<(), &'static str> {
        let mut conn = establish_connection();
        
        conn.transaction(|conn| {
            // Get user inside transaction to prevent race conditions
            let mut user = user_dsl::users
                .filter(user_dsl::id.eq(id))
                .first::<Users>(conn)
                .map_err(|_| Error::RollbackTransaction)?;
                
            // Generate new password hash
            let password_hash = hash(new_password, DEFAULT_COST)
                .map_err(|_| Error::RollbackTransaction)?;
                
            user.password_hash = password_hash;
            user.should_change_password = true;

            // Update user in database within the same transaction
            diesel::update(user_dsl::users.filter(user_dsl::id.eq(id)))
                .set((
                    user_dsl::password_hash.eq(&user.password_hash), 
                    user_dsl::should_change_password.eq(user.should_change_password)
                ))
                .execute(conn)
                .map_err(|_| Error::RollbackTransaction)?;

            Ok(())
        })
        .map_err(|_: diesel::result::Error| "Error resetting password")
    }

    pub fn update_profile(&mut self, first_name: &str, last_name: &str, email: Option<&str>) -> Result<(), &'static str> {
        let mut conn = establish_connection();
        let user_id = self.id; // Store ID for use in closure
        
        // Local updates to self
        self.first_name = first_name.to_string();
        self.last_name = last_name.to_string();
        self.email = email.map(|e| e.to_string());
        self.updated_at = chrono::Utc::now().timestamp();
        
        // Clone values for use in transaction
        let first_name = self.first_name.clone();
        let last_name = self.last_name.clone();
        let email = self.email.clone();
        let updated_at = self.updated_at;
        
        conn.transaction(|conn| {
            // Get the latest user data inside the transaction
            let user = user_dsl::users
                .filter(user_dsl::id.eq(user_id))
                .first::<Users>(conn)
                .map_err(|_| Error::RollbackTransaction)?;
                
            // Check if user is still active before updating
            if !user.active {
                return Err(Error::RollbackTransaction);
            }
            
            // Update the user profile
            diesel::update(user_dsl::users.filter(user_dsl::id.eq(user_id)))
                .set((
                    user_dsl::first_name.eq(&first_name),
                    user_dsl::last_name.eq(&last_name),
                    user_dsl::email.eq(&email),
                    user_dsl::updated_at.eq(updated_at),
                ))
                .execute(conn)
                .map_err(|_| Error::RollbackTransaction)?;
                
            Ok(())
        })
        .map_err(|e: diesel::result::Error| {
            match e {
                diesel::result::Error::DatabaseError(_, _) => "Database error updating profile",
                _ => "Error updating profile"
            }
        })
    }
}
