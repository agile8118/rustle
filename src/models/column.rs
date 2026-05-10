use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct BoardColumn {
    pub id: Uuid,
    pub board_id: Uuid,
    pub title: String,
    pub position: i32,
    pub created_at: DateTime<Utc>,
}
