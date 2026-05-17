use crate::auth::CurrentUser;
use crate::error::{AppError, AppResult};
use crate::handlers::columns::fetch_column_owned;
use crate::models::card::Card;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCardReq {
    #[validate(length(min = 1, max = 200))]
    pub title: String,
    pub description: Option<String>,
    pub due_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateCardReq {
    #[validate(length(min = 1, max = 200))]
    pub title: Option<String>,
    pub description: Option<String>,
    pub due_at: Option<Option<DateTime<Utc>>>,
}

#[derive(Debug, Deserialize)]
pub struct MoveCardReq {
    pub column_id: Uuid,
    pub position: i32,
}

pub async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(column_id): Path<Uuid>,
    Json(req): Json<CreateCardReq>,
) -> AppResult<Json<Card>> {
    req.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    fetch_column_owned(&state.pool, column_id, user.id).await?;
    let next_pos: Option<i32> = sqlx::query_scalar(
        "SELECT COALESCE(MAX(position) + 1, 0)::int FROM cards WHERE column_id = $1",
    )
    .bind(column_id)
    .fetch_one(&state.pool)
    .await?;
    let card = sqlx::query_as!(
        Card,
        "INSERT INTO cards (column_id, title, description, position, due_at) VALUES ($1, $2, $3, $4, $5)
         RETURNING id, column_id, title, description, position, due_at, created_at",
        column_id,
        req.title,
        req.description.unwrap_or_default(),
        next_pos.unwrap_or(0),
        req.due_at
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(card))
}

pub async fn get(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Card>> {
    Ok(Json(fetch_card_owned(&state.pool, id, user.id).await?))
}

pub async fn update(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateCardReq>,
) -> AppResult<Json<Card>> {
    req.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let card = fetch_card_owned(&state.pool, id, user.id).await?;
    let title = req.title.unwrap_or(card.title.clone());
    let description = req.description.unwrap_or(card.description.clone());
    let due_at = match req.due_at {
        Some(v) => v,
        None => card.due_at,
    };
    let updated = sqlx::query_as!(
        Card,
        "UPDATE cards SET title = $1, description = $2, due_at = $3 WHERE id = $4
         RETURNING id, column_id, title, description, position, due_at, created_at",
        title,
        description,
        due_at,
        id
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(updated))
}

pub async fn move_card(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<MoveCardReq>,
) -> AppResult<Json<Card>> {
    let card = fetch_card_owned(&state.pool, id, user.id).await?;
    fetch_column_owned(&state.pool, req.column_id, user.id).await?;

    let mut tx = state.pool.begin().await?;

    // Remove from source column ordering (close the gap)
    sqlx::query!(
        "UPDATE cards SET position = position - 1
         WHERE column_id = $1 AND position > $2",
        card.column_id,
        card.position
    )
    .execute(&mut *tx)
    .await?;

    // Make space in target column
    sqlx::query!(
        "UPDATE cards SET position = position + 1
         WHERE column_id = $1 AND position >= $2 AND id <> $3",
        req.column_id,
        req.position,
        id
    )
    .execute(&mut *tx)
    .await?;

    let updated = sqlx::query_as!(
        Card,
        "UPDATE cards SET column_id = $1, position = $2 WHERE id = $3
         RETURNING id, column_id, title, description, position, due_at, created_at",
        req.column_id,
        req.position,
        id
    )
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Json(updated))
}

pub async fn delete(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    let card = fetch_card_owned(&state.pool, id, user.id).await?;
    let mut tx = state.pool.begin().await?;
    sqlx::query!("DELETE FROM cards WHERE id = $1", id)
        .execute(&mut *tx)
        .await?;
    sqlx::query!(
        "UPDATE cards SET position = position - 1
         WHERE column_id = $1 AND position > $2",
        card.column_id,
        card.position
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn fetch_card_owned(
    pool: &sqlx::PgPool,
    card_id: Uuid,
    user_id: Uuid,
) -> AppResult<Card> {
    sqlx::query_as!(
        Card,
        "SELECT c.id, c.column_id, c.title, c.description, c.position, c.due_at, c.created_at
         FROM cards c
         JOIN board_columns bc ON bc.id = c.column_id
         JOIN boards b ON b.id = bc.board_id
         WHERE c.id = $1 AND b.owner_id = $2",
        card_id,
        user_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}
