pub mod auth;
pub mod config;
pub mod error;
pub mod handlers;
pub mod logging;
pub mod models;
pub mod router;
pub mod state;

use crate::config::AppConfig;
use crate::router::build_router;
use crate::state::AppState;
use axum::Router;
use sqlx::PgPool;

pub fn app(pool: PgPool) -> Router {
    let config = AppConfig {
        database_url: String::new(),
        host: "127.0.0.1".to_string(),
        port: 0,
        cookie_secure: false,
    };
    build_router(AppState::new(pool, config))
}

pub fn app_with_config(pool: PgPool, config: AppConfig) -> Router {
    build_router(AppState::new(pool, config))
}
