use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct Board {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub title: String,
    pub created_at: DateTime<Utc>,
}
