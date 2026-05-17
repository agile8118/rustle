use rustle::{app_with_config, config::AppConfig};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("rustle=info,tower_http=info,axum=info")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = AppConfig::from_env()?;
    tracing::info!(host = %config.host, port = config.port, "starting rustle");

    ensure_database(&config.database_url).await?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    let addr = format!("{}:{}", config.host, config.port);
    let app = app_with_config(pool, config);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on http://{}", addr);
    axum::serve(listener, app).await?;
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
