use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use uuid::Uuid;

use crate::client::ApiClient;
use crate::config::Config;

/// Estado compartido entre handlers. Mantenerlo pequeño y detrás de un RwLock
/// es suficiente hasta que enchufemos una cola persistente (Redis / Postgres).
#[derive(Clone)]
pub struct AppState {
    pub cfg: Config,
    pub jobs: Arc<RwLock<HashMap<Uuid, JobRecord>>>,
    pub api: ApiClient,
}

impl AppState {
    pub fn new(cfg: Config) -> Self {
        let api = ApiClient::new(&cfg.api_url);
        Self {
            cfg,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            api,
        }
    }
}

#[derive(Clone, Debug)]
pub struct JobRecord {
    pub id: Uuid,
    pub kind: String,
    pub media_id: String,
    pub status: JobStatus,
    pub notes: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Running,
    Done,
    Failed,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobStatus::Queued => "queued",
            JobStatus::Running => "running",
            JobStatus::Done => "done",
            JobStatus::Failed => "failed",
        }
    }
}