use {
    crate::{run_http_host::RunHttpHostArgs, setup_tracing::SetupTracingOpts},
    clap::Parser,
    tracing::{error, info},
};

mod layers;
mod modules;
mod never_error;
mod run_http_host;
mod setup_tracing;
mod states;
#[cfg(test)]
mod test_helper;

#[derive(Parser)]
struct Opts {
    #[clap(flatten)]
    tracing: SetupTracingOpts,
    #[clap(flatten)]
    run_http_host: RunHttpHostArgs,
}

#[tokio::main]
async fn main() {
    let Opts {
        tracing,
        run_http_host,
        ..
    } = Opts::parse();

    let _tracing_guard = tracing.setup();
    info!("app starting up");

    match run_http_host.run().await {
        Ok(_) => {
            info!("http server exit normally");
        }
        Err(err) => {
            error!("http server exit with error {:?}", err);
        }
    }
}
