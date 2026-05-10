mod common;

use common::{register_user, spawn};
use reqwest::Client;
use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn landing_page_renders(pool: PgPool) {
    let app = spawn(pool).await;
    let client = Client::builder().redirect(reqwest::redirect::Policy::none()).build().unwrap();
    let res = client.get(app.url("/")).send().await.unwrap();
    assert_eq!(res.status(), 200);
    let body = res.text().await.unwrap();
    assert!(body.contains("Rustle"));
    assert!(body.contains("<html"));
}

#[sqlx::test(migrations = "./migrations")]
async fn dashboard_redirects_when_logged_out(pool: PgPool) {
    let app = spawn(pool).await;
    let client = Client::builder().redirect(reqwest::redirect::Policy::none()).build().unwrap();
    let res = client.get(app.url("/dashboard")).send().await.unwrap();
    assert!(res.status().is_redirection(), "expected redirect, got {}", res.status());
    let location = res.headers().get("location").unwrap().to_str().unwrap();
    assert!(location.starts_with("/login"));
}

#[sqlx::test(migrations = "./migrations")]
async fn dashboard_renders_when_logged_in(pool: PgPool) {
    let app = spawn(pool).await;
    register_user(&app, "isla@example.com", "islapassword11").await;
    let res = app.client.get(app.url("/dashboard")).send().await.unwrap();
    assert_eq!(res.status(), 200);
    let body = res.text().await.unwrap();
    assert!(body.contains("Your boards"));
    assert!(body.contains("isla"));
}

#[sqlx::test(migrations = "./migrations")]
async fn healthz_returns_ok(pool: PgPool) {
    let app = spawn(pool).await;
    let client = Client::new();
    let res = client.get(app.url("/healthz")).send().await.unwrap();
    assert_eq!(res.status(), 200);
    let body: serde_json::Value = res.json().await.unwrap();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["db"], "ok");
}

#[sqlx::test(migrations = "./migrations")]
async fn static_css_is_served(pool: PgPool) {
    let app = spawn(pool).await;
    let client = Client::new();
    let res = client.get(app.url("/static/css/app.css")).send().await.unwrap();
    assert_eq!(res.status(), 200);
    let body = res.text().await.unwrap();
    assert!(body.contains("--accent"));
}
