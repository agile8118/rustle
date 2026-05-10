use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct Card {
    pub id: Uuid,
    pub column_id: Uuid,
    pub title: String,
    pub description: String,
    pub position: i32,
    pub due_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}
