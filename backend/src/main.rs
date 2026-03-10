mod ai;
mod arabic_stemmer;
mod auth;
mod cache;
mod config;
mod db;
mod handlers;
mod models;
mod produk_hukum;
mod query_translator;
mod rate_limit;
mod search;

use actix_cors::Cors;
use actix_web::{web, App, HttpServer, middleware};
use log::info;
use std::sync::Arc;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let config = config::Config::from_env();
    info!("Starting Kizana Search on {}:{}", config.host, config.port);

    // Initialize database
    let db = db::Database::new(&config).expect("Failed to initialize database");
    info!("Database initialized with {} books", db.get_book_ids().len());

    // Initialize search engine
    let search = search::SearchEngine::new(&config.tantivy_index_path, db.clone())
        .expect("Failed to initialize search engine");

    // Build index in background
    let search_clone = search.clone();
    std::thread::spawn(move || {
        if let Err(e) = search_clone.build_index() {
            log::error!("Failed to build index: {}", e);
        }
    });

    // Initialize cache (optional - graceful degradation without Redis)
    let cache = match cache::CacheManager::new(&config.redis_url) {
        Ok(c) => {
            info!("Redis cache connected");
            Some(c)
        }
        Err(e) => {
            log::warn!("Redis not available ({}), running without cache", e);
            None
        }
    };

    // Initialize AI client
    let ai = ai::AiClient::new(&config);

    // Initialize Produk Hukum database (optional - graceful degradation)
    let produk_hukum_db = match produk_hukum::ProdukHukumDb::new(&config.produk_hukum_db_path) {
        Ok(db) => {
            info!("Produk Hukum database loaded");
            Some(db)
        }
        Err(e) => {
            log::warn!("Produk Hukum DB not available ({}), feature disabled", e);
            None
        }
    };

    // Initialize rate limiters
    let rate_limiter = Arc::new(rate_limit::RateLimiter::new(
        config.rate_limit_per_minute,
        60,
    ));
    let auth_rate_limiter = Arc::new(rate_limit::RateLimiter::new(
        config.rate_limit_auth_per_minute,
        60,
    ));
    info!(
        "Rate limiters: {} req/min general, {} req/min auth",
        config.rate_limit_per_minute, config.rate_limit_auth_per_minute
    );

    // Background task: cleanup rate limiter stale entries every 5 minutes
    let rl_cleanup = rate_limiter.clone();
    let arl_cleanup = auth_rate_limiter.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(300));
        rl_cleanup.cleanup();
        arl_cleanup.cleanup();
    });

    // Background task: cleanup old sessions and query logs every 6 hours
    let db_for_cleanup = db.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(21600)); // 6 hours
        match db_for_cleanup.cleanup_old_sessions(90) {
            Ok(count) => log::info!("Session cleanup: {} old sessions removed", count),
            Err(e) => log::error!("Session cleanup error: {}", e),
        }
        match db_for_cleanup.cleanup_old_query_logs(30) {
            Ok(count) => log::info!("Query log cleanup: {} old logs removed", count),
            Err(e) => log::error!("Query log cleanup error: {}", e),
        }
    });

    let frontend_url = config.frontend_url.clone();
    let host = config.host.clone();
    let port = config.port;

    let app_state = web::Data::new(handlers::AppState {
        db,
        search,
        ai,
        cache,
        produk_hukum_db,
        config,
        rate_limiter,
        auth_rate_limiter,
    });

    info!("Server ready at http://{}:{}", host, port);

    HttpServer::new(move || {
        let mut cors = Cors::default()
            .allowed_origin(&frontend_url)
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE", "OPTIONS"])
            .allowed_headers(vec![
                actix_web::http::header::AUTHORIZATION,
                actix_web::http::header::CONTENT_TYPE,
                actix_web::http::header::HeaderName::from_static("x-api-key"),
            ])
            .supports_credentials()
            .max_age(3600);

        // Only allow localhost origins in debug builds
        #[cfg(debug_assertions)]
        {
            cors = cors
                .allowed_origin("http://localhost:5173")
                .allowed_origin("http://localhost:3000");
        }

        App::new()
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .app_data(web::JsonConfig::default().limit(65_536)) // 64KB max payload
            .app_data(app_state.clone())
            // Public routes
            .route("/api/health", web::get().to(handlers::health))
            .route("/api/status", web::get().to(handlers::status))
            .route("/api/auth/register", web::post().to(handlers::register))
            .route("/api/auth/login", web::post().to(handlers::login))
            // Public Produk Hukum routes
            .route("/api/produk-hukum", web::get().to(handlers::list_produk_hukum))
            .route("/api/produk-hukum/search", web::get().to(handlers::search_produk_hukum))
            .route("/api/produk-hukum/stats", web::get().to(handlers::produk_hukum_stats))
            .route("/api/produk-hukum/{id}", web::get().to(handlers::get_produk_hukum))
            // Protected routes — Auth & Profile
            .route("/api/auth/profile", web::get().to(handlers::get_profile))
            .route("/api/auth/profile", web::put().to(handlers::update_profile))
            .route("/api/auth/change-password", web::post().to(handlers::change_password))
            // Protected routes — Search & Chat
            .route("/api/query", web::post().to(handlers::query))
            .route("/api/query/stream", web::post().to(handlers::query_stream))
            .route("/api/book", web::post().to(handlers::read_book))
            // Protected routes — Session Management
            .route("/api/sessions", web::get().to(handlers::get_sessions))
            .route("/api/sessions/delete", web::post().to(handlers::delete_sessions_batch_handler))
            .route("/api/sessions/{id}", web::get().to(handlers::get_session))
            .route("/api/sessions/{id}", web::delete().to(handlers::delete_session_handler))
            .route("/api/sessions/{id}", web::put().to(handlers::rename_session_handler))
            // Protected routes — API Key Management (Task 11)
            .route("/api/api-keys", web::get().to(handlers::list_api_keys))
            .route("/api/api-keys", web::post().to(handlers::create_api_key))
            .route("/api/api-keys/{id}", web::delete().to(handlers::revoke_api_key))
            // Public API v1 — API Key auth (Task 11)
            .route("/api/v1/search", web::post().to(handlers::api_v1_search))
            // Evaluation endpoints — Admin only (Academic Research)
            .route("/api/eval/batch", web::post().to(handlers::eval_batch))
            .route("/api/eval/logs", web::get().to(handlers::query_log_stats))
            // P2: Feedback endpoints
            .route("/api/feedback", web::post().to(handlers::submit_feedback))
    })
    .bind(format!("{}:{}", host, port))?
    .workers(4)
    .run()
    .await
}
