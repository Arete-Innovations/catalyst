use crate::meltdown::*;
use diesel::pg::PgConnection;
use diesel::r2d2::{self, ConnectionManager, Pool};
use dotenv::dotenv;
use std::collections::HashMap;
use std::env;
use std::sync::{Arc, OnceLock};

const DEFAULT_POOL_NAME: &str = "default";
const MAX_POOL_SIZE: u32 = 150;
const MIN_IDLE_CONNECTIONS: u32 = 5;

type PgPool = Pool<ConnectionManager<PgConnection>>;
pub type PgPooledConnection = r2d2::PooledConnection<ConnectionManager<PgConnection>>;

static DB_POOLS: OnceLock<Arc<HashMap<String, PgPool>>> = OnceLock::new();

pub fn init_connection_pools() -> Result<(), MeltDown> {
    dotenv().ok();

    let url = env::var("DATABASE_URL").map_err(|e| MeltDown::new(MeltType::EnvironmentError, format!("DATABASE_URL environment variable not found: {}", e)))?;

    let manager = ConnectionManager::<PgConnection>::new(url);
    let pool = r2d2::Pool::builder()
        .max_size(MAX_POOL_SIZE)
        .min_idle(Some(MIN_IDLE_CONNECTIONS))
        .build(manager)
        .map_err(|e| MeltDown::db_connection(format!("Failed to create pool for default database: {}", e)))?;

    let mut pools = HashMap::new();
    pools.insert(DEFAULT_POOL_NAME.to_string(), pool);

    DB_POOLS
        .set(Arc::new(pools))
        .map_err(|_| MeltDown::new(MeltType::ConfigurationError, "Failed to initialize database pools: already initialized"))
}

pub fn establish_connection_safe() -> Result<PgPooledConnection, MeltDown> {
    get_pooled_connection(DEFAULT_POOL_NAME).map_err(|e| MeltDown::db_connection(format!("Failed to get connection from default pool: {}", e)))
}

pub fn establish_connection() -> PgPooledConnection {
    use crate::cata_log;

    establish_connection_safe().unwrap_or_else(|e| {
        cata_log!(Error, format!("Database connection error: {}", e));
        panic!("Database connection error: {}", e)
    })
}

pub fn get_pooled_connection(name: &str) -> Result<PgPooledConnection, MeltDown> {
    let pools = DB_POOLS.get().ok_or_else(|| MeltDown::new(MeltType::DatabaseConnection, "Database pools not initialized"))?;

    pools
        .get(name)
        .ok_or_else(|| MeltDown::new(MeltType::DatabaseConnection, format!("Pool '{}' not found", name)).with_context("requested_pool", name))
        .and_then(|pool| pool.get().map_err(|err| MeltDown::new(MeltType::DatabaseConnection, format!("Pool error: {}", err)).with_context("pool", name)))
}

pub fn get_connection_names() -> Vec<String> {
    dotenv().ok();
    let mut names = vec![DEFAULT_POOL_NAME.to_string()];

    for (key, _) in env::vars() {
        if key.ends_with("_DATABASE_URL") {
            let name = key.replace("_DATABASE_URL", "").to_lowercase();
            names.push(name);
        }
    }

    names
}

pub fn get_database_urls() -> HashMap<String, String> {
    dotenv().ok();
    let mut db_urls = HashMap::new();

    if let Ok(url) = env::var("DATABASE_URL") {
        db_urls.insert(DEFAULT_POOL_NAME.to_string(), url);
    }

    for (key, value) in env::vars().filter(|(k, _)| k.ends_with("_DATABASE_URL")) {
        let name = key.replace("_DATABASE_URL", "").to_lowercase();
        db_urls.insert(name, value);
    }

    db_urls
}
