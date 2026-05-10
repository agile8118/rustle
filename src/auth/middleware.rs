use crate::auth::session::{user_for_token, SESSION_COOKIE};
use crate::error::AppError;
use crate::models::user::User;
use crate::state::AppState;
use axum::extract::{FromRequestParts, State};
use axum::http::request::Parts;
use axum::http::Uri;
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use tower_cookies::Cookies;

#[derive(Clone, Debug)]
pub struct CurrentUser(pub User);

pub async fn require_user(
    State(state): State<AppState>,
    cookies: Cookies,
    uri: Uri,
    mut req: axum::extract::Request,
    next: Next,
) -> Response {
    let token = cookies.get(SESSION_COOKIE).map(|c| c.value().to_string());

    let user = match token {
        Some(t) => match user_for_token(&state.pool, &t).await {
            Ok(Some(u)) => u,
            Ok(None) => return reject(uri),
            Err(e) => return e.into_response(),
        },
        None => return reject(uri),
    };

    req.extensions_mut().insert(CurrentUser(user));
    next.run(req).await
}

fn reject(uri: Uri) -> Response {
    if uri.path().starts_with("/api/") {
        AppError::Unauthorized.into_response()
    } else {
        Redirect::to("/login").into_response()
    }
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<CurrentUser>()
            .cloned()
            .ok_or(AppError::Unauthorized)
    }
}
