use std::env;

const DEFAULT_DATABASE_URL: &str =
    "postgres://postgres:postgres@localhost:5432/event_register";
const DEFAULT_BIND_ADDR: &str = "0.0.0.0:8080";
const DEFAULT_CORS_ORIGINS: &str = "http://localhost:3000,http://localhost,https://localhost";

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: String,
    pub max_connections: u32,
    pub allowed_origins: Vec<String>,
}

impl Config {
    /// Builds configuration from the process environment.
    ///
    /// All settings have development-friendly defaults so `cargo run` works
    /// against the local compose database without extra setup.
    pub fn from_env() -> Result<Self, String> {
        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_owned());
        let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| DEFAULT_BIND_ADDR.to_owned());

        let max_connections = match env::var("DATABASE_MAX_CONNECTIONS") {
            Ok(value) => value
                .parse::<u32>()
                .map_err(|_| "DATABASE_MAX_CONNECTIONS must be a positive integer".to_owned())?,
            Err(_) => 20,
        };
        if max_connections == 0 {
            return Err("DATABASE_MAX_CONNECTIONS must be greater than zero".to_owned());
        }

        let allowed_origins = env::var("CORS_ALLOWED_ORIGINS")
            .unwrap_or_else(|_| DEFAULT_CORS_ORIGINS.to_owned())
            .split(',')
            .map(str::trim)
            .filter(|origin| !origin.is_empty())
            .map(ToOwned::to_owned)
            .collect();

        Ok(Self {
            database_url,
            bind_addr,
            max_connections,
            allowed_origins,
        })
    }
}
