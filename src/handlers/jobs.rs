use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::jobs;
use crate::state::{AppState, JobRecord, JobStatus};

#[derive(Debug, Deserialize)]
pub struct SubmitRequest {
    pub kind: String,
    pub media_id: String,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SubmitResponse {
    pub job_id: String,
    pub status: String,
}

pub async fn submit(
    State(state): State<AppState>,
    Json(req): Json<SubmitRequest>,
) -> Result<impl IntoResponse, AppError> {
    if req.kind.is_empty() || req.media_id.is_empty() {
        return Err(AppError::BadRequest("kind and media_id required".into()));
    }

    let id = Uuid::new_v4();
    let record = JobRecord {
        id,
        kind: req.kind.clone(),
        media_id: req.media_id.clone(),
        status: JobStatus::Queued,
        notes: req.notes.clone(),
        created_at: chrono::Utc::now(),
    };

    state.jobs.write().await.insert(id, record.clone());

    // Despachamos el job. El ejecutor real corre en background; el handler
    // responde inmediatamente con estado `queued`.
    let job_state = state.clone();
    let job_record = record.clone();
    tokio::spawn(async move {
        jobs::dispatch(job_state.clone(), job_record).await;
    });

    Ok(Json(SubmitResponse {
        job_id: id.to_string(),
        status: "queued".into(),
    }))
}

#[derive(Debug, Serialize)]
pub struct JobView {
    pub job_id: String,
    pub kind: String,
    pub media_id: String,
    pub status: String,
}

pub async fn status(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<JobView>, AppError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| AppError::BadRequest("invalid uuid".into()))?;
    let jobs = state.jobs.read().await;
    let rec = jobs
        .get(&uuid)
        .ok_or_else(|| AppError::NotFound(format!("job {id}")))?;
    Ok(Json(JobView {
        job_id: rec.id.to_string(),
        kind: rec.kind.clone(),
        media_id: rec.media_id.clone(),
        status: rec.status.as_str().to_string(),
    }))
}