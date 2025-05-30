use std::env;

use diesel_async::{AsyncConnection, AsyncPgConnection};

use crate::{cata_log, meltdown::*};

pub async fn establish_connection() -> Result<AsyncPgConnection, MeltDown> {
    let database_url = env::var("VESSEL_DATABASE_URL").unwrap_or_else(|_| env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://postgres:postgres@localhost/vessel".to_string()));

    cata_log!(Info, format!("Connecting to vessel database: {}", database_url));

    match AsyncPgConnection::establish(&database_url).await {
        Ok(conn) => Ok(conn),
        Err(e) => {
            let error_message = format!("Error connecting to vessel database: {}", e);
            cata_log!(Error, &error_message);
            Err(MeltDown::new(MeltType::DatabaseError, "Failed to connect to vessel database").with_context("error", &error_message))
        }
    }
}
