use axum::{response::IntoResponse, Json};
use serde_json::{json, Value};

pub async fn healthz() -> impl IntoResponse {
    Json(json!({ "status": "ok", "service": "movie_worker" })) as Json<Value>
}