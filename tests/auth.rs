mod common;

use common::{register_user, spawn};
use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn register_login_me_logout_flow(pool: PgPool) {
    let app = spawn(pool).await;

    let user = register_user(&app, "alice@example.com", "supersecret123").await;
    assert_eq!(user["email"], "alice@example.com");
    assert_eq!(user["display_name"], "alice");

    // /me returns the user
    let me = app.client.get(app.url("/api/auth/me")).send().await.unwrap();
    assert_eq!(me.status(), 200);
    let me_body: serde_json::Value = me.json().await.unwrap();
    assert_eq!(me_body["email"], "alice@example.com");

    // logout
    let logout = app
        .client
        .post(app.url("/api/auth/logout"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(logout.status(), 200);

    // After logout, /me returns 401
    let me2 = app.client.get(app.url("/api/auth/me")).send().await.unwrap();
    assert_eq!(me2.status(), 401);

    // Re-login with same credentials
    let login = app
        .client
        .post(app.url("/api/auth/login"))
        .json(&serde_json::json!({
            "email": "alice@example.com",
            "password": "supersecret123",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(login.status(), 200);

    // Now /me works again
    let me3 = app.client.get(app.url("/api/auth/me")).send().await.unwrap();
    assert_eq!(me3.status(), 200);
}

#[sqlx::test(migrations = "./migrations")]
async fn login_rejects_wrong_password(pool: PgPool) {
    let app = spawn(pool).await;
    register_user(&app, "bob@example.com", "correctpassword1").await;

    // Use a fresh client so the registration cookie is not present
    let fresh = reqwest::Client::builder()
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();

    let bad = fresh
        .post(app.url("/api/auth/login"))
        .json(&serde_json::json!({
            "email": "bob@example.com",
            "password": "wrongpass1",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(bad.status(), 401);
}

#[sqlx::test(migrations = "./migrations")]
async fn duplicate_email_is_rejected(pool: PgPool) {
    let app = spawn(pool).await;
    register_user(&app, "carol@example.com", "longpassword99").await;

    let dup = app
        .client
        .post(app.url("/api/auth/register"))
        .json(&serde_json::json!({
            "email": "carol@example.com",
            "password": "anotherone99",
            "display_name": "Carol2",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(dup.status(), 409);
}

#[sqlx::test(migrations = "./migrations")]
async fn delete_account_removes_user_and_logs_out(pool: PgPool) {
    let app = spawn(pool.clone()).await;
    register_user(&app, "erin@example.com", "erinpassword11").await;

    // Create a board so we know cascades work
    app.client
        .post(app.url("/api/boards"))
        .json(&serde_json::json!({ "title": "to be wiped" }))
        .send()
        .await
        .unwrap();

    // Delete account
    let res = app
        .client
        .delete(app.url("/api/auth/me"))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // Subsequent /me is 401 (cookie was cleared, session was deleted via cascade)
    let me = app.client.get(app.url("/api/auth/me")).send().await.unwrap();
    assert_eq!(me.status(), 401);

    // Logging in fails — user no longer exists
    let login = app
        .client
        .post(app.url("/api/auth/login"))
        .json(&serde_json::json!({
            "email": "erin@example.com",
            "password": "erinpassword11",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(login.status(), 401);

    // Direct DB check: user row + their board are gone
    let user_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE email = 'erin@example.com'")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(user_count.0, 0);
    let board_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM boards")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(board_count.0, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn change_password_workflow(pool: PgPool) {
    let app = spawn(pool).await;
    register_user(&app, "dave@example.com", "originalpassword1").await;

    let res = app
        .client
        .patch(app.url("/api/auth/password"))
        .json(&serde_json::json!({
            "current": "originalpassword1",
            "new": "rotatedpassword2",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // logout, then login with the new password
    app.client
        .post(app.url("/api/auth/logout"))
        .json(&serde_json::json!({}))
        .send()
        .await
        .unwrap();

    let bad = app
        .client
        .post(app.url("/api/auth/login"))
        .json(&serde_json::json!({
            "email": "dave@example.com",
            "password": "originalpassword1",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(bad.status(), 401);

    let good = app
        .client
        .post(app.url("/api/auth/login"))
        .json(&serde_json::json!({
            "email": "dave@example.com",
            "password": "rotatedpassword2",
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(good.status(), 200);
}
