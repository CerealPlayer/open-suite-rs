use axum::{Json, Router, routing::get};
use serde::Serialize;

use crate::config::Conns;

pub mod config;
pub mod storage;

pub fn get_router() -> Router<Conns> {
    Router::new().route("/health", get(health))
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}
