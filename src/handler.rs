use crate::{
    domain::{CreatedJournal, Entries, Entry, NewJournal},
    service::{CreateError, EntryService},
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
        (status = 401, description = "認証が必要です"),
        (status = 404, description = "仕訳行が存在しません"),
        (status = 500, description = "データベースエラー")
    ),
    security(("bearerAuth" = []))
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

#[utoipa::path(
    post,
    path = "/journals",
    request_body = NewJournal,
    responses(
        (status = 201, body = CreatedJournal),
        (status = 401, description = "認証が必要です"),
        (status = 422, description = "仕訳が貸借一致していないか入力が不正です"),
        (status = 500, description = "データベースエラー")
    ),
    security(("bearerAuth" = []))
)]
pub async fn create_journal<R: Entries>(
    State(service): State<EntryService<R>>,
    Json(journal): Json<NewJournal>,
) -> Result<(StatusCode, Json<CreatedJournal>), StatusCode> {
    match service.create_journal(journal).await {
        Ok(created) => Ok((StatusCode::CREATED, Json(created))),
        Err(CreateError::Invalid) => Err(StatusCode::UNPROCESSABLE_ENTITY),
        Err(CreateError::Database(error)) => {
            tracing::error!(%error, "database write failed");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
