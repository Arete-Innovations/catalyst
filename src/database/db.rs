use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;
use std::env;
use std::collections::HashMap;

// Default connection function
pub fn establish_connection() -> PgConnection {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", database_url))
}

// Get a list of all connection names from .env
pub fn get_connection_names() -> Vec<String> {
    dotenv().ok();
    
    let mut names = Vec::new();
    names.push("default".to_string()); // Default connection is always available
    
    // Look for any DATABASE_URL_* variables
    for (key, _) in env::vars() {
        if key.starts_with("DATABASE_URL_") {
            let name = key.replace("DATABASE_URL_", "").to_lowercase();
            names.push(name);
        }
    }
    
    names
}

// Get all database connection URLs from .env
pub fn get_database_urls() -> HashMap<String, String> {
    dotenv().ok();
    
    let mut db_urls = HashMap::new();
    
    // First check for the default DATABASE_URL
    if let Ok(url) = env::var("DATABASE_URL") {
        db_urls.insert("default".to_string(), url);
    }
    
    // Then look for any other DATABASE_URL_* variables
    for (key, value) in env::vars() {
        if key.starts_with("DATABASE_URL_") {
            let name = key.replace("DATABASE_URL_", "").to_lowercase();
            db_urls.insert(name, value);
        }
    }
    
    db_urls
}

// Additional connection functions will be generated by the blast tool
// based on DATABASE_URL_* entries in the .env file
