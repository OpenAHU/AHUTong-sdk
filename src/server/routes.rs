use axum::{routing::{get, post}, Router};

use super::handlers::{self, AppState};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/init", post(handlers::init))
        .route("/cookies/dump", get(handlers::dump_cookies))
        .route("/cookies/flat", get(handlers::cookies_flat))
        .route("/login", post(handlers::login))
        .route("/schedule", get(handlers::schedule))
        .route("/exam", get(handlers::exam))
        .route("/grade", get(handlers::grade))
        .route("/ycard/balance", get(handlers::balance))
        .route("/ycard/qrcode", get(handlers::qrcode))
        .route("/ycard/refresh_token", post(handlers::refresh_token))
        .with_state(state)
}
