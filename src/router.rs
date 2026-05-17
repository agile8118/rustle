use crate::auth::middleware::require_user;
use crate::handlers::*;
use crate::state::AppState;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use axum::routing::{delete, get, patch, post};
use axum::{middleware, Router};
use std::time::Instant;
use tower_cookies::CookieManagerLayer;
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;

async fn access_log(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = Instant::now();
    let res = next.run(req).await;
    let ms = start.elapsed().as_millis();
    let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    println!("{ts}  -- {method} {uri} {} -- response-time: {ms} ms", res.status());
    res
}

pub fn build_router(state: AppState) -> Router {
    let public_pages = Router::new()
        .route("/", get(pages::landing))
        .route("/login", get(pages::login_page))
        .route("/register", get(pages::register_page));

    let auth_api_public = Router::new()
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login));

    let public_misc = Router::new().route("/healthz", get(health::healthz));

    let private_pages = Router::new()
        .route("/dashboard", get(pages::dashboard))
        .route("/board/:id", get(pages::board_page))
        .route("/settings", get(pages::settings_page));

    let private_api = Router::new()
        // auth
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/auth/me", get(auth::me).delete(auth::delete_account))
        .route("/api/auth/password", patch(auth::change_password))
        // boards
        .route("/api/boards", get(boards::list).post(boards::create))
        .route(
            "/api/boards/:id",
            get(boards::get).patch(boards::update).delete(boards::delete),
        )
        // columns
        .route("/api/boards/:id/columns", post(columns::create))
        .route(
            "/api/columns/:id",
            patch(columns::update).delete(columns::delete),
        )
        // cards
        .route("/api/columns/:id/cards", post(cards::create))
        .route(
            "/api/cards/:id",
            get(cards::get).patch(cards::update).delete(cards::delete),
        )
        .route("/api/cards/:id/move", post(cards::move_card))
        // comments
        .route(
            "/api/cards/:id/comments",
            get(comments::list).post(comments::create),
        )
        .route("/api/comments/:id", delete(comments::delete))
        // labels
        .route("/api/labels", get(labels::list).post(labels::create))
        .route("/api/cards/:id/labels", post(labels::attach))
        .route("/api/cards/:id/labels/:label_id", delete(labels::detach));

    let protected = private_pages.merge(private_api).route_layer(
        middleware::from_fn_with_state(state.clone(), require_user),
    );

    Router::new()
        .merge(public_pages)
        .merge(auth_api_public)
        .merge(public_misc)
        .merge(protected)
        .nest_service("/static", ServeDir::new("public"))
        .layer(CookieManagerLayer::new())
        .layer(CompressionLayer::new())
        .layer(middleware::from_fn(access_log))
        .with_state(state)
}
