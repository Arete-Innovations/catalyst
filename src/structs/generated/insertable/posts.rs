use diesel::{AsChangeset, Insertable, Queryable};
use serde::{Deserialize, Serialize};

use crate::database::schema::posts;

#[derive(Debug, Insertable, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = posts)]
pub struct NewPosts {
    pub user_id: i32,
    pub title: String,
    pub content: String,
}
