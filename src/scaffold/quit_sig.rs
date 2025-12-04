use std::future::pending;

use tracing::warn;

pub async fn wait() {
    if cfg!(unix) {
        #[cfg(unix)]
        unix::wait().await;
    } else if cfg!(windows) {
        #[cfg(windows)]
        win::wait().await;
    } else {
        warn!("quit signal not support on this platform");
        pending().await
    }
}

#[cfg(windows)]
mod win {
    use std::future::pending;

    use tokio::signal::ctrl_c;
    use tracing::info;

    use crate::scaffold::pretty::Pretty;

    async fn wait() {
        match ctrl_c().await {
            Ok(_) => {
                info!("CTRL+C received");
            }
            Err(err) => {
                info!(err=?Pretty(err), "CTRL+C observer error");
                pending().await
            }
        }
    }
}

#[cfg(unix)]
mod unix {
    use std::future::pending;

    use tokio::{
        select,
        signal::unix::{SignalKind, signal},
    };
    use tracing::{error, info, instrument, warn};

    use crate::scaffold::pretty::Pretty;

    #[instrument("wait-sig", skip_all, fields(kind=?kind))]
    async fn wait_sig(kind: SignalKind) {
        let sig = match signal(kind) {
            Ok(v) => v,
            Err(err) => {
                warn!(err=?Pretty(err), "create signal with error");
                return pending().await;
            }
        };

        let received_sig = {
            let mut sig = sig;
            sig.recv().await
        };

        match received_sig {
            Some(_) => {
                info!("signal received");
            }
            None => {
                error!("signal remote closed?");
                pending().await
            }
        }
    }

    pub async fn wait() {
        select! {
            _ = wait_sig(SignalKind::interrupt()) => {}
            _ = wait_sig(SignalKind::terminate()) => {}
        }
    }
}
