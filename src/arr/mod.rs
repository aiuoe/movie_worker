//! Cliente para Radarr / Sonarr. El worker habla con ellos para:
//!   - Registrar títulos nuevos
//!   - Listar títulos disponibles
//!   - Buscar en TMDB (proxy)
//!   - Recibir webhooks de "import complete" → subir a MinIO

pub mod radarr;
pub mod sonarr;

use anyhow::Result;
use reqwest::Client;
use std::time::Duration;

#[derive(Clone)]
pub struct ArrClient {
    pub http: Client,
    pub base_url: String,
    pub api_key: String,
}

impl ArrClient {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .expect("reqwest");
        Self {
            http,
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
        }
    }

    pub async fn ping(&self) -> Result<()> {
        let url = format!("{}/api/v3/system/status", self.base_url);
        let r = self.http.get(&url).header("X-Api-Key", &self.api_key).send().await?;
        if r.status().is_success() { Ok(()) } else { Err(anyhow::anyhow!("status {}", r.status())) }
    }

    pub async fn get_json(&self, path: &str) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let r = self.http.get(&url).header("X-Api-Key", &self.api_key).send().await?;
        if !r.status().is_success() {
            return Err(anyhow::anyhow!("GET {path} -> {}", r.status()));
        }
        Ok(r.json().await?)
    }

    pub async fn post_json(&self, path: &str, body: serde_json::Value) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let r = self
            .http
            .post(&url)
            .header("X-Api-Key", &self.api_key)
            .json(&body)
            .send()
            .await?;
        if !r.status().is_success() {
            let s = r.status();
            let body = r.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("POST {path} -> {s}: {body}"));
        }
        Ok(r.json().await?)
    }
}