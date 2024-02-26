use {
    crate::states::{mysql_state::MysqlStateOpts, redis_state::RedisStateOpts},
    anyhow::{Context, Result},
    clap::Args,
    tokio::join,
    tracing::{info_span, instrument, Instrument},
};

pub use crate::states::{mysql_state::MysqlState, redis_state::RedisState};

mod mysql_state;
pub mod operations;
mod redis_state;

#[derive(Args)]
pub struct AppStateOpts {
    #[clap(flatten)]
    redis: RedisStateOpts,
    #[clap(flatten)]
    mysql: MysqlStateOpts,
}

#[derive(Clone)]
pub struct AppState {
    pub redis: RedisState,
    pub mysql: MysqlState,
}

impl AppState {
    #[instrument("create-app-state", skip_all)]
    pub async fn new(opts: &AppStateOpts) -> Result<Self> {
        let (redis, mysql) = join!(
            RedisState::new(&opts.redis).instrument(info_span!("redis-state")),
            MysqlState::new(&opts.mysql).instrument(info_span!("mysql-state")),
        );

        let redis = redis.context("create redis state")?;
        let mysql = mysql.context("create mysql state")?;

        Ok(Self { redis, mysql })
    }
}
