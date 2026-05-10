use crate::config::AppConfig;
use sqlx::PgPool;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: AppConfig,
}

impl AppState {
    pub fn new(pool: PgPool, config: AppConfig) -> Self {
        Self { pool, config }
    }
}
