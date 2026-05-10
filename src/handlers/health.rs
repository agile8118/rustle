use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

pub async fn healthz(State(state): State<AppState>) -> Json<Value> {
    let db = match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await
    {
        Ok(_) => "ok",
        Err(_) => "error",
    };
    Json(json!({ "status": "ok", "db": db }))
}
