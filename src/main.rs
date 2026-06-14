use rustle::{app_with_config, config::AppConfig, logging};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    proctitle::set_title("rustle-server");

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("rustle=info,tower_http=info,axum=info")))
        .with(logging::file_log_layer())
        .init();

    let config = AppConfig::from_env()?;
    tracing::info!(host = %config.host, port = config.port, "starting rustle");

    ensure_database(&config.database_url).await?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;
    println!("[postgres] connected successfully");

    sqlx::migrate!("./migrations").run(&pool).await?;

    let port = config.port;
    let addr = format!("{}:{}", config.host, config.port);
    let app = app_with_config(pool, config);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("Server is on port {port}");
    tracing::info!("listening on http://{}", addr);
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await?;
    Ok(())
}

async fn ensure_database(database_url: &str) -> anyhow::Result<()> {
    let (admin_url, db_name) = match database_url.rfind('/') {
        Some(i) => (&database_url[..i], &database_url[i + 1..]),
        None => anyhow::bail!("invalid DATABASE_URL"),
    };
    let admin_url = format!("{}/postgres", admin_url);
    let pool = PgPoolOptions::new().max_connections(1).connect(&admin_url).await?;
    let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname = $1)")
        .bind(db_name)
        .fetch_one(&pool)
        .await?;
    if !exists {
        sqlx::query(&format!("CREATE DATABASE \"{}\"", db_name))
            .execute(&pool)
            .await?;
        tracing::info!("created database '{}'", db_name);
    }
    Ok(())
}
