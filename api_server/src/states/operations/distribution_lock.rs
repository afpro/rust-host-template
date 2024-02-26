use {
    crate::states::RedisState,
    anyhow::{Context, Error, Result},
    indoc::indoc,
    lazy_static::lazy_static,
    redis::{aio::Connection, Script},
    std::{
        sync::atomic::{AtomicU64, Ordering::Relaxed},
        time::Duration,
    },
    tokio::time::sleep,
};

static LOCK_NUMBER: AtomicU64 = AtomicU64::new(1);

lazy_static! {
    static ref SCRIPT_ACQUIRE: Script = Script::new(indoc! {r#"
        return redis.call('SET', KEYS[1], ARGV[1], 'NX', 'EX', 10);
    "#});
    static ref SCRIPT_EXTEND: Script = Script::new(indoc! {r#"
        if redis.call('GET', KEYS[1]) == ARGV[1]
        then
            return redis.call('EXPIRE', KEYS[1], 10);
        else
            return false;
        end
    "#});
    static ref SCRIPT_REMOVE: Script = Script::new(indoc! {r#"
        if redis.call('GET', KEYS[1]) == ARGV[1]
        then
            redis.call('DEL', KEYS[1]);
        end
    "#});
}

pub struct DistributionLock {
    key: String,
    number: u64,
    conn: Connection,
}

impl Drop for DistributionLock {
    fn drop(&mut self) {}
}

impl DistributionLock {
    #[allow(dead_code)]
    pub async fn acquire<K: ToString>(state: &RedisState, key: K) -> Result<Self> {
        let mut conn = state.get_conn().await?;
        let key = key.to_string();
        let number = LOCK_NUMBER.fetch_add(1, Relaxed);

        if !SCRIPT_ACQUIRE
            .prepare_invoke()
            .key(&key)
            .arg(number)
            .invoke_async::<_, bool>(&mut conn)
            .await
            .unwrap()
        {
            return Err(Error::msg("acquire lock key failed"));
        }

        Ok(Self { key, number, conn })
    }

    #[allow(dead_code)]
    pub async fn use_loop(&mut self) -> Result<()> {
        loop {
            if !SCRIPT_EXTEND
                .prepare_invoke()
                .key(&self.key)
                .arg(self.number)
                .invoke_async(&mut self.conn)
                .await
                .context("extend lock key")?
            {
                return Err(Error::msg("extend lock key failed"));
            }
            sleep(Duration::from_secs(10)).await;
        }
    }

    #[allow(dead_code)]
    pub async fn release(mut self) -> Result<()> {
        SCRIPT_REMOVE
            .prepare_invoke()
            .key(&self.key)
            .arg(self.number)
            .invoke_async::<_, ()>(&mut self.conn)
            .await
            .context("invoke remove script")
    }

    #[allow(dead_code)]
    pub fn lock_number(&self) -> u64 {
        self.number
    }
}

#[cfg(test)]
mod test {
    use crate::states::{operations::distribution_lock::DistributionLock, RedisState};

    #[tokio::test]
    async fn use_lock() {
        let redis = RedisState::dev_state();
        let lock = DistributionLock::acquire(&redis, "LOCK:test")
            .await
            .unwrap();
        lock.release().await.unwrap();
    }
}
