//! Webhook receiver — Radarr/Sonarr/etc. nos mandan eventos acá.
//!
//! Configuración típica en Radarr: Settings → Connect → Custom Script
//!   Arguments: --media-id {Movie.TmdbId} --path "{MovieFile.RelativePath}"
//!
//! O alternativamente como Webhook (no soportado nativamente por Radarr, pero
//! podemos agregar uno custom en el futuro).

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

use crate::error::AppError;
use crate::jobs;
use crate::state::{AppState, JobRecord, JobStatus};

#[derive(Debug, Deserialize)]
pub struct ImportQuery {
    pub media_id: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct RadarrWebhook {
    #[serde(default)]
    pub media_id: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub movie: Option<serde_json::Value>,
    #[serde(default)]
    pub movie_file: Option<serde_json::Value>,
    #[serde(default)]
    pub series: Option<serde_json::Value>,
    #[serde(default)]
    pub episode_file: Option<serde_json::Value>,
    /// Radarr manda "eventType": "Download" o "Test"
    #[serde(default)]
    pub event_type: Option<String>,
    /// Radarr manda el id del movie (interno, numérico). Lo usamos para mapear.
    #[serde(default)]
    pub movie_id: Option<u64>,
    /// TMDB id directamente (lo agregamos a la config de Radarr custom script)
    #[serde(default)]
    pub tmdb_id: Option<u64>,
    /// TVDB id para series
    #[serde(default)]
    pub tvdb_id: Option<u64>,
}

/// Endpoint principal: recibe un webhook tipo Radarr/Sonarr y dispara el job `import`.
/// Acepta tanto JSON como query params para máxima compatibilidad.
pub async fn import(
    State(state): State<AppState>,
    query: Query<ImportQuery>,
    body: Option<Json<RadarrWebhook>>,
) -> Result<impl IntoResponse, AppError> {
    let (media_id, path) = extract_fields(query.0, body.map(|Json(b)| b));

    let media_id = media_id.ok_or_else(|| AppError::BadRequest("missing media_id/tmdbId/tvdbId".into()))?;
    let path = path.ok_or_else(|| AppError::BadRequest("missing path/movieFile.path".into()))?;

    let mut args: HashMap<String, String> = HashMap::new();
    args.insert("media_id".into(), media_id.clone());
    args.insert("path".into(), path.clone());
    args.insert("lang".into(), "es-ES".into());

    let id = Uuid::new_v4();
    let rec = JobRecord {
        id,
        kind: "import".into(),
        media_id: media_id.clone(),
        status: JobStatus::Queued,
        notes: Some(serde_json::to_string(&args).unwrap_or_default()),
        created_at: Utc::now(),
    };

    let st = state.clone();
    tokio::spawn(async move {
        jobs::dispatch(st, rec).await;
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "job_id": id.to_string(),
            "media_id": media_id,
            "path": path,
            "status": "queued"
        })),
    ))
}

fn extract_fields(
    query: ImportQuery,
    body: Option<RadarrWebhook>,
) -> (Option<String>, Option<String>) {
    if let Some(b) = body {
        // Prioridad: TMDB id / TVDB id (los que usa nuestro seed del API)
        // → media_id con prefijo de tipo para que el job lo use correctamente
        let mid = b
            .media_id
            .clone()
            .or_else(|| b.tmdb_id.map(|n| format!("tmdb:{}", n)))
            .or_else(|| b.tvdb_id.map(|n| format!("tvdb:{}", n)))
            .or_else(|| {
                b.movie
                    .as_ref()
                    .and_then(|m| m.get("tmdbId"))
                    .and_then(|v| v.as_u64())
                    .map(|n| format!("tmdb:{}", n))
            })
            .or_else(|| {
                b.series
                    .as_ref()
                    .and_then(|s| s.get("tvdbId"))
                    .and_then(|v| v.as_u64())
                    .map(|n| format!("tvdb:{}", n))
            });

        let p = b.path.clone().or_else(|| {
            b.movie_file
                .as_ref()
                .and_then(|f| f.get("path"))
                .and_then(|v| v.as_str())
                .map(String::from)
        })
        .or_else(|| {
            b.episode_file
                .as_ref()
                .and_then(|f| f.get("path"))
                .and_then(|v| v.as_str())
                .map(String::from)
        });
        (mid, p)
    } else {
        (query.media_id, query.path)
    }
}