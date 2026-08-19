use std::env;

use anyhow::{anyhow, Result};

/// Config runtime. Todas tienen defaults sensatos — el binario funciona sin
/// env vars, pero `MEDIA_ROOT` y `API_URL` van a ser los reales en compose.
#[derive(Clone, Debug)]
pub struct Config {
    pub addr: String,
    pub api_url: String,
    pub media_root: String,
    pub radarr_url: String,
    pub radarr_api_key: String,
    pub sonarr_url: String,
    pub sonarr_api_key: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            addr: env_or("ADDR", "0.0.0.0:9090"),
            api_url: env_or("API_URL", "http://localhost:8080"),
            media_root: env_or("MEDIA_ROOT", "/media"),
            radarr_url: env_or("RADARR_URL", "http://localhost:7878"),
            radarr_api_key: env_or("RADARR_API_KEY", ""),
            sonarr_url: env_or("SONARR_URL", "http://localhost:8989"),
            sonarr_api_key: env_or("SONARR_API_KEY", ""),
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.addr.is_empty() {
            return Err(anyhow!("ADDR empty"));
        }
        Ok(())
    }
}

fn env_or(key: &str, fallback: &str) -> String {
    env::var(key).unwrap_or_else(|_| fallback.to_string())
}