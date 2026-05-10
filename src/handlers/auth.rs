use crate::auth::{password, session, CurrentUser};
use crate::error::{AppError, AppResult};
use crate::models::user::{PublicUser, User};
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use tower_cookies::Cookies;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterReq {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, max = 256))]
    pub password: String,
    #[validate(length(min = 1, max = 80))]
    pub display_name: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginReq {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 1))]
    pub password: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ChangePasswordReq {
    #[validate(length(min = 1))]
    pub current: String,
    #[validate(length(min = 8, max = 256))]
    pub new: String,
}

pub async fn register(
    State(state): State<AppState>,
    cookies: Cookies,
    Json(req): Json<RegisterReq>,
) -> AppResult<Json<PublicUser>> {
    req.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    let exists: Option<(uuid::Uuid,)> = sqlx::query_as("SELECT id FROM users WHERE email = $1")
        .bind(&req.email)
        .fetch_optional(&state.pool)
        .await?;
    if exists.is_some() {
        return Err(AppError::Conflict("email already registered".into()));
    }

    let hash = password::hash(&req.password)?;
    let user = sqlx::query_as!(
        User,
        r#"INSERT INTO users (email, password_hash, display_name)
           VALUES ($1, $2, $3)
           RETURNING id, email::text as "email!", password_hash, display_name, created_at"#,
        req.email,
        hash,
        req.display_name
    )
    .fetch_one(&state.pool)
    .await?;

    let token = session::create(&state.pool, user.id).await?;
    cookies.add(session::cookie_for(token, state.config.cookie_secure));
    Ok(Json(user.into()))
}

pub async fn login(
    State(state): State<AppState>,
    cookies: Cookies,
    Json(req): Json<LoginReq>,
) -> AppResult<Json<PublicUser>> {
    req.validate().map_err(|e| AppError::Validation(e.to_string()))?;

    let user: Option<User> = sqlx::query_as!(
        User,
        r#"SELECT id, email::text as "email!", password_hash, display_name, created_at
           FROM users WHERE email = $1"#,
        req.email
    )
    .fetch_optional(&state.pool)
    .await?;

    let user = user.ok_or(AppError::Unauthorized)?;
    password::verify(&req.password, &user.password_hash)?;

    let token = session::create(&state.pool, user.id).await?;
    cookies.add(session::cookie_for(token, state.config.cookie_secure));
    Ok(Json(user.into()))
}

pub async fn logout(
    State(state): State<AppState>,
    cookies: Cookies,
) -> AppResult<Json<serde_json::Value>> {
    if let Some(c) = cookies.get(session::SESSION_COOKIE) {
        let _ = session::revoke(&state.pool, c.value()).await;
    }
    cookies.add(session::clear_cookie());
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn me(CurrentUser(user): CurrentUser) -> AppResult<Json<PublicUser>> {
    Ok(Json(user.into()))
}

pub async fn change_password(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Json(req): Json<ChangePasswordReq>,
) -> AppResult<Json<serde_json::Value>> {
    req.validate().map_err(|e| AppError::Validation(e.to_string()))?;
    password::verify(&req.current, &user.password_hash)?;
    let new_hash = password::hash(&req.new)?;
    sqlx::query!(
        "UPDATE users SET password_hash = $1 WHERE id = $2",
        new_hash,
        user.id
    )
    .execute(&state.pool)
    .await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn delete_account(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    cookies: Cookies,
) -> AppResult<Json<serde_json::Value>> {
    // ON DELETE CASCADE on every FK fans out to sessions, boards,
    // columns, cards, comments, labels, and card_labels.
    sqlx::query!("DELETE FROM users WHERE id = $1", user.id)
        .execute(&state.pool)
        .await?;
    cookies.add(crate::auth::session::clear_cookie());
    Ok(Json(serde_json::json!({ "ok": true })))
}
