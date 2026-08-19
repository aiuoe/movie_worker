//! Wrapper específico para Radarr (movies).

use anyhow::Result;
use serde_json::{json, Value};

use super::ArrClient;

#[derive(Clone)]
pub struct RadarrClient {
    pub inner: ArrClient,
}

impl RadarrClient {
    pub fn new(base_url: &str, api_key: &str) -> Self {
        Self { inner: ArrClient::new(base_url, api_key) }
    }

    /// Busca una película por TMDB id y la agrega a Radarr para monitorear.
    /// Devuelve el id interno de Radarr.
    pub async fn add_movie_by_tmdb(&self, tmdb_id: u32, title: &str) -> Result<u64> {
        // 1. lookup por tmdb
        let lookup: Value = self.inner.get_json(&format!("/api/v3/movie/lookup/tmdb?tmdbId={tmdb_id}")).await?;
        let tmdb = lookup.get("tmdbId").and_then(|v| v.as_u64()).unwrap_or(tmdb_id as u64);

        // 2. agregar (rootFolder=1, qualityProfileId=1 son defaults típicos)
        let body = json!({
            "title": title,
            "qualityProfileId": 1,
            "tmdbId": tmdb,
            "year": lookup.get("year").and_then(|v| v.as_u64()).unwrap_or(0),
            "rootFolderPath": "/movies",
            "monitored": true,
            "minimumAvailability": "released",
            "addOptions": { "searchForMovie": true }
        });
        let out: Value = self.inner.post_json("/api/v3/movie", body).await?;
        Ok(out.get("id").and_then(|v| v.as_u64()).unwrap_or(0))
    }

    pub async fn lookup_tmdb(&self, term: &str) -> Result<Value> {
        // Radarr soporta lookup por término también, pero devuelve más resultados.
        let path = format!("/api/v3/movie/lookup?term={}", urlencoded(term));
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