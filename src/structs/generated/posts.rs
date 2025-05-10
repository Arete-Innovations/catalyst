use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};

use crate::database::schema::posts;
#[derive(Debug, Queryable, Clone, Serialize, Deserialize, Identifiable)]
#[diesel(table_name = posts)]
pub struct Posts {
    pub id: i32,
    pub user_id: i32,
    pub title: String,
    pub content: String,
    pub public: bool,
    pub created_at: i64,
    pub updated_at: i64,
}
