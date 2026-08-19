use axum::{routing::{get, post}, Router};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::handlers;
use crate::state::AppState;

/// Router raíz del worker. Cada ruta se documenta en `handlers/*`.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(handlers::health::healthz))
        .route("/jobs", post(handlers::jobs::submit))
        .route("/jobs/:id", get(handlers::jobs::status))
        .route("/streams/notify", post(handlers::streams::notify))
        .route("/webhooks/radarr", post(handlers::webhook::import))
        .route("/webhooks/sonarr", post(handlers::webhook::import))
        .route("/storage/presign", post(handlers::presign::presign))
        .route("/storage/list", get(handlers::presign::list))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}