use {
    crate::modules::AppRouter,
    anyhow::Result,
    axum::{
        debug_handler, extract::Query, http::StatusCode, response::IntoResponse, routing::get,
        Router,
    },
    duration_str::deserialize_duration,
    serde::Deserialize,
    std::time::Duration,
    tokio::time::sleep,
    tracing::info,
};

pub fn router() -> AppRouter {
    Router::new()
        .route("/ping", get(ping))
        .route("/delay", get(delay))
        .route("/forbidden", get(forbidden))
        .route("/error", get(internal_error))
}

#[debug_handler]
async fn ping() -> impl IntoResponse {
    info!("ping");
    "echo!"
}

#[derive(Deserialize)]
struct DelayQuery {
    #[serde(deserialize_with = "deserialize_duration")]
    duration: Duration,
}

#[debug_handler]
async fn delay(Query(query): Query<DelayQuery>) -> impl IntoResponse {
    info!("delay {}ms", query.duration.as_millis());
    sleep(query.duration).await;
    info!("delay pass");
    "done!"
}

#[debug_handler]
async fn forbidden() -> impl IntoResponse {
    info!("forbidden");
    (StatusCode::FORBIDDEN, "forbidden!")
}

#[debug_handler]
async fn internal_error() -> Result<&'static str, &'static str> {
    info!("internal error");
    Err("failed")
}
