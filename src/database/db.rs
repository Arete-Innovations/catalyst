use std::{
    collections::HashMap,
    env,
    sync::{Mutex, OnceLock},
};

use diesel_async::{
    pooled_connection::{
        deadpool::{Object, Pool},
        AsyncDieselConnectionManager,
    },
    AsyncConnection, AsyncPgConnection,
};
use dotenv::dotenv;
use once_cell::sync::Lazy;

use crate::{cata_log, database::tenant::TenantConnection, meltdown::*};

const MAX_POOL_SIZE: usize = 20;
const MAX_TENANT_POOL_SIZE: usize = 5;

type DbPool = Pool<AsyncPgConnection>;
type PooledConn = Object<AsyncPgConnection>;

static DB_POOL: OnceLock<DbPool> = OnceLock::new();
static TENANT_POOLS: Lazy<Mutex<HashMap<String, DbPool>>> = Lazy::new(|| Mutex::new(HashMap::new()));

pub async fn init_connection_pool() -> Result<(), MeltDown> {
    dotenv().ok();

    if DB_POOL.get().is_some() {
        cata_log!(Debug, "Connection pool already initialized");
        return Ok(());
    }

    let database_url = env::var("DATABASE_URL").map_err(|e| MeltDown::new(MeltType::EnvironmentError, format!("DATABASE_URL not set: {}", e)))?;

    cata_log!(Info, "Initializing database connection pool");

    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(database_url);

    let pool = Pool::builder(config)
        .max_size(MAX_POOL_SIZE)
        .build()
        .map_err(|e| MeltDown::db_connection(format!("Failed to create connection pool: {}", e)))?;

    let _ = pool.get().await.map_err(|e| MeltDown::db_connection(format!("Failed to verify connection pool: {}", e)))?;

    match DB_POOL.set(pool) {
        Ok(_) => {
            cata_log!(Info, "Database connection pool initialized successfully");
            Ok(())
        }
        Err(_) => Err(MeltDown::new(MeltType::ConfigurationError, "Failed to set connection pool: already initialized")),
    }
}

async fn get_conn_from_pool() -> Result<PooledConn, MeltDown> {
    let pool = DB_POOL.get().ok_or_else(|| MeltDown::new(MeltType::DatabaseConnection, "Database pool not initialized"))?;

    pool.get().await.map_err(|e| MeltDown::db_connection(format!("Failed to get connection from pool: {}", e)))
}

pub async fn establish_connection() -> AsyncPgConnection {
    match get_pooled_connection().await {
        Ok(conn) => conn,
        Err(e) => {
            cata_log!(Error, format!("Database connection error: {}", e));
            panic!("Database connection error: {}", e);
        }
    }
}

async fn get_pooled_connection() -> Result<AsyncPgConnection, MeltDown> {
    let database_url = env::var("DATABASE_URL").map_err(|e| MeltDown::new(MeltType::EnvironmentError, format!("DATABASE_URL not set: {}", e)))?;

    let _pooled = get_conn_from_pool().await?;

    AsyncPgConnection::establish(&database_url)
        .await
        .map_err(|e| MeltDown::db_connection(format!("Error connecting to database: {}", e)))
}

pub async fn get_or_create_tenant_pool(tenant_name: &str) -> Result<DbPool, MeltDown> {
    cata_log!(Info, format!("Getting or creating tenant pool for database: {}", tenant_name));

    {
        let tenant_pools = TENANT_POOLS.lock().map_err(|_| MeltDown::new(MeltType::ConfigurationError, "Failed to acquire lock on tenant pools".to_string()))?;

        if let Some(pool) = tenant_pools.get(tenant_name) {
            cata_log!(Debug, format!("Found existing pool for tenant database: {}", tenant_name));
            return Ok(pool.clone());
        }

        cata_log!(Info, format!("No existing pool found, creating new pool for tenant database: {}", tenant_name));
    }

    let tenant_conn = TenantConnection::from_env(tenant_name.to_string()).map_err(|e| MeltDown::new(MeltType::EnvironmentError, format!("Failed to create tenant connection: {}", e)))?;

    let connection_string = tenant_conn.build_connection_string();
    cata_log!(Debug, format!("Created connection string for tenant: {}", tenant_name));

    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(connection_string.clone());

    let pool = Pool::builder(config)
        .max_size(MAX_TENANT_POOL_SIZE)
        .build()
        .map_err(|e| MeltDown::db_connection(format!("Failed to create tenant connection pool: {}", e)))?;

    cata_log!(Debug, format!("Testing connection to tenant database: {}", tenant_name));
    let conn_test = pool.get().await.map_err(|e| MeltDown::db_connection(format!("Failed to verify tenant connection pool: {}", e)))?;
    drop(conn_test);
    cata_log!(Info, format!("Successfully connected to tenant database: {}", tenant_name));

    {
        let mut tenant_pools = TENANT_POOLS.lock().map_err(|_| MeltDown::new(MeltType::ConfigurationError, "Failed to acquire lock on tenant pools".to_string()))?;

        let pool_clone = pool.clone();
        tenant_pools.insert(tenant_name.to_string(), pool);
        cata_log!(Info, format!("Added pool for tenant database: {} to tenant pools cache", tenant_name));

        Ok(pool_clone)
    }
}

pub async fn establish_tenant_connection(tenant_name: &str) -> Result<AsyncPgConnection, MeltDown> {
    cata_log!(Info, format!("Establishing connection to tenant database: {}", tenant_name));

    let _pool = get_or_create_tenant_pool(tenant_name).await?;

    let tenant_conn = TenantConnection::from_env(tenant_name.to_string()).map_err(|e| {
        cata_log!(Error, format!("Failed to create tenant connection from env: {}", e));
        MeltDown::new(MeltType::EnvironmentError, format!("Failed to create tenant connection: {}", e))
    })?;

    let connection_string = tenant_conn.build_connection_string();

    cata_log!(Info, format!("Connecting to tenant database: {}", tenant_name));

    let connection = AsyncPgConnection::establish(&connection_string).await.map_err(|e| {
        cata_log!(Error, format!("Error connecting to tenant database ({}): {}", tenant_name, e));
        MeltDown::db_connection(format!("Error connecting to tenant database: {}", e))
    })?;

    cata_log!(Debug, format!("Successfully established connection to tenant database: {}", tenant_name));

    Ok(connection)
}

pub async fn establish_connection_with_tenant(tenant_name: &str) -> Result<AsyncPgConnection, MeltDown> {
    cata_log!(Info, format!("Establishing connection with tenant: {}", tenant_name));
    establish_tenant_connection(tenant_name).await
}
