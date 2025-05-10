use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

use crate::database::schema::cronjobs;
#[derive(Debug, Queryable, Clone, Serialize, Deserialize, Identifiable)]
#[diesel(table_name = cronjobs)]
pub struct Cronjobs {
    pub id: i32,
    pub name: String,
    pub timer: i32,
    pub status: String,
    pub last_run: Option<i64>,
}
