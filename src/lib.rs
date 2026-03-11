use axum::{Json, Router, routing::get};
use serde::Serialize;

pub fn get_router() -> Router {
    Router::new().route("/health", get(health))
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}
