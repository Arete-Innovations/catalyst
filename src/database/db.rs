use crate::cata_log;
use crate::meltdown::*;
use diesel_async::pooled_connection::deadpool::{Object, Pool};
use diesel_async::pooled_connection::AsyncDieselConnectionManager;
use diesel_async::{AsyncConnection, AsyncPgConnection};
use dotenv::dotenv;
use std::env;
use std::sync::OnceLock;

const MAX_POOL_SIZE: usize = 20;

type DbPool = Pool<AsyncPgConnection>;
type PooledConn = Object<AsyncPgConnection>;

static DB_POOL: OnceLock<DbPool> = OnceLock::new();

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

