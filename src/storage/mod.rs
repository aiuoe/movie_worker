//! Storage provider — el corazón del patrón Provider/Inject.
//!
//! Un único trait `Storage` define las operaciones que el worker necesita.
//! Todas las impls son S3-compatible (mismo protocolo), así que hoy se
//! monta contra MinIO, mañana contra AWS S3 / Cloudflare R2 / Backblaze B2
//! sin cambiar una línea del código que usa el trait — sólo cambia el env.

pub mod s3;

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use serde::{Deserialize, Serialize};

/// Metadata de un objeto en el bucket. Lo que devolvería `mc stat`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObjectInfo {
    pub key: String,
    pub size: u64,
    pub content_type: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<chrono::DateTime<chrono::Utc>>,
}

/// Resolved backend config. Se usa para construir la impl correcta.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Minio,
    Aws,
    R2,
    GenericS3,
}

impl Backend {
    pub fn from_env(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "minio" => Backend::Minio,
            "aws" | "s3" | "aws-s3" => Backend::Aws,
            "r2" | "cloudflare" => Backend::R2,
            _ => Backend::GenericS3,
        }
    }
}

/// El trait. Notar que es `Send + Sync` — podemos envolverlo en `Arc<dyn Storage>`
/// y compartirlo entre handlers y jobs.
#[async_trait]
pub trait Storage: Send + Sync {
    fn backend(&self) -> Backend;
    fn bucket(&self) -> &str;

    /// Sube un blob. `key` es el path dentro del bucket
    /// (ej: "series/got/s01/e01/es-ES/source.mp4").
    async fn put(&self, key: &str, data: Bytes, content_type: Option<&str>) -> Result<()>;

    /// Lee el blob entero (ok para archivos chicos; para video usar `stream`).
    async fn get(&self, key: &str) -> Result<Bytes>;

    /// Lista objetos con un prefijo. Devuelve hasta `limit` resultados.
    async fn list(&self, prefix: &str, limit: usize) -> Result<Vec<ObjectInfo>>;

    /// Genera una URL presigned (GET) válida por `ttl_secs` segundos.
    /// El SPA usa esta URL directo en el tag `<video>` — sin proxy por el API.
    async fn presigned_get(&self, key: &str, ttl_secs: u64) -> Result<String>;

    /// Verifica que el bucket existe y responde.
    async fn ping(&self) -> Result<()>;
}

/// Trait helper para logging/debug.
pub trait StorageExt {
    fn back_backend_str(&self) -> &'static str;
}

impl<T: Storage + ?Sized> StorageExt for T {
    fn back_backend_str(&self) -> &'static str {
        match self.backend() {
            Backend::Minio => "minio",
            Backend::Aws => "aws-s3",
            Backend::R2 => "cloudflare-r2",
            Backend::GenericS3 => "s3-generic",
        }
    }
}

/// Alias público para inyectar.
pub type SharedStorage = Arc<dyn Storage>;

/// Factory: lee el env y devuelve la impl correcta envuelta en Arc.
pub fn build_from_env() -> Result<SharedStorage> {
    let endpoint = std::env::var("S3_ENDPOINT").ok();
    let public_endpoint = std::env::var("S3_PUBLIC_ENDPOINT").ok();
    let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "media".into());
    let region = std::env::var("S3_REGION").unwrap_or_else(|_| "us-east-1".into());
    let access_key = std::env::var("S3_ACCESS_KEY")?;
    let secret_key = std::env::var("S3_SECRET_KEY")?;
    let backend = Backend::from_env(
        std::env::var("STORAGE_BACKEND")
            .unwrap_or_else(|_| "minio".into())
            .as_str(),
    );
    let force_path_style = std::env::var("S3_FORCE_PATH_STYLE")
        .map(|v| v == "true" || v == "1")
        .unwrap_or(true);

    let storage = s3::S3Storage::new(
        backend,
        endpoint,
        public_endpoint,
        bucket,
        region,
        access_key,
        secret_key,
        force_path_style,
    )?;
    Ok(Arc::new(storage))
}