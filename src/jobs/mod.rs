pub mod probe;
pub mod transcode;
pub mod scrape;
pub mod ingest;

use crate::state::{AppState, JobRecord, JobStatus};

/// Despacha un job según `kind`. Cada rama marca su propio progreso en
/// `AppState::jobs` y notifica al API al terminar.
pub async fn dispatch(state: AppState, rec: JobRecord) {
    let id = rec.id;
    let media_id = rec.media_id.clone();
    let kind = rec.kind.clone();

    set_status(&state, id, JobStatus::Running).await;
    let result = match kind.as_str() {
        "probe" => probe::run(&state, &media_id).await,
        "transcode" => transcode::run(&state, &media_id).await,
        "scrape" => scrape::run(&state, &media_id).await,
        "ingest" => ingest::run(&state).await,
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

    state.api.notify_job_done(&id.to_string(), final_status).await;
}

async fn set_status(state: &AppState, id: uuid::Uuid, status: JobStatus) {
    let mut jobs = state.jobs.write().await;
    if let Some(rec) = jobs.get_mut(&id) {
        rec.status = status;
    }
}