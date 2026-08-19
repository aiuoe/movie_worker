//! Wrapper específico para Sonarr (series).

use anyhow::Result;
use serde_json::{json, Value};

use super::ArrClient;

#[derive(Clone)]
pub struct SonarrClient {
    pub inner: ArrClient,
}

impl SonarrClient {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        Self { inner: ArrClient::new(base_url, api_key) }
    }

    /// Busca una serie por TVDB id y la agrega a Sonarr.
    pub async fn add_series_by_tvdb(&self, tvdb_id: u32, title: &str) -> Result<u64> {
        let lookup: Value = self.inner.get_json(&format!("/api/v3/series/lookup?term=tvdb:{tvdb_id}")).await?;
        let tvdb = lookup.get("tvdbId").and_then(|v| v.as_u64()).unwrap_or(tvdb_id as u64);

        let body = json!({
            "title": title,
            "qualityProfileId": 1,
            "tvdbId": tvdb,
            "year": lookup.get("year").and_then(|v| v.as_u64()).unwrap_or(0),
            "rootFolderPath": "/tv",
            "monitored": true,
            "addOptions": { "searchForMissingEpisodes": true }
        });
        let out: Value = self.inner.post_json("/api/v3/series", body).await?;
        Ok(out.get("id").and_then(|v| v.as_u64()).unwrap_or(0))
    }

    pub async fn lookup(&self, term: &str) -> Result<Value> {
        let path = format!("/api/v3/series/lookup?term={}", urlencoded(term));
        Ok(self.inner.get_json(&path).await?)
    }
}

fn urlencoded(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}