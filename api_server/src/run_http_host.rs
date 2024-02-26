use {
    crate::{
        layers::access_log::AccessLog,
        states::{AppState, AppStateOpts},
    },
    anyhow::{Context, Result},
    axum::Router,
    clap::Args,
    std::net::SocketAddr,
    tokio::{net::TcpListener, signal::ctrl_c},
    tracing::{error, info, instrument},
};

#[derive(Args)]
pub struct RunHttpHostArgs {
    #[clap(
        short = 'b',
        long = "bind",
        default_value = "127.0.0.1:3000",
        help = "api bind address"
    )]
    bind: SocketAddr,
    #[clap(flatten)]
    state: AppStateOpts,
}

impl RunHttpHostArgs {
    #[instrument("api", skip_all)]
    pub async fn run(&self) -> Result<()> {
        let tcp_listener = TcpListener::bind(self.bind)
            .await
            .with_context(|| format!("can't bind tcp socket {}", self.bind))?;
        info!("bound at {}", self.bind);

        let state = AppState::new(&self.state)
            .await
            .context("prepare http state")?;

        let router = Router::new()
            .nest("/", crate::modules::router())
            .with_state(state)
            .layer(AccessLog);

        axum::serve(
            tcp_listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(Self::graceful_shutdown())
        .await
        .context("run http")?;

        Ok(())
    }

    #[instrument("graceful-shutdown")]
    async fn graceful_shutdown() {
        match ctrl_c().await {
            Ok(_) => {
                info!("CTRL+C pressed, quiting");
            }
            Err(err) => {
                error!("tokio CTRL+C signal handler error {}", err);
            }
        }
    }
}
