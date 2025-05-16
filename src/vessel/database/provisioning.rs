use std::{fs, io::Read, path::Path, process::Command};

use diesel_async::AsyncPgConnection;
use tokio::task;

use crate::{cata_log, meltdown::*, vessel::structs::Vessel};

pub async fn provision_vessel_database(name: &str, username: &str, email: &str, password_hash: &str, display_name: &str) -> Result<(), MeltDown> {
    let tenant_name = name.to_string();

    create_database(&tenant_name).await?;

    run_migrations(&tenant_name).await?;

    seed_database(&tenant_name).await?;

    // Get the vessel to access first_name and last_name
    let vessel = match Vessel::find_by_name(name).await {
        Ok(Some(vessel)) => vessel,
        Ok(None) => {
            cata_log!(Warning, format!("Vessel '{}' not found for admin user creation", name));
            // Fall back to display_name
            create_admin_user(&tenant_name, username, email, password_hash, display_name).await?;
            return Ok(());
        },
        Err(e) => {
            cata_log!(Error, format!("Error finding vessel: {}", e.log_message()));
            create_admin_user(&tenant_name, username, email, password_hash, display_name).await?;
            return Ok(());
        }
    };

    // Use vessel data for admin user creation
    create_admin_user_from_vessel(&tenant_name, &vessel).await?;

    cata_log!(Info, format!("Successfully provisioned vessel database '{}'", tenant_name));
    Ok(())
}

async fn create_database(name: &String) -> Result<(), MeltDown> {
    let db_name = name.clone();
    cata_log!(Info, format!("Creating database '{}'", db_name));

    task::spawn_blocking(move || {
        let output = Command::new("createdb").arg("-U").arg("postgres").arg(&db_name).output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    cata_log!(Info, format!("Database '{}' created successfully", db_name));
                    Ok(())
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);

                    if error.contains("already exists") {
                        cata_log!(Warning, format!("Database '{}' already exists", db_name));
                        Ok(())
                    } else {
                        cata_log!(Error, format!("Failed to create database '{}': {}", db_name, error));
                        Err(MeltDown::new(MeltType::DatabaseError, "Failed to create database").with_context("error", error.to_string()))
                    }
                }
            }
            Err(e) => {
                cata_log!(Error, format!("Failed to execute createdb command: {}", e));
                Err(MeltDown::new(MeltType::DatabaseError, "Failed to create database").with_context("error", e.to_string()))
            }
        }
    })
    .await
    .unwrap_or_else(|e| {
        cata_log!(Error, format!("Task to create database failed: {}", e));
        Err(MeltDown::new(MeltType::DatabaseError, "Task to create database failed").with_context("error", e.to_string()))
    })
}

async fn run_migrations(name: &String) -> Result<(), MeltDown> {
    let db_name = name.clone();
    cata_log!(Info, format!("Running migrations for database '{}'", db_name));

    let migrations_dir = Path::new("src/database/migrations");

    if !migrations_dir.exists() {
        return Err(MeltDown::new(MeltType::ConfigurationError, "Migrations directory not found"));
    }

    let mut migration_dirs = fs::read_dir(migrations_dir)
        .map_err(|e| MeltDown::new(MeltType::FileOperationFailed, format!("Failed to read migrations directory: {}", e)))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();

    migration_dirs.sort_by(|a, b| a.file_name().unwrap_or_default().cmp(&b.file_name().unwrap_or_default()));

    for migration_dir in migration_dirs {
        let migration_name = migration_dir.file_name().unwrap_or_default().to_string_lossy().to_string();
        let db_name_copy = db_name.clone();

        let up_sql_path = migration_dir.join("up.sql");
        if !up_sql_path.exists() {
            cata_log!(Warning, format!("Migration '{}' has no up.sql file, skipping", migration_name));
            continue;
        }

        cata_log!(Info, format!("Running migration '{}'", migration_name));

        let mut file = fs::File::open(&up_sql_path).map_err(|e| MeltDown::new(MeltType::FileOperationFailed, format!("Failed to open migration file: {}", e)))?;

        let mut sql = String::new();
        file.read_to_string(&mut sql)
            .map_err(|e| MeltDown::new(MeltType::FileOperationFailed, format!("Failed to read migration file: {}", e)))?;

        let sql_content = sql.clone();
        let migration_name_copy = migration_name.clone();

        task::spawn_blocking(move || {
            let output = Command::new("psql").arg("-U").arg("postgres").arg("-d").arg(&db_name_copy).arg("-c").arg(&sql_content).output();

            match output {
                Ok(output) => {
                    if output.status.success() {
                        cata_log!(Info, format!("Migration '{}' completed successfully", migration_name_copy));
                        Ok(())
                    } else {
                        let error = String::from_utf8_lossy(&output.stderr);
                        cata_log!(Error, format!("Migration '{}' failed: {}", migration_name_copy, error));
                        Err(MeltDown::new(MeltType::DatabaseError, "Failed to run migration").with_context("error", error.to_string()))
                    }
                }
                Err(e) => {
                    cata_log!(Error, format!("Failed to execute psql command: {}", e));
                    Err(MeltDown::new(MeltType::DatabaseError, "Failed to run migration").with_context("error", e.to_string()))
                }
            }
        })
        .await
        .unwrap_or_else(|e| {
            cata_log!(Error, format!("Task to run migration failed: {}", e));
            Err(MeltDown::new(MeltType::DatabaseError, "Task to run migration failed").with_context("error", e.to_string()))
        })?;
    }

    cata_log!(Info, format!("All migrations completed for database '{}'", db_name));
    Ok(())
}

async fn seed_database(name: &String) -> Result<(), MeltDown> {
    let db_name = name.clone();
    cata_log!(Info, format!("Seeding database '{}'", db_name));

    let seeds_dir = Path::new("src/database/seeds");

    if !seeds_dir.exists() {
        cata_log!(Warning, "Seeds directory not found, skipping seeding");
        return Ok(());
    }

    let mut seed_files = fs::read_dir(seeds_dir)
        .map_err(|e| MeltDown::new(MeltType::FileOperationFailed, format!("Failed to read seeds directory: {}", e)))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().map_or(false, |ext| ext == "sql"))
        .collect::<Vec<_>>();

    seed_files.sort_by(|a, b| a.file_name().unwrap_or_default().cmp(&b.file_name().unwrap_or_default()));

    for seed_file in seed_files {
        let seed_name = seed_file.file_name().unwrap_or_default().to_string_lossy().to_string();
        let db_name_copy = db_name.clone();

        cata_log!(Info, format!("Running seed '{}'", seed_name));

        let mut file = fs::File::open(&seed_file).map_err(|e| MeltDown::new(MeltType::FileOperationFailed, format!("Failed to open seed file: {}", e)))?;

        let mut sql = String::new();
        file.read_to_string(&mut sql)
            .map_err(|e| MeltDown::new(MeltType::FileOperationFailed, format!("Failed to read seed file: {}", e)))?;

        let sql_content = sql.clone();
        let seed_name_copy = seed_name.clone();

        task::spawn_blocking(move || {
            let output = Command::new("psql").arg("-U").arg("postgres").arg("-d").arg(&db_name_copy).arg("-c").arg(&sql_content).output();

            match output {
                Ok(output) => {
                    if output.status.success() {
                        cata_log!(Info, format!("Seed '{}' completed successfully", seed_name_copy));
                        Ok(())
                    } else {
                        let error = String::from_utf8_lossy(&output.stderr);
                        cata_log!(Error, format!("Seed '{}' failed: {}", seed_name_copy, error));
                        Err(MeltDown::new(MeltType::DatabaseError, "Failed to run seed").with_context("error", error.to_string()))
                    }
                }
                Err(e) => {
                    cata_log!(Error, format!("Failed to execute psql command: {}", e));
                    Err(MeltDown::new(MeltType::DatabaseError, "Failed to run seed").with_context("error", e.to_string()))
                }
            }
        })
        .await
        .unwrap_or_else(|e| {
            cata_log!(Error, format!("Task to run seed failed: {}", e));
            Err(MeltDown::new(MeltType::DatabaseError, "Task to run seed failed").with_context("error", e.to_string()))
        })?;
    }

    cata_log!(Info, format!("All seeds completed for database '{}'", db_name));
    Ok(())
}

async fn create_admin_user(db_name: &String, username: &str, email: &str, password_hash: &str, display_name: &str) -> Result<(), MeltDown> {
    cata_log!(Info, format!("Creating admin user '{}' in database '{}'", username, db_name));

    // Extract first and last name from display_name
    let name_parts: Vec<&str> = display_name.split_whitespace().collect();
    let first_name: String;
    let last_name: String;

    match name_parts.len() {
        0 => {
            first_name = "Admin".to_string();
            last_name = "User".to_string();
        }
        1 => {
            first_name = name_parts[0].to_string();
            last_name = "Admin".to_string();
        }
        _ => {
            first_name = name_parts[0].to_string();
            last_name = name_parts[1..].join(" ");
        }
    };

    // Use the vessel username and other properties directly for the tenant admin user
    let insert_sql = format!(
        "INSERT INTO users (
            username, 
            email, 
            first_name, 
            last_name, 
            password_hash, 
            role, 
            active, 
            should_change_password,
            created_at,
            updated_at
        ) 
        VALUES (
            '{}', 
            '{}', 
            '{}', 
            '{}', 
            '{}', 
            'admin', 
            TRUE, 
            FALSE,
            EXTRACT(EPOCH FROM NOW()),
            EXTRACT(EPOCH FROM NOW())
        ) 
        ON CONFLICT (username) DO NOTHING;",
        username, email, first_name, last_name, password_hash
    );

    let db_name_copy = db_name.clone();
    let username_copy = username.to_string();

    task::spawn_blocking(move || {
        let output = Command::new("psql").arg("-U").arg("postgres").arg("-d").arg(&db_name_copy).arg("-c").arg(&insert_sql).output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    cata_log!(Info, format!("Admin user '{}' created successfully in database '{}'", username_copy, db_name_copy));
                    Ok(())
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    cata_log!(Error, format!("Failed to create admin user in database '{}': {}", db_name_copy, error));
                    Err(MeltDown::new(MeltType::DatabaseError, "Failed to create admin user").with_context("error", error.to_string()))
                }
            }
            Err(e) => {
                cata_log!(Error, format!("Failed to execute psql command: {}", e));
                Err(MeltDown::new(MeltType::DatabaseError, "Failed to create admin user").with_context("error", e.to_string()))
            }
        }
    })
    .await
    .unwrap_or_else(|e| {
        cata_log!(Error, format!("Task to create admin user failed: {}", e));
        Err(MeltDown::new(MeltType::DatabaseError, "Task to create admin user failed").with_context("error", e.to_string()))
    })
}

async fn create_admin_user_from_vessel(db_name: &String, vessel: &Vessel) -> Result<(), MeltDown> {
    cata_log!(Info, format!("Creating admin user from vessel data for database '{}'", db_name));

    // Use the first_name and last_name directly from the vessel struct
    let insert_sql = format!(
        "INSERT INTO users (
            username, 
            email, 
            first_name, 
            last_name, 
            password_hash, 
            role, 
            active, 
            should_change_password,
            created_at,
            updated_at
        ) 
        VALUES (
            '{}', 
            '{}', 
            '{}', 
            '{}', 
            '{}', 
            'admin', 
            TRUE, 
            FALSE,
            EXTRACT(EPOCH FROM NOW()),
            EXTRACT(EPOCH FROM NOW())
        ) 
        ON CONFLICT (username) DO NOTHING;",
        vessel.username, vessel.email, vessel.first_name, vessel.last_name, vessel.password_hash
    );

    let db_name_copy = db_name.clone();
    let username_copy = vessel.username.clone();

    task::spawn_blocking(move || {
        let output = Command::new("psql").arg("-U").arg("postgres").arg("-d").arg(&db_name_copy).arg("-c").arg(&insert_sql).output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    cata_log!(Info, format!("Admin user '{}' created successfully in database '{}'", username_copy, db_name_copy));
                    Ok(())
                } else {
                    let error = String::from_utf8_lossy(&output.stderr);
                    cata_log!(Error, format!("Failed to create admin user in database '{}': {}", db_name_copy, error));
                    Err(MeltDown::new(MeltType::DatabaseError, "Failed to create admin user").with_context("error", error.to_string()))
                }
            }
            Err(e) => {
                cata_log!(Error, format!("Failed to execute psql command: {}", e));
                Err(MeltDown::new(MeltType::DatabaseError, "Failed to create admin user").with_context("error", e.to_string()))
            }
        }
    })
    .await
    .unwrap_or_else(|e| {
        cata_log!(Error, format!("Task to create admin user failed: {}", e));
        Err(MeltDown::new(MeltType::DatabaseError, "Task to create admin user failed").with_context("error", e.to_string()))
    })
}

pub fn get_tenant_connection_string(tenant_name: &str) -> String {
    use std::env;

    let prefix_template = env::var("PREFIX_DATABASE_URL").expect("PREFIX_DATABASE_URL environment variable must be set");

    if prefix_template.contains("<database_name>") {
        prefix_template.replace("<database_name>", tenant_name)
    } else {
        let parts: Vec<&str> = prefix_template.splitn(2, "://").collect();

        if parts.len() != 2 {
            panic!("Invalid PREFIX_DATABASE_URL format: Expected protocol://rest");
        }

        let protocol = parts[0];
        let rest_parts: Vec<&str> = parts[1].rsplitn(2, "/").collect();

        if rest_parts.len() < 2 {
            format!("{}://{}/{}", protocol, rest_parts[0], tenant_name)
        } else {
            format!("{}://{}/{}", protocol, rest_parts[1], tenant_name)
        }
    }
}
