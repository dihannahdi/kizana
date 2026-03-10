use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_path: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub ai_api_key: String,
    pub ai_api_url: String,
    pub ai_model: String,
    pub host: String,
    pub port: u16,
    pub tantivy_index_path: String,
    pub frontend_url: String,
    pub produk_hukum_db_path: String,
    pub bcrypt_cost: u32,
    pub jwt_expiry_hours: u64,
    pub rate_limit_per_minute: u32,
    pub rate_limit_auth_per_minute: u32,
}

impl Config {
    pub fn from_env() -> Self {
        dotenv::dotenv().ok();
        Config {
            database_path: std::env::var("DATABASE_PATH")
                .unwrap_or_else(|_| "./kizana_all_books.sqlite".to_string()),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            jwt_secret: {
                let secret = std::env::var("JWT_SECRET")
                    .expect("FATAL: JWT_SECRET environment variable must be set");
                if secret.len() < 32 {
                    panic!("FATAL: JWT_SECRET must be at least 32 characters for security");
                }
                secret
            },
            // Validate JWT_SECRET meets minimum length
            ai_api_key: {
                let key = std::env::var("AI_API_KEY").unwrap_or_default();
                if key.is_empty() {
                    log::warn!("AI_API_KEY not set - AI features will be disabled");
                }
                key
            },
            ai_api_url: std::env::var("AI_API_URL")
                .unwrap_or_else(|_| "https://api.x.ai/v1/chat/completions".to_string()),
            ai_model: std::env::var("AI_MODEL")
                .unwrap_or_else(|_| "grok-3-mini".to_string()),
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            tantivy_index_path: std::env::var("TANTIVY_INDEX_PATH")
                .unwrap_or_else(|_| "./tantivy_index".to_string()),
            frontend_url: std::env::var("FRONTEND_URL")
                .unwrap_or_else(|_| "http://localhost:5173".to_string()),
            produk_hukum_db_path: std::env::var("PRODUK_HUKUM_DB_PATH")
                .unwrap_or_else(|_| "./produk_hukum.sqlite".to_string()),
            bcrypt_cost: std::env::var("BCRYPT_COST")
                .unwrap_or_else(|_| "12".to_string())
                .parse()
                .unwrap_or(12),
            jwt_expiry_hours: std::env::var("JWT_EXPIRY_HOURS")
                .unwrap_or_else(|_| "24".to_string()) // 24 hours
                .parse()
                .unwrap_or(24),
            rate_limit_per_minute: std::env::var("RATE_LIMIT_PER_MINUTE")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .unwrap_or(60),
            rate_limit_auth_per_minute: std::env::var("RATE_LIMIT_AUTH_PER_MINUTE")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
        }
    }
}
