use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use uuid::Uuid;

use crate::arr::{radarr::RadarrClient, sonarr::SonarrClient};
use crate::client::ApiClient;
use crate::config::Config;
use crate::storage::SharedStorage;

/// Estado compartido entre handlers. Mantenerlo pequeño y detrás de un RwLock
/// es suficiente hasta que enchufemos una cola persistente (Redis / Postgres).
#[derive(Clone)]
pub struct AppState {
    pub cfg: Config,
    pub jobs: Arc<RwLock<HashMap<Uuid, JobRecord>>>,
    pub api: ApiClient,
    pub storage: SharedStorage,
    pub radarr: Option<RadarrClient>,
    pub sonarr: Option<SonarrClient>,
}

impl AppState {
    pub fn new(cfg: Config, storage: SharedStorage) -> Self {
        let api = ApiClient::new(&cfg.api_url);
        let radarr = if !cfg.radarr_api_key.is_empty() {
            Some(RadarrClient::new(&cfg.radarr_url, &cfg.radarr_api_key))
        } else {
            None
        };
        let sonarr = if !cfg.sonarr_api_key.is_empty() {
            Some(SonarrClient::new(&cfg.sonarr_url, &cfg.sonarr_api_key))
        } else {
            None
        };
        Self {
            cfg,
            jobs: Arc::new(RwLock::new(HashMap::new())),
            api,
            storage,
            radarr,
            sonarr,
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

impl JobRecord {
    pub fn new(id: Uuid, kind: &str, media_id: &str) -> Self {
        Self {
            id,
            kind: kind.into(),
            media_id: media_id.into(),
            status: JobStatus::Queued,
            notes: None,
            created_at: chrono::Utc::now(),
        }
    }
}

/// Versión "light" del JobRecord — útil para inserts rápidos (ej. webhook).
#[derive(Clone, Debug)]
pub struct JobRecordStub {
    pub id: Uuid,
    pub kind: String,
    pub media_id: String,
    pub notes: Option<String>,
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