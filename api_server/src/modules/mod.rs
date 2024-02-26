use {crate::states::AppState, axum::Router};

#[cfg(debug_assertions)]
mod dev_api;

pub type AppRouter = Router<AppState>;

pub fn router() -> AppRouter {
    let router = Router::new();
    #[cfg(debug_assertions)]
    let router = router.nest("/dev", dev_api::router());
    router
}
