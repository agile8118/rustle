use crate::error::AppResult;
use crate::models::user::User;
use chrono::{Duration, Utc};
use rand::RngCore;
use sqlx::PgPool;
use tower_cookies::{
    cookie::{time::Duration as CookieDuration, SameSite},
    Cookie,
};
use uuid::Uuid;

pub const SESSION_COOKIE: &str = "session";
pub const SESSION_DAYS: i64 = 30;

pub fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub async fn create(pool: &PgPool, user_id: Uuid) -> AppResult<String> {
    let token = generate_token();
    let expires_at = Utc::now() + Duration::days(SESSION_DAYS);
    sqlx::query!(
        "INSERT INTO sessions (token, user_id, expires_at) VALUES ($1, $2, $3)",
        token,
        user_id,
        expires_at
    )
    .execute(pool)
    .await?;
    Ok(token)
}

pub async fn user_for_token(pool: &PgPool, token: &str) -> AppResult<Option<User>> {
    let row = sqlx::query_as!(
        User,
        r#"SELECT u.id, u.email::text as "email!", u.password_hash, u.display_name, u.created_at
           FROM users u
           JOIN sessions s ON s.user_id = u.id
           WHERE s.token = $1 AND s.expires_at > now()"#,
        token
    )
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn revoke(pool: &PgPool, token: &str) -> AppResult<()> {
    sqlx::query!("DELETE FROM sessions WHERE token = $1", token)
        .execute(pool)
        .await?;
    Ok(())
}

pub fn cookie_for(token: String, secure: bool) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, token))
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(secure)
        .path("/")
        .max_age(CookieDuration::days(SESSION_DAYS))
        .build()
}

pub fn clear_cookie() -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, ""))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .max_age(CookieDuration::seconds(0))
        .build()
}
