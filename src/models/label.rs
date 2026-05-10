use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, sqlx::FromRow)]
pub struct Label {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub name: String,
    pub color: String,
}
