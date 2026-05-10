use std::env;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub cookie_secure: bool,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL must be set in .env"))?;
        let host = env::var("APP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
        let port: u16 = env::var("APP_PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()?;
        let cookie_secure = env::var("COOKIE_SECURE")
            .map(|v| v == "true" || v == "1")
            .unwrap_or(false);
        Ok(Self { database_url, host, port, cookie_secure })
    }
}
