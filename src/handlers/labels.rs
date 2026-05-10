use crate::auth::CurrentUser;
use crate::error::{AppError, AppResult};
use crate::handlers::cards::fetch_card_owned;
use crate::models::label::Label;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateLabelReq {
    #[validate(length(min = 1, max = 40))]
    pub name: String,
    #[validate(length(min = 4, max = 9))]
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AttachLabelReq {
    pub label_id: Uuid,
}

pub async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<Vec<Label>>> {
    let rows = sqlx::query_as!(
        Label,
        "SELECT id, owner_id, name, color FROM labels WHERE owner_id = $1 ORDER BY name ASC",
        user.id
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

pub async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(req): Json<CreateLabelReq>,
) -> AppResult<Json<Label>> {
    req.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let color = req.color.unwrap_or_else(|| "#888888".into());
    let row = sqlx::query_as!(
        Label,
        "INSERT INTO labels (owner_id, name, color) VALUES ($1, $2, $3)
         RETURNING id, owner_id, name, color",
        user.id,
        req.name,
        color
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(row))
}

pub async fn attach(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(card_id): Path<Uuid>,
    Json(req): Json<AttachLabelReq>,
) -> AppResult<Json<serde_json::Value>> {
    fetch_card_owned(&state.pool, card_id, user.id).await?;
    let owns: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM labels WHERE id = $1 AND owner_id = $2")
            .bind(req.label_id)
            .bind(user.id)
            .fetch_optional(&state.pool)
            .await?;
    if owns.is_none() {
        return Err(AppError::NotFound);
    }
    sqlx::query!(
        "INSERT INTO card_labels (card_id, label_id) VALUES ($1, $2)
         ON CONFLICT DO NOTHING",
        card_id,
        req.label_id
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn detach(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path((card_id, label_id)): Path<(Uuid, Uuid)>,
) -> AppResult<Json<serde_json::Value>> {
    fetch_card_owned(&state.pool, card_id, user.id).await?;
    sqlx::query!(
        "DELETE FROM card_labels WHERE card_id = $1 AND label_id = $2",
        card_id,
        label_id
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
