use axum::{extract::State, response::IntoResponse, Json};
use serde::Deserialize;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct NotifyRequest {
    pub media_id: String,
}

/// Notificación desde movie_api de que hay un nuevo id listo para catalogar.
/// Hoy solo logueamos — mañana encola un job `ingest` automático.
pub async fn notify(
    State(state): State<AppState>,
    Json(req): Json<NotifyRequest>,
) -> Result<impl IntoResponse, AppError> {
    if req.media_id.is_empty() {
        return Err(AppError::BadRequest("media_id required".into()));
    }
    tracing::info!(media_id = %req.media_id, media_root = %state.cfg.media_root, "stream notified");
    Ok(Json(serde_json::json!({ "received": true })))
}