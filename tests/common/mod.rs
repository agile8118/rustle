use reqwest::Client;
use rustle::app;
use sqlx::PgPool;
use std::net::SocketAddr;

pub struct TestApp {
    pub address: String,
    pub client: Client,
    #[allow(dead_code)]
    pub pool: PgPool,
}

impl TestApp {
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.address, path)
    }
}

pub async fn spawn(pool: PgPool) -> TestApp {
    let app = app(pool.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    let client = Client::builder()
        .cookie_store(true)
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .unwrap();
    TestApp {
        address: format!("http://{}", addr),
        client,
        pool,
    }
}

pub async fn register_user(app: &TestApp, email: &str, password: &str) -> serde_json::Value {
    let res = app
        .client
        .post(app.url("/api/auth/register"))
        .json(&serde_json::json!({
            "email": email,
            "password": password,
            "display_name": email.split('@').next().unwrap(),
        }))
        .send()
        .await
        .unwrap();
    assert!(res.status().is_success(), "register failed: {}", res.status());
    res.json().await.unwrap()
}
