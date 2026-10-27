use axum::{
    Router,
    extract::Request,
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};

use super::handlers::{self, AppState};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/init", post(handlers::init))
        .route("/cookies/dump", get(handlers::dump_cookies))
        .route("/cookies/flat", get(handlers::cookies_flat))
        .route("/login", post(handlers::login))
        .route("/schedule", get(handlers::schedule))
        .route("/schedule/current-week", get(handlers::current_week))
        .route("/exam", get(handlers::exam))
        .route("/grade", get(handlers::grade))
        .route("/ycard/balance", get(handlers::balance))
        .route("/ycard/qrcode", get(handlers::qrcode))
        .route("/ycard/refresh_token", post(handlers::refresh_token))
        .with_state(state)
        .layer(middleware::from_fn(diagnostic_generation))
}

async fn diagnostic_generation(request: Request, next: Next) -> Response {
    let generation = request
        .headers()
        .get("X-AHUTong-Diagnostic-Generation")
        .map(|value| value.to_str().unwrap_or("").to_owned());
    crate::diagnostics::scope_request(generation.as_deref(), next.run(request)).await
}
