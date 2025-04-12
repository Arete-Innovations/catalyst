use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenv::dotenv;
use std::collections::HashMap;
use std::env;

pub fn establish_connection() -> PgConnection {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    PgConnection::establish(&database_url).expect(&format!("Error connecting to {}", database_url))
}

pub fn get_connection_names() -> Vec<String> {
    dotenv().ok();
    let mut names = Vec::new();
    names.push("default".to_string());

    for (key, _) in env::vars() {
        if key.starts_with("DATABASE_URL_") {
            let name = key.replace("DATABASE_URL_", "").to_lowercase();
            names.push(name);
        }
    }

    names
}

pub fn get_database_urls() -> HashMap<String, String> {
    dotenv().ok();
    let mut db_urls = HashMap::new();

    if let Ok(url) = env::var("DATABASE_URL") {
        db_urls.insert("default".to_string(), url);
    }

    for (key, value) in env::vars() {
        if key.starts_with("DATABASE_URL_") {
            let name = key.replace("DATABASE_URL_", "").to_lowercase();
            db_urls.insert(name, value);
        }
    }

    db_urls
}
