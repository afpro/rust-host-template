use std::ops::DerefMut;

use anyhow::{Context, Result, anyhow};
use bb8::Pool;
use redis::{Client, cmd};
use tracing::info;

use crate::scaffold::pretty::Pretty;

pub type CachePool = Pool<Client>;

pub async fn create(config: &str, max_size: u32) -> Result<CachePool> {
    let client = Client::open(config).context("create cache client")?;
    Pool::builder()
        .max_size(max_size)
        .build(client)
        .await
        .context("create cache pool")
}

pub async fn check(pool: &CachePool) -> Result<()> {
    let mut cache_conn = pool.get().await.context("connect to cache")?;

    let info = cmd("info")
        .arg("server")
        .query_async::<String>(cache_conn.deref_mut())
        .await
        .context("dump redis server info")?;

    let infos = info
        .lines()
        .map(|v| v.trim())
        .filter(|v| {
            v.starts_with("redis_version:")
                || v.starts_with("valkey_version:")
                || v.starts_with("server_name:")
        })
        .collect::<Vec<_>>();

    if infos.is_empty() {
        return Err(anyhow!("no valid server info found"));
    }

    info!(infos=?Pretty(infos), "cache server info queried");
    Ok(())
}
