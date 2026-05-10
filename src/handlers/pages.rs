use crate::auth::session::{user_for_token, SESSION_COOKIE};
use crate::auth::CurrentUser;
use crate::error::AppResult;
use crate::models::user::User;
use crate::state::AppState;
use askama::Template;
use askama_axum::IntoResponse;
use axum::extract::{Path, State};
use axum::response::Response;
use tower_cookies::Cookies;
use uuid::Uuid;

#[derive(Template)]
#[template(path = "landing.html")]
pub struct LandingTpl {
    pub current_user: Option<User>,
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTpl {
    pub current_user: Option<User>,
}

#[derive(Template)]
#[template(path = "register.html")]
pub struct RegisterTpl {
    pub current_user: Option<User>,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
pub struct DashboardTpl {
    pub current_user: Option<User>,
    pub boards: Vec<crate::models::board::Board>,
}

#[derive(Template)]
#[template(path = "board.html")]
pub struct BoardTpl {
    pub current_user: Option<User>,
    pub board_id: Uuid,
    pub board_title: String,
}

#[derive(Template)]
#[template(path = "settings.html")]
pub struct SettingsTpl {
    pub current_user: Option<User>,
}

pub async fn landing(State(state): State<AppState>, cookies: Cookies) -> Response {
    let user = current_user_opt(&state, &cookies).await;
    LandingTpl { current_user: user }.into_response()
}

pub async fn login_page(State(state): State<AppState>, cookies: Cookies) -> Response {
    let user = current_user_opt(&state, &cookies).await;
    LoginTpl { current_user: user }.into_response()
}

pub async fn register_page(State(state): State<AppState>, cookies: Cookies) -> Response {
    let user = current_user_opt(&state, &cookies).await;
    RegisterTpl { current_user: user }.into_response()
}

pub async fn dashboard(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
) -> AppResult<Response> {
    let boards = sqlx::query_as!(
        crate::models::board::Board,
        "SELECT id, owner_id, title, created_at FROM boards WHERE owner_id = $1 ORDER BY created_at DESC",
        user.id
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(DashboardTpl {
        current_user: Some(user),
        boards,
    }
    .into_response())
}

pub async fn board_page(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> AppResult<Response> {
    let board = crate::handlers::boards::fetch_board_owned(&state.pool, id, user.id).await?;
    Ok(BoardTpl {
        current_user: Some(user),
        board_id: board.id,
        board_title: board.title,
    }
    .into_response())
}

pub async fn settings_page(CurrentUser(user): CurrentUser) -> Response {
    SettingsTpl {
        current_user: Some(user),
    }
    .into_response()
}

async fn current_user_opt(state: &AppState, cookies: &Cookies) -> Option<User> {
    let token = cookies.get(SESSION_COOKIE)?.value().to_string();
    user_for_token(&state.pool, &token).await.ok().flatten()
}
