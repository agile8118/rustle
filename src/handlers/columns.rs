use crate::auth::CurrentUser;
use crate::error::{AppError, AppResult};
use crate::handlers::boards::fetch_board_owned;
use crate::models::column::BoardColumn;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateColumnReq {
    #[validate(length(min = 1, max = 80))]
    pub title: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateColumnReq {
    #[validate(length(min = 1, max = 80))]
    pub title: Option<String>,
    pub position: Option<i32>,
}

pub async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(board_id): Path<Uuid>,
    Json(req): Json<CreateColumnReq>,
) -> AppResult<Json<BoardColumn>> {
    req.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    fetch_board_owned(&state.pool, board_id, user.id).await?;
    let next_pos: Option<i32> = sqlx::query_scalar(
        "SELECT COALESCE(MAX(position) + 1, 0)::int FROM board_columns WHERE board_id = $1",
    )
    .bind(board_id)
    .fetch_one(&state.pool)
    .await?;
    let col = sqlx::query_as!(
        BoardColumn,
        "INSERT INTO board_columns (board_id, title, position) VALUES ($1, $2, $3)
         RETURNING id, board_id, title, position, created_at",
        board_id,
        req.title,
        next_pos.unwrap_or(0)
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(col))
}

pub async fn update(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateColumnReq>,
) -> AppResult<Json<BoardColumn>> {
    req.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let col = fetch_column_owned(&state.pool, id, user.id).await?;
    let title = req.title.unwrap_or(col.title.clone());
    let position = req.position.unwrap_or(col.position);
    let updated = sqlx::query_as!(
        BoardColumn,
        "UPDATE board_columns SET title = $1, position = $2 WHERE id = $3
         RETURNING id, board_id, title, position, created_at",
        title,
        position,
        id
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(updated))
}

pub async fn delete(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    fetch_column_owned(&state.pool, id, user.id).await?;
    sqlx::query!("DELETE FROM board_columns WHERE id = $1", id)
        .execute(&state.pool)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn fetch_column_owned(
    pool: &sqlx::PgPool,
    column_id: Uuid,
    user_id: Uuid,
) -> AppResult<BoardColumn> {
    sqlx::query_as!(
        BoardColumn,
        "SELECT bc.id, bc.board_id, bc.title, bc.position, bc.created_at
         FROM board_columns bc JOIN boards b ON b.id = bc.board_id
         WHERE bc.id = $1 AND b.owner_id = $2",
        column_id,
        user_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}
