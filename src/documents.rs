use axum::{
    Json, Router,
    extract::{Multipart, State},
    http::StatusCode,
    routing::{get, post},
};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::Serialize;
use std::path::Path;
use uuid::Uuid;

use crate::{config::Conns, entities::document, storage::upload_bytes};

pub fn router() -> Router<Conns> {
    Router::new()
        .route("/", get(list_documents))
        .route("/upload", post(upload))
}

#[derive(Serialize)]
struct UploadDocumentResponse {
    id: Uuid,
    path: String,
}

#[derive(Serialize)]
struct ListDocumentsResponse {
    documents: Vec<document::Model>,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

async fn list_documents(
    State(conns): State<Conns>,
) -> Result<Json<ListDocumentsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let documents = document::Entity::find()
        .all(&conns.db)
        .await
        .map_err(|err| internal_error(err.to_string()))?;

    Ok(Json(ListDocumentsResponse { documents }))
}

async fn upload(
    State(conns): State<Conns>,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<UploadDocumentResponse>), (StatusCode, Json<ErrorResponse>)> {
    let mut file_name: Option<String> = None;
    let mut file_bytes: Option<Vec<u8>> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|err| bad_request(err.to_string()))?
    {
        if let Some(name) = field.file_name() {
            file_name = Some(name.to_owned());
            file_bytes = Some(
                field
                    .bytes()
                    .await
                    .map_err(|err| internal_error(err.to_string()))?
                    .to_vec(),
            );
            break;
        }
    }

    let file_name =
        file_name.ok_or_else(|| bad_request("multipart payload must include a file"))?;
    let file_bytes =
        file_bytes.ok_or_else(|| bad_request("multipart payload must include a file"))?;

    let object_id = Uuid::new_v4();
    let extension = Path::new(&file_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| !ext.is_empty())
        .map(|ext| format!(".{ext}"))
        .unwrap_or_default();
    let object_path = format!("documents/{object_id}{extension}");

    upload_bytes(&conns.bucket, &object_path, &file_bytes)
        .await
        .map_err(|err| internal_error(err.to_string()))?;

    let inserted = document::ActiveModel {
        id: Set(object_id),
        path: Set(object_path.clone()),
        ..Default::default()
    }
    .insert(&conns.db)
    .await
    .map_err(|err| internal_error(err.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(UploadDocumentResponse {
            id: inserted.id,
            path: inserted.path,
        }),
    ))
}

fn bad_request(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}

fn internal_error(message: impl Into<String>) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: message.into(),
        }),
    )
}
