use anyhow::{Context, Result};
use tokio::spawn;
use tracing::level_filters::LevelFilter;
use tracing_appender::{non_blocking::WorkerGuard, rolling::Rotation};
use tracing_loki::BackgroundTaskController;
use tracing_subscriber::{
    EnvFilter, Layer, filter::Filtered, layer::SubscriberExt, util::SubscriberInitExt,
};
use url::Url;

pub fn setup(
    output_dir: &str,
    filter: &str,
    loki: Option<&str>,
) -> Result<(WorkerGuard, Option<BackgroundTaskController>)> {
    let file_rolling = tracing_appender::rolling::Builder::new()
        .filename_prefix("host")
        .filename_suffix("log")
        .rotation(Rotation::DAILY)
        .max_log_files(7) // more logs in loki
        .build(output_dir)
        .context("create tracing file rolling output")?;

    let (file_rolling, guard) = tracing_appender::non_blocking(file_rolling);

    let env_filter = EnvFilter::builder()
        .with_default_directive(LevelFilter::DEBUG.into())
        .parse_lossy(filter);

    let registry = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(env_filter.clone()))
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_ansi(false)
                .with_writer(file_rolling)
                .with_filter(env_filter.clone()),
        );

    let controller = match loki {
        Some(loki) => {
            let (layer, controller, task) = tracing_loki::builder()
                .label("source", "host")
                .context("set source label of loki")?
                .build_controller_url(Url::parse(loki).context("parse loki address")?)
                .context("build loki task & layer")?;
            registry
                .with(Filtered::new(layer, env_filter))
                .try_init()
                .context("setup tracing output with loki")?;
            spawn(task);
            Some(controller)
        }
        None => {
            registry.try_init().context("setup tracing output")?;
            None
        }
    };

    Ok((guard, controller))
}
