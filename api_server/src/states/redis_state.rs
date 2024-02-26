use {
    anyhow::{Context, Result},
    clap::Args,
    redis::{
        aio::Connection, cmd, Client, ConnectionAddr, ConnectionInfo, IntoConnectionInfo,
        RedisConnectionInfo, RedisResult,
    },
    tracing::{info, instrument},
};

#[cfg(test)]
use crate::test_helper::get_opt_dot_env;

#[derive(Args)]
pub struct RedisStateOpts {
    #[clap(
        name = "redis-host",
        long = "redis-host",
        default_value = "127.0.0.1",
        help = "redis ip or host"
    )]
    host: String,
    #[clap(
        name = "redis-port",
        long = "redis-port",
        default_value = "6379",
        help = "redis port"
    )]
    port: u16,
    #[clap(name = "redis-username", long = "redis-user", help = "redis username")]
    user: Option<String>,
    #[clap(name = "redis-password", long = "redis-pass", help = "redis password")]
    pass: Option<String>,
    #[clap(
        name = "redis-index",
        long = "redis-index",
        default_value = "0",
        help = "redis database index"
    )]
    index: i64,
}

#[derive(Clone)]
pub struct RedisState {
    client: Client,
}

impl IntoConnectionInfo for &RedisStateOpts {
    fn into_connection_info(self) -> RedisResult<ConnectionInfo> {
        Ok(ConnectionInfo {
            addr: ConnectionAddr::Tcp(self.host.clone(), self.port),
            redis: RedisConnectionInfo {
                db: self.index,
                username: self.user.clone(),
                password: self.pass.clone(),
            },
        })
    }
}

impl RedisStateOpts {
    #[cfg(test)]
    #[allow(dead_code)]
    pub fn dev_opts() -> Self {
        let host = get_opt_dot_env("redis_host", "127.0.0.1");
        let port = get_opt_dot_env("redis_port", "6379");
        let user = get_opt_dot_env("redis_user", "");
        let pass = get_opt_dot_env("redis_pass", "");
        let index = get_opt_dot_env("redis_index", "0");

        Self {
            host,
            port: port.parse().unwrap(),
            user: if user.is_empty() { None } else { Some(user) },
            pass: if pass.is_empty() { None } else { Some(pass) },
            index: index.parse().unwrap(),
        }
    }
}

impl From<&RedisState> for RedisState {
    fn from(value: &RedisState) -> Self {
        value.clone()
    }
}

impl RedisState {
    #[instrument("setup-redis", skip_all)]
    pub async fn new(opts: &RedisStateOpts) -> Result<Self> {
        let client = Client::open(opts).context("open redis client")?;
        let inst = Self { client };
        inst.dump_sys_info().await?;
        Ok(inst)
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub fn dev_state() -> Self {
        Self {
            client: Client::open(&RedisStateOpts::dev_opts()).unwrap(),
        }
    }

    pub async fn get_conn(&self) -> Result<Connection> {
        self.client
            .get_async_connection()
            .await
            .context("obtain redis connection")
    }

    #[instrument("dump-sys-info", skip_all)]
    async fn dump_sys_info(&self) -> Result<()> {
        let mut conn = self.get_conn().await?;
        let info = cmd("info")
            .arg("server")
            .query_async::<_, String>(&mut conn)
            .await
            .context("dump redis server info")?;

        let version = info
            .lines()
            .map(|v| v.trim())
            .filter_map(|v| v.strip_prefix("redis_version:"))
            .next()
            .context("can't extract redis_version from info dump")?;

        info!("connected to redis {}", version);

        Ok(())
    }
}
