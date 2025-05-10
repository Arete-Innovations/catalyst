use chrono::Utc;
use diesel::prelude::*;
use diesel_async::{scoped_futures::ScopedFutureExt, AsyncConnection, RunQueryDsl};

use crate::{
    database::{
        db::establish_connection,
        schema::posts::dsl::{self as post_dsl},
    },
    meltdown::*,
    structs::*,
};

impl Posts {
    pub async fn get_all() -> Result<Vec<Posts>, MeltDown> {
        let mut conn = establish_connection().await;

        post_dsl::posts
            .order(post_dsl::id.asc())
            .load::<Posts>(&mut conn)
            .await
            .map_err(|e: diesel::result::Error| MeltDown::from(e).with_context("operation", "get_all"))
    }

    pub async fn get_by_id(id: i32) -> Result<Posts, MeltDown> {
        let mut conn = establish_connection().await;

        post_dsl::posts
            .filter(post_dsl::id.eq(id))
            .first::<Posts>(&mut conn)
            .await
            .map_err(|e: diesel::result::Error| MeltDown::from(e).with_context("operation", "get_by_id").with_context("id", id.to_string()))
    }

    pub async fn create(new_record: NewPosts) -> Result<Posts, MeltDown> {
        let mut conn = establish_connection().await;

        diesel::insert_into(post_dsl::posts)
            .values(&new_record)
            .get_result::<Posts>(&mut conn)
            .await
            .map_err(|e| MeltDown::from(e).with_context("operation", "create"))
    }

    pub async fn update_by_id(id: i32, updates: &NewPosts) -> Result<Posts, MeltDown> {
        let mut conn = establish_connection().await;

        diesel::update(post_dsl::posts.filter(post_dsl::id.eq(id)))
            .set(updates)
            .get_result::<Posts>(&mut conn)
            .await
            .map_err(|e| MeltDown::from(e).with_context("operation", "update_by_id").with_context("id", id.to_string()))
    }

    pub async fn delete_by_id(id: i32) -> Result<(), MeltDown> {
        let mut conn = establish_connection().await;

        conn.transaction::<_, MeltDown, _>(|conn| {
            async move {
                let _ = post_dsl::posts.filter(post_dsl::id.eq(id)).first::<Posts>(conn).await?;

                diesel::delete(post_dsl::posts.filter(post_dsl::id.eq(id))).execute(conn).await?;

                Ok(())
            }
            .scope_boxed()
        })
        .await
        .map_err(|e| MeltDown::from(e).with_context("operation", "delete_by_id").with_context("id", id.to_string()))
    }

    pub async fn count() -> Result<i64, MeltDown> {
        let mut conn = establish_connection().await;

        post_dsl::posts
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .map_err(|e: diesel::result::Error| MeltDown::from(e).with_context("operation", "count"))
    }
    pub async fn is_public(&self) -> bool {
        self.public
    }

    pub async fn set_public(&mut self, value: bool) -> Result<Self, MeltDown> {
        let mut conn = establish_connection().await;
        let current_timestamp = Utc::now().timestamp();
        let item_id = self.id;

        let updated = diesel::update(post_dsl::posts.filter(post_dsl::id.eq(item_id)))
            .set((post_dsl::public.eq(value), post_dsl::updated_at.eq(current_timestamp)))
            .get_result::<Self>(&mut conn)
            .await
            .map_err(|e| MeltDown::from(e).with_context("operation", "set_public").with_context("id", item_id.to_string()))?;

        *self = updated.clone();
        Ok(updated)
    }

    pub async fn set_public_true(&mut self) -> Result<Self, MeltDown> {
        self.set_public(true).await
    }

    pub async fn set_public_false(&mut self) -> Result<Self, MeltDown> {
        self.set_public(false).await
    }

    pub async fn created_after(timestamp: i64) -> Result<Vec<Self>, MeltDown> {
        let mut conn = establish_connection().await;

        post_dsl::posts
            .filter(post_dsl::created_at.gt(timestamp))
            .order(post_dsl::created_at.desc())
            .load::<Self>(&mut conn)
            .await
            .map_err(|e: diesel::result::Error| MeltDown::from(e).with_context("operation", "created_after").with_context("timestamp", timestamp.to_string()))
    }

    pub async fn created_before(timestamp: i64) -> Result<Vec<Self>, MeltDown> {
        let mut conn = establish_connection().await;

        post_dsl::posts
            .filter(post_dsl::created_at.lt(timestamp))
            .order(post_dsl::created_at.desc())
            .load::<Self>(&mut conn)
            .await
            .map_err(|e: diesel::result::Error| MeltDown::from(e).with_context("operation", "created_before").with_context("timestamp", timestamp.to_string()))
    }

    pub async fn created_between(start: i64, end: i64) -> Result<Vec<Self>, MeltDown> {
        let mut conn = establish_connection().await;

        post_dsl::posts
            .filter(post_dsl::created_at.ge(start).and(post_dsl::created_at.le(end)))
            .order(post_dsl::created_at.desc())
            .load::<Self>(&mut conn)
            .await
            .map_err(|e: diesel::result::Error| {
                MeltDown::from(e)
                    .with_context("operation", "created_between")
                    .with_context("start", start.to_string())
                    .with_context("end", end.to_string())
            })
    }

    pub async fn recent(limit: i64) -> Result<Vec<Self>, MeltDown> {
        let mut conn = establish_connection().await;

        post_dsl::posts
            .order(post_dsl::created_at.desc())
            .limit(limit)
            .load::<Self>(&mut conn)
            .await
            .map_err(|e: diesel::result::Error| MeltDown::from(e).with_context("operation", "recent").with_context("limit", limit.to_string()))
    }

    pub async fn updated_after(timestamp: i64) -> Result<Vec<Self>, MeltDown> {
        let mut conn = establish_connection().await;

        post_dsl::posts
            .filter(post_dsl::updated_at.gt(timestamp))
            .order(post_dsl::updated_at.desc())
            .load::<Self>(&mut conn)
            .await
            .map_err(|e: diesel::result::Error| MeltDown::from(e).with_context("operation", "updated_after").with_context("timestamp", timestamp.to_string()))
    }

    pub async fn recently_updated(limit: i64) -> Result<Vec<Self>, MeltDown> {
        let mut conn = establish_connection().await;

        post_dsl::posts
            .order(post_dsl::updated_at.desc())
            .limit(limit)
            .load::<Self>(&mut conn)
            .await
            .map_err(|e: diesel::result::Error| MeltDown::from(e).with_context("operation", "recently_updated").with_context("limit", limit.to_string()))
    }

    pub async fn get_by_user_id(user_id: i32) -> Result<Vec<Self>, MeltDown> {
        let mut conn = establish_connection().await;

        post_dsl::posts
            .filter(post_dsl::user_id.eq(user_id))
            .load::<Self>(&mut conn)
            .await
            .map_err(|e: diesel::result::Error| MeltDown::from(e).with_context("operation", "get_by_user_id").with_context("user_id", user_id.to_string()))
    }

    pub async fn get_by_user_id_created_before(user_id: i32, timestamp: i64) -> Result<Vec<Self>, MeltDown> {
        let mut conn = establish_connection().await;

        post_dsl::posts
            .filter(post_dsl::user_id.eq(user_id))
            .filter(post_dsl::created_at.lt(timestamp))
            .order(post_dsl::created_at.desc())
            .load::<Self>(&mut conn)
            .await
            .map_err(|e: diesel::result::Error| {
                MeltDown::from(e)
                    .with_context("operation", "get_by_user_id_created_before")
                    .with_context("user_id", user_id.to_string())
                    .with_context("timestamp", timestamp.to_string())
            })
    }

    pub async fn get_by_user_id_created_after(user_id: i32, timestamp: i64) -> Result<Vec<Self>, MeltDown> {
        let mut conn = establish_connection().await;

        post_dsl::posts
            .filter(post_dsl::user_id.eq(user_id))
            .filter(post_dsl::created_at.gt(timestamp))
            .order(post_dsl::created_at.desc())
            .load::<Self>(&mut conn)
            .await
            .map_err(|e: diesel::result::Error| {
                MeltDown::from(e)
                    .with_context("operation", "get_by_user_id_created_after")
                    .with_context("user_id", user_id.to_string())
                    .with_context("timestamp", timestamp.to_string())
            })
    }
}
