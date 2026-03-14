use axum::{Json, Router, routing::get};
use serde::Serialize;
use tower_http::cors::CorsLayer;

use crate::router::documents::documents_router;
use state::Conns;

mod documents;
pub mod state;

pub fn router() -> Router<Conns> {
    Router::new()
        .route("/health", get(health))
        .nest("/documents", documents_router())
        .layer(CorsLayer::permissive())
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}
