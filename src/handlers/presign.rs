//! Endpoints de storage. La API los llama para generar URLs presigned
//! que devuelve al SPA, y para listar el contenido del bucket.

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct PresignRequest {
    pub key: String,
    #[serde(default = "default_ttl")]
    pub ttl_secs: u64,
}

fn default_ttl() -> u64 {
    3600
}

#[derive(Debug, Serialize)]
pub struct PresignResponse {
    pub url: String,
    pub expires_in: u64,
}

pub async fn presign(
    State(state): State<AppState>,
    Json(req): Json<PresignRequest>,
) -> Result<Json<PresignResponse>, AppError> {
    let url = state
        .storage
        .presigned_get(&req.key, req.ttl_secs)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(PresignResponse {
        url,
        expires_in: req.ttl_secs,
    }))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub prefix: String,
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    100
}

pub async fn list(
    State(state): State<AppState>,
    Query(q): Query<ListQuery>,
) -> Result<impl IntoResponse, AppError> {
    let items = state
        .storage
        .list(&q.prefix, q.limit)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(items))
}