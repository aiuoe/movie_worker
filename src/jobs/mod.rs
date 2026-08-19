pub mod download;
pub mod probe;
pub mod scrape;
pub mod transcode;
pub mod upload;
pub mod ingest;

use std::collections::HashMap;

use crate::state::{AppState, JobRecord, JobStatus};

/// Argsparseamos las notas del job como JSON. Si falla, usamos vacío.
fn parse_args(notes: &Option<String>) -> HashMap<String, String> {
    notes
        .as_deref()
        .and_then(|n| serde_json::from_str::<HashMap<String, String>>(n).ok())
        .unwrap_or_default()
}

/// Despacha un job según `kind`. Cada rama marca su propio progreso en
/// `AppState::jobs` y notifica al API al terminar.
pub async fn dispatch(state: AppState, rec: JobRecord) {
    let id = rec.id;
    let media_id = rec.media_id.clone();
    let kind = rec.kind.clone();
    let args = parse_args(&rec.notes);

    set_status(&state, id, JobStatus::Running).await;
    let result = match kind.as_str() {
        "probe" => probe::run(&state, &args).await,
        "transcode" => transcode::run(&state, &args).await,
        "scrape" => scrape::run(&state, &args).await,
        "ingest" => ingest::run(&state, &args).await,
        "download" => download::run(&state, &args).await,
        "upload" => upload::run(&state, &args).await,
        other => Err(format!("unknown job kind: {other}").into()),
    };

    let final_status = match result {
        Ok(_) => {
            set_status(&state, id, JobStatus::Done).await;
            "done"
        }
        Err(e) => {
            tracing::warn!(job_id = %id, kind = %kind, error = %e, "job failed");
            set_status(&state, id, JobStatus::Failed).await;
            "failed"
        }
    };

    tracing::info!(job_id = %id, kind = %kind, media_id = %media_id, status = final_status, "job complete");
    state.api.notify_job_done(&id.to_string(), final_status).await;
}

async fn set_status(state: &AppState, id: uuid::Uuid, status: JobStatus) {
    let mut jobs = state.jobs.write().await;
    if let Some(rec) = jobs.get_mut(&id) {
        rec.status = status;
    }
}