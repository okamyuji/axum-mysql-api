use crate::{
    repository::{Entries, Entry},
    service::EntryService,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

#[utoipa::path(
    get,
    path = "/entries/{id}",
    params(("id" = i64, Path, description = "仕訳行ID")),
    responses(
        (status = 200, body = Entry),
        (status = 404, description = "仕訳行が存在しません"),
        (status = 500, description = "データベースエラー")
    )
)]
pub async fn get_entry<R: Entries>(
    State(service): State<EntryService<R>>,
    Path(id): Path<i64>,
) -> Result<Json<Entry>, StatusCode> {
    match service.get_entry(id).await {
        Ok(Some(entry)) => Ok(Json(entry)),
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(error) => {
            tracing::error!(%error, "database query failed");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
