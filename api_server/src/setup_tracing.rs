use {
    clap::Args,
    std::borrow::Cow,
    tracing::level_filters::LevelFilter,
    tracing_appender::{non_blocking::WorkerGuard, rolling::Rotation},
    tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer},
};

#[derive(Args)]
pub struct SetupTracingOpts {
    #[clap(long = "tracing", default_value = "debug", help = "tracing env filter")]
    tracing: Cow<'static, str>,
    #[clap(
        long = "tracing-output-dir",
        default_value = "./log",
        help = "tracing env filter"
    )]
    tracing_output_dir: Cow<'static, str>,
}

impl SetupTracingOpts {
    pub fn setup(&self) -> WorkerGuard {
        let file_rolling = tracing_appender::rolling::Builder::new()
            .filename_prefix("api_server")
            .filename_suffix("log")
            .rotation(Rotation::HOURLY)
            .max_log_files(24 * 7) // week
            .build(self.tracing_output_dir.as_ref())
            .expect("create tracing file rolling output");

        let (file_rolling, guard) = tracing_appender::non_blocking(file_rolling);

        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer().with_filter(
                    EnvFilter::builder()
                        .with_default_directive(LevelFilter::DEBUG.into())
                        .parse_lossy(self.tracing.as_ref()),
                ),
            )
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_ansi(false)
                    .with_writer(file_rolling)
                    .with_filter(
                        EnvFilter::builder()
                            .with_default_directive(LevelFilter::DEBUG.into())
                            .parse_lossy(self.tracing.as_ref()),
                    ),
            )
            .try_init()
            .expect("setup tracing output");

        guard
    }
}
