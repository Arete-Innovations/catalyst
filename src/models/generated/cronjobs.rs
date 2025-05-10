use chrono::Utc;
use diesel::prelude::*;
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection, RunQueryDsl};

use crate::{
    database::{
        db::establish_connection,
        schema::cronjobs::dsl::{self as cronjob_dsl},
    },
    meltdown::*,
    structs::*,
};

impl Cronjobs {
    pub async fn get_all() -> Result<Vec<Cronjobs>, MeltDown> {
        let mut conn = establish_connection().await;

        cronjob_dsl::cronjobs
            .order(cronjob_dsl::id.asc())
            .load::<Cronjobs>(&mut conn)
            .await
            .map_err(|e: diesel::result::Error| MeltDown::from(e).with_context("operation", "get_all"))
    }

    pub async fn get_by_id(id: i32) -> Result<Cronjobs, MeltDown> {
        let mut conn = establish_connection().await;

        cronjob_dsl::cronjobs
            .filter(cronjob_dsl::id.eq(id))
            .first::<Cronjobs>(&mut conn)
            .await
            .map_err(|e: diesel::result::Error| MeltDown::from(e).with_context("operation", "get_by_id").with_context("id", id.to_string()))
    }

    pub async fn create(new_record: NewCronjobs) -> Result<Cronjobs, MeltDown> {
        let mut conn = establish_connection().await;

        diesel::insert_into(cronjob_dsl::cronjobs)
            .values(&new_record)
            .get_result::<Cronjobs>(&mut conn)
            .await
            .map_err(|e| MeltDown::from(e).with_context("operation", "create"))
    }

    pub async fn update_by_id(id: i32, updates: &NewCronjobs) -> Result<Cronjobs, MeltDown> {
        let mut conn = establish_connection().await;

        diesel::update(cronjob_dsl::cronjobs.filter(cronjob_dsl::id.eq(id)))
            .set(updates)
            .get_result::<Cronjobs>(&mut conn)
            .await
            .map_err(|e| MeltDown::from(e).with_context("operation", "update_by_id").with_context("id", id.to_string()))
    }

    pub async fn delete_by_id(id: i32) -> Result<(), MeltDown> {
        let mut conn = establish_connection().await;

        conn.transaction::<_, MeltDown, _>(|conn| {
            async move {
                let _ = cronjob_dsl::cronjobs.filter(cronjob_dsl::id.eq(id)).first::<Cronjobs>(conn).await?;

                diesel::delete(cronjob_dsl::cronjobs.filter(cronjob_dsl::id.eq(id))).execute(conn).await?;

                Ok(())
            }
            .scope_boxed()
        })
        .await
        .map_err(|e| MeltDown::from(e).with_context("operation", "delete_by_id").with_context("id", id.to_string()))
    }

    pub async fn count() -> Result<i64, MeltDown> {
        let mut conn = establish_connection().await;

        cronjob_dsl::cronjobs
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .map_err(|e: diesel::result::Error| MeltDown::from(e).with_context("operation", "count"))
    }
}
