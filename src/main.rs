mod api;
mod db_enum;
mod scaffold;
mod schema;

use std::net::SocketAddr;

use anyhow::{Context, Result};
use axum::{Router, http::StatusCode, routing::get};
use clap::Parser;
use serde::Deserialize;
use tokio::net::TcpListener;
use tracing::{Instrument, info, info_span};

use crate::{
    api::state::HostState,
    scaffold::{access_log::AccessLog, quit_sig, tracing_output},
};

#[derive(Parser, Deserialize)]
struct Opts {
    #[clap(
        long = "bind",
        default_value = "127.0.0.1:5000",
        help = "api bind addr"
    )]
    bind: SocketAddr,

    #[clap(long = "remote-header", help = "remote header(eg. X-Forward-Ip)")]
    remote_header: Option<String>,

    #[clap(long = "log-dir", default_value = "logs", help = "log output dir")]
    log_dir: String,

    #[clap(
        long = "log-filter",
        default_value = "debug",
        help = "log global filter"
    )]
    log_filter: String,

    #[clap(long = "log-loki", help = "log loki push endpoint")]
    log_loki: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let Opts {
        bind,
        remote_header,
        log_dir,
        log_filter,
        log_loki,
    } = Opts::parse();

    let (_tracing_file_guard, tracing_loki_guard) =
        tracing_output::setup(&log_dir, &log_filter, log_loki.as_deref())
            .context("setup tracing")?;

    let state = HostState::new(remote_header);
    let router = Router::new()
        .layer(AccessLog::new(state.clone()))
        .with_state(state)
        .route("/gen_204", get(|| async { StatusCode::NO_CONTENT }));

    // bind tcp socket
    let tcp_listener = TcpListener::bind(bind)
        .await
        .with_context(|| format!("can't bind tcp socket at {}", bind))?;
    let tcp_bind = tcp_listener.local_addr().context("get tcp bound addr")?;
    info!("bound at {}", tcp_bind);

    // enter task loop
    info!("host start up");
    axum::serve(
        tcp_listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(quit_sig::wait().instrument(info_span!("wait-quit-sig")))
    .await
    .context("run http")?;

    // clean
    info!("host shutdown");

    if let Some(guard) = tracing_loki_guard {
        info!("shutdown loki");
        guard.shutdown().await;
        info!("shutdown loki ok");
    }

    Ok(())
}
