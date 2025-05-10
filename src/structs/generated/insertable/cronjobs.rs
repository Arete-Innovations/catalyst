use diesel::{AsChangeset, Insertable, Queryable};
use serde::{Deserialize, Serialize};

use crate::database::schema::cronjobs;

#[derive(Debug, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = cronjobs)]
pub struct NewCronjobs {
    pub name: String,
    pub timer: i32,
    pub status: String,
    pub last_run: Option<i64>,
}
