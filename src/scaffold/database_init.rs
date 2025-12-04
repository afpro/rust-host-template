use std::ops::DerefMut;

use anyhow::{Context, Result};
use diesel::{QueryableByName, sql_query};
use diesel_async::{
    AsyncPgConnection, RunQueryDsl,
    pooled_connection::{AsyncDieselConnectionManager, bb8::Pool},
};
use tracing::info;

#[derive(QueryableByName)]
struct DbMeta {
    #[diesel(sql_type = diesel::sql_types::VarChar)]
    version: String,
}

pub type DbPool = Pool<AsyncPgConnection>;

pub async fn create(config: &str, max_size: u32) -> Result<DbPool> {
    let config = AsyncDieselConnectionManager::<AsyncPgConnection>::new(config);
    DbPool::builder()
        .max_size(max_size)
        .build(config)
        .await
        .context("create database pool")
}

pub async fn check(pool: &DbPool) -> Result<()> {
    let mut database_conn = pool.get().await.context("connect to database")?;
    let version = sql_query("select version() as version")
        .get_result::<DbMeta>(database_conn.deref_mut())
        .await
        .context("query pg meta")?
        .version;
    info!(%version, "database server info queried");
    Ok(())
}
