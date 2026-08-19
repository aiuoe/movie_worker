use std::env;

use anyhow::{anyhow, Result};

/// Config runtime. Todas tienen defaults sensatos — el binario funciona sin
/// env vars, pero `MEDIA_ROOT` y `API_URL` van a ser los reales en compose.
#[derive(Clone, Debug)]
pub struct Config {
    pub addr: String,
    pub api_url: String,
    pub media_root: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            addr: env_or("ADDR", "0.0.0.0:9090"),
            api_url: env_or("API_URL", "http://localhost:8080"),
            media_root: env_or("MEDIA_ROOT", "/media"),
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