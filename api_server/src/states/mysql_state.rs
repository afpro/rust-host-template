use {
    crate::never_error::NeverError,
    anyhow::{Context, Result},
    clap::Args,
    mysql_async::{Conn, Opts, OptsBuilder, Pool},
    tracing::{info, instrument},
};

#[cfg(test)]
use crate::test_helper::get_opt_dot_env;

#[derive(Args)]
pub struct MysqlStateOpts {
    #[clap(
        name = "mysql-host",
        long = "mysql-host",
        default_value = "127.0.0.1",
        help = "mysql ip or host"
    )]
    host: String,
    #[clap(
        name = "mysql-port",
        long = "mysql-port",
        default_value = "3306",
        help = "mysql port"
    )]
    port: u16,
    #[clap(
        name = "mysql-username",
        long = "mysql-user",
        default_value = "root",
        help = "mysql username"
    )]
    user: String,
    #[clap(
        name = "mysql-password",
        long = "mysql-pass",
        default_value = "",
        help = "mysql password"
    )]
    pass: String,
    #[clap(
        name = "mysql-db-name",
        long = "mysql-db-name",
        default_value = "{{mysql_db_name}}",
        help = "mysql database name"
    )]
    name: String,
}

#[derive(Clone)]
pub struct MysqlState {
    pool: Pool,
}

impl MysqlStateOpts {
    #[cfg(test)]
    #[allow(dead_code)]
    pub fn dev_opts() -> Self {
        let host = get_opt_dot_env("mysql_host", "127.0.0.1");
        let port = get_opt_dot_env("mysql_port", "3306");
        let user = get_opt_dot_env("mysql_user", "root");
        let pass = get_opt_dot_env("mysql_user", "");
        let db_name = get_opt_dot_env("mysql_db_name", "cross_copy");

        Self {
            host,
            port: port.parse().unwrap(),
            user,
            pass,
            name: db_name,
        }
    }
}

impl From<&MysqlState> for MysqlState {
    fn from(value: &MysqlState) -> Self {
        value.clone()
    }
}

impl TryFrom<&MysqlStateOpts> for Opts {
    type Error = NeverError;

    fn try_from(value: &MysqlStateOpts) -> Result<Self, NeverError> {
        Ok(Opts::from(
            OptsBuilder::default()
                .ip_or_hostname(&value.host)
                .tcp_port(value.port)
                .user(Some(&value.user))
                .pass(Some(&value.pass))
                .db_name(Some(&value.name)),
        ))
    }
}

impl MysqlState {
    #[instrument("setup-mysql", skip_all)]
    pub async fn new(opts: &MysqlStateOpts) -> Result<Self> {
        let pool = Pool::new(opts);
        let inst = Self { pool };
        inst.dump_sys_info().await?;
        Ok(inst)
    }

    #[cfg(test)]
    #[allow(dead_code)]
    pub fn dev_state() -> Self {
        Self {
            pool: Pool::new(&MysqlStateOpts::dev_opts()),
        }
    }

    pub async fn get_conn(&self) -> Result<Conn> {
        self.pool
            .get_conn()
            .await
            .context("obtain mysql connection")
    }

    #[instrument("dump-sys-info", skip_all)]
    async fn dump_sys_info(&self) -> Result<()> {
        let conn = self.get_conn().await?;
        let (major, minor, patch) = conn.server_version();
        info!("connected to mysql {}.{}.{}", major, minor, patch);
        Ok(())
    }
}
