use crate::auth::CurrentUser;
use crate::error::{AppError, AppResult};
use crate::models::{board::Board, card::Card, column::BoardColumn};
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct UpsertBoardReq {
    #[validate(length(min = 1, max = 120))]
    pub title: String,
}

#[derive(Debug, Serialize)]
pub struct ColumnWithCards {
    #[serde(flatten)]
    pub column: BoardColumn,
    pub cards: Vec<Card>,
}

#[derive(Debug, Serialize)]
pub struct BoardDetail {
    #[serde(flatten)]
    pub board: Board,
    pub columns: Vec<ColumnWithCards>,
}

pub async fn list(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Json<Vec<Board>>> {
    let boards = sqlx::query_as!(
        Board,
        "SELECT id, owner_id, title, created_at FROM boards WHERE owner_id = $1 ORDER BY created_at DESC",
        user.id
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(boards))
}

pub async fn create(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(req): Json<UpsertBoardReq>,
) -> AppResult<Json<Board>> {
    req.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    let board = sqlx::query_as!(
        Board,
        "INSERT INTO boards (owner_id, title) VALUES ($1, $2)
         RETURNING id, owner_id, title, created_at",
        user.id,
        req.title
    )
    .fetch_one(&state.pool)
    .await?;

    // Seed three default columns
    for (i, name) in ["To Do", "In Progress", "Done"].iter().enumerate() {
        sqlx::query!(
            "INSERT INTO board_columns (board_id, title, position) VALUES ($1, $2, $3)",
            board.id,
            name,
            i as i32
        )
        .execute(&state.pool)
        .await?;
    }
    Ok(Json(board))
}

pub async fn get(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<BoardDetail>> {
    let board = fetch_board_owned(&state.pool, id, user.id).await?;

    let columns = sqlx::query_as!(
        BoardColumn,
        "SELECT id, board_id, title, position, created_at FROM board_columns
         WHERE board_id = $1 ORDER BY position ASC",
        board.id
    )
    .fetch_all(&state.pool)
    .await?;

    let cards = sqlx::query_as!(
        Card,
        "SELECT c.id, c.column_id, c.title, c.description, c.position, c.due_at, c.created_at
         FROM cards c JOIN board_columns bc ON bc.id = c.column_id
         WHERE bc.board_id = $1 ORDER BY c.column_id, c.position ASC",
        board.id
    )
    .fetch_all(&state.pool)
    .await?;

    let mut by_col: std::collections::HashMap<Uuid, Vec<Card>> = std::collections::HashMap::new();
    for c in cards {
        by_col.entry(c.column_id).or_default().push(c);
    }

    let cols_with_cards = columns
        .into_iter()
        .map(|col| ColumnWithCards {
            cards: by_col.remove(&col.id).unwrap_or_default(),
            column: col,
        })
        .collect();

    Ok(Json(BoardDetail { board, columns: cols_with_cards }))
}

pub async fn update(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
    Json(req): Json<UpsertBoardReq>,
) -> AppResult<Json<Board>> {
    req.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    fetch_board_owned(&state.pool, id, user.id).await?;
    let board = sqlx::query_as!(
        Board,
        "UPDATE boards SET title = $1 WHERE id = $2
         RETURNING id, owner_id, title, created_at",
        req.title,
        id
    )
    .fetch_one(&state.pool)
    .await?;
    Ok(Json(board))
}

pub async fn delete(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<serde_json::Value>> {
    fetch_board_owned(&state.pool, id, user.id).await?;
    sqlx::query!("DELETE FROM boards WHERE id = $1", id)
        .execute(&state.pool)
        .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn fetch_board_owned(
    pool: &sqlx::PgPool,
    board_id: Uuid,
    user_id: Uuid,
) -> AppResult<Board> {
    sqlx::query_as!(
        Board,
        "SELECT id, owner_id, title, created_at FROM boards WHERE id = $1 AND owner_id = $2",
        board_id,
        user_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)
}
