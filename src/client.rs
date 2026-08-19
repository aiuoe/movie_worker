use std::time::Duration;

use reqwest::Client;

/// Cliente HTTP al API. Lo usamos para callbacks cuando un job termina —
/// en producción este sería un canal autenticado.
#[derive(Clone)]
pub struct ApiClient {
    http: Client,
    base_url: String,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .expect("reqwest client");
        Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// Avisa al API que un job cambió de estado. Fire-and-forget — si el API
    /// está caído lo logueamos y seguimos.
    pub async fn notify_job_done(&self, job_id: &str, status: &str) {
        let url = format!("{}/internal/jobs/{}", self.base_url, job_id);
        let body = serde_json::json!({ "status": status });
        match self.http.patch(&url).json(&body).send().await {
            Ok(_) => tracing::debug!(job_id, "notified api"),
            Err(e) => tracing::warn!(job_id, error=%e, "api unreachable, dropping notification"),
        }
    }
}