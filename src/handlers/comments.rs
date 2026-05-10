use crate::auth::CurrentUser;
use crate::error::{AppError, AppResult};
use crate::handlers::cards::fetch_card_owned;
use crate::models::comment::Comment;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCommentReq {
    #[validate(length(min = 1, max = 5000))]
    pub body: String,
}

pub async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(card_id): Path<Uuid>,
) -> AppResult<Json<Vec<Comment>>> {
    fetch_card_owned(&state.pool, card_id, user.id).await?;
    let rows = sqlx::query_as!(
        Comment,
        "SELECT id, card_id, author_id, body, created_at FROM comments
         WHERE card_id = $1 ORDER BY created_at ASC",
        card_id
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

pub async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(card_id): Path<Uuid>,
    Json(req): Json<CreateCommentReq>,
) -> AppResult<Json<Comment>> {
    req.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    fetch_card_owned(&state.pool, card_id, user.id).await?;
    let row = sqlx::query_as!(
        Comment,
        "INSERT INTO comments (card_id, author_id, body) VALUES ($1, $2, $3)
         RETURNING id, card_id, author_id, body, created_at",
        card_id,
        user.id,
        req.body
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(row))
}

pub async fn delete(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let res = sqlx::query!(
        "DELETE FROM comments WHERE id = $1 AND author_id = $2",
        id,
        user.id
    )
    .execute(&state.pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}
