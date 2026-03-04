use crate::ai::AiClient;
use crate::auth;
use crate::cache::CacheManager;
use crate::config::Config;
use crate::db::Database;
use crate::models::*;
use crate::produk_hukum::ProdukHukumDb;
use crate::rate_limit::RateLimiter;
use crate::search::SearchEngine;
use actix_web::{web, HttpRequest, HttpResponse};
use log::{info, error, warn};
use std::sync::Arc;
use std::time::Instant;

pub struct AppState {
    pub db: Arc<Database>,
    pub search: Arc<SearchEngine>,
    pub ai: AiClient,
    pub cache: Option<CacheManager>,
    pub produk_hukum_db: Option<Arc<ProdukHukumDb>>,
    pub config: Config,
    pub rate_limiter: Arc<RateLimiter>,
    pub auth_rate_limiter: Arc<RateLimiter>,
}

/// Extract client IP from request — only trust X-Forwarded-For from loopback (Nginx)
fn get_client_ip(req: &HttpRequest) -> String {
    let peer_ip = req.peer_addr()
        .map(|addr| addr.ip().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    
    // Only trust X-Forwarded-For if request comes from local reverse proxy
    let trusted = peer_ip == "127.0.0.1" || peer_ip == "::1";
    if trusted {
        req.headers()
            .get("X-Forwarded-For")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(',').next())
            .map(|s| s.trim().to_string())
            .unwrap_or(peer_ip)
    } else {
        peer_ip
    }
}

// ─── Auth Handlers ───

pub async fn register(
    req: HttpRequest,
    data: web::Data<AppState>,
    body: web::Json<RegisterRequest>,
) -> HttpResponse {
    // Rate limit auth endpoints
    let ip = get_client_ip(&req);
    if !data.auth_rate_limiter.check(&ip) {
        return HttpResponse::TooManyRequests().json(serde_json::json!({
            "error": "Terlalu banyak percobaan. Coba lagi nanti."
        }));
    }

    let email = body.email.trim().to_lowercase();
    if email.is_empty() || body.password.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Email dan password wajib diisi"
        }));
    }

    // Validate email format
    if !auth::is_valid_email(&email) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Format email tidak valid"
        }));
    }

    // Validate password strength
    if let Err(e) = auth::validate_password(&body.password) {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": e
        }));
    }

    let password_hash = match bcrypt::hash(&body.password, data.config.bcrypt_cost) {
        Ok(h) => h,
        Err(e) => {
            error!("Bcrypt hash error: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Gagal memproses password"
            }));
        }
    };

    let display_name = body.display_name.as_deref().unwrap_or("").trim().to_string();

    match data.db.create_user(&email, &password_hash, &display_name) {
        Ok(user) => {
            let token = auth::create_token(user.id, &user.email, &user.role, &data.config)
                .unwrap_or_default();
            info!("New user registered: {} (id={})", user.email, user.id);
            HttpResponse::Ok().json(AuthResponse {
                token,
                user: UserInfo {
                    id: user.id,
                    email: user.email,
                    display_name: user.display_name,
                    role: user.role,
                },
            })
        }
        Err(e) => HttpResponse::Conflict().json(serde_json::json!({ "error": e })),
    }
}

pub async fn login(
    req: HttpRequest,
    data: web::Data<AppState>,
    body: web::Json<LoginRequest>,
) -> HttpResponse {
    // Rate limit auth endpoints
    let ip = get_client_ip(&req);
    if !data.auth_rate_limiter.check(&ip) {
        return HttpResponse::TooManyRequests().json(serde_json::json!({
            "error": "Terlalu banyak percobaan login. Coba lagi nanti."
        }));
    }

    let email = body.email.trim().to_lowercase();

    match data.db.get_user_by_email(&email) {
        Ok(Some(user)) => {
            if !user.is_active {
                return HttpResponse::Forbidden().json(serde_json::json!({
                    "error": "Akun dinonaktifkan. Hubungi administrator."
                }));
            }
            if bcrypt::verify(&body.password, &user.password_hash).unwrap_or(false) {
                let token = auth::create_token(user.id, &user.email, &user.role, &data.config)
                    .unwrap_or_default();
                info!("User logged in: {} (id={})", user.email, user.id);
                HttpResponse::Ok().json(AuthResponse {
                    token,
                    user: UserInfo {
                        id: user.id,
                        email: user.email,
                        display_name: user.display_name,
                        role: user.role,
                    },
                })
            } else {
                warn!("Failed login attempt for: {}", email);
                HttpResponse::Unauthorized().json(serde_json::json!({
                    "error": "Email atau password salah"
                }))
            }
        }
        Ok(None) => HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Email atau password salah"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({
            "error": e
        })),
    }
}

// ─── Profile Handlers ───

pub async fn get_profile(
    req: HttpRequest,
    data: web::Data<AppState>,
) -> HttpResponse {
    let claims = match auth::extract_user_from_request(&req, &data.config) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({ "error": e })),
    };

    match data.db.get_user_by_id(claims.sub) {
        Ok(Some(user)) => HttpResponse::Ok().json(UserInfo {
            id: user.id,
            email: user.email,
            display_name: user.display_name,
            role: user.role,
        }),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({ "error": "User tidak ditemukan" })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

pub async fn update_profile(
    req: HttpRequest,
    data: web::Data<AppState>,
    body: web::Json<UpdateProfileRequest>,
) -> HttpResponse {
    let claims = match auth::extract_user_from_request(&req, &data.config) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({ "error": e })),
    };

    // Validate new email if provided
    if let Some(ref email) = body.email {
        let email = email.trim().to_lowercase();
        if !auth::is_valid_email(&email) {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "error": "Format email tidak valid"
            }));
        }
    }

    match data.db.update_user_profile(
        claims.sub,
        body.display_name.as_deref(),
        body.email.as_deref(),
    ) {
        Ok(_) => {
            // Return updated profile
            match data.db.get_user_by_id(claims.sub) {
                Ok(Some(user)) => HttpResponse::Ok().json(UserInfo {
                    id: user.id,
                    email: user.email,
                    display_name: user.display_name,
                    role: user.role,
                }),
                _ => HttpResponse::Ok().json(serde_json::json!({ "message": "Profil diperbarui" })),
            }
        }
        Err(e) => HttpResponse::BadRequest().json(serde_json::json!({ "error": e })),
    }
}

pub async fn change_password(
    req: HttpRequest,
    data: web::Data<AppState>,
    body: web::Json<ChangePasswordRequest>,
) -> HttpResponse {
    let claims = match auth::extract_user_from_request(&req, &data.config) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({ "error": e })),
    };

    // Validate new password strength
    if let Err(e) = auth::validate_password(&body.new_password) {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": e }));
    }

    // Verify current password
    match data.db.get_user_by_id(claims.sub) {
        Ok(Some(user)) => {
            if !bcrypt::verify(&body.current_password, &user.password_hash).unwrap_or(false) {
                return HttpResponse::Unauthorized().json(serde_json::json!({
                    "error": "Password saat ini salah"
                }));
            }
        }
        _ => return HttpResponse::NotFound().json(serde_json::json!({ "error": "User tidak ditemukan" })),
    }

    // Hash new password
    let new_hash = match bcrypt::hash(&body.new_password, data.config.bcrypt_cost) {
        Ok(h) => h,
        Err(e) => {
            error!("Bcrypt hash error: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Gagal memproses password baru"
            }));
        }
    };

    match data.db.update_user_password(claims.sub, &new_hash) {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "message": "Password berhasil diubah" })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

// ─── Search + Chat Handler ───

pub async fn query(
    req: HttpRequest,
    data: web::Data<AppState>,
    body: web::Json<QueryRequest>,
) -> HttpResponse {
    // Rate limit
    let ip = get_client_ip(&req);
    if !data.rate_limiter.check(&ip) {
        return HttpResponse::TooManyRequests().json(serde_json::json!({
            "error": "Rate limit exceeded. Coba lagi nanti."
        }));
    }

    // Verify auth
    let claims = match auth::extract_user_from_request(&req, &data.config) {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::Unauthorized().json(serde_json::json!({ "error": e }));
        }
    };

    let query_str = body.query.trim().to_string();
    if query_str.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Query cannot be empty"
        }));
    }
    if query_str.len() > 2000 {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Query terlalu panjang (maksimal 2000 karakter)"
        }));
    }

    // Check cache
    // ─── Step 1: Translate query (language detection, Arabic expansion) ───
    let translated = data.search.translate_query(&query_str);
    info!(
        "Query translated: lang={:?}, domain={:?}, arabic_terms={:?}",
        translated.detected_language, translated.detected_domain, translated.arabic_terms
    );

    let search_start = Instant::now();

    let cached_results = if let Some(ref cache) = data.cache {
        cache.get_cached_search(&query_str).await
    } else {
        None
    };

    let results = if let Some(cached) = cached_results {
        info!("Cache hit for query: {}", query_str);
        cached
    } else {
        // Search with translated query for better Arabic matching
        let mut kitab_results = match data.search.search_with_translated(&translated, 20) {
            Ok(r) => r,
            Err(e) => {
                error!("Search error: {}", e);
                return HttpResponse::InternalServerError().json(serde_json::json!({
                    "error": format!("Search failed: {}", e)
                }));
            }
        };

        // Mark all kitab results
        for r in &mut kitab_results {
            if r.source_type.is_empty() {
                r.source_type = "kitab".to_string();
            }
        }

        // ─── Unified Search: Include Produk Hukum results ───
        if let Some(ref ph_db) = data.produk_hukum_db {
            if let Ok(ph_results) = ph_db.search_for_unified(&query_str, 5) {
                kitab_results.extend(ph_results);
                // Re-sort by score (produk hukum results are already scored relative to kitab)
                kitab_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
            }
        }

        // Cache combined results
        if let Some(ref cache) = data.cache {
            cache.cache_search(&query_str, &kitab_results).await;
        }
        kitab_results
    };

    // Log query for research analysis (non-blocking, errors are silent)
    let search_time_ms = search_start.elapsed().as_millis() as i64;
    let top_score = results.first().map(|r| r.score).unwrap_or(0.0);
    let num_results = results.len() as i32;
    let lang_str = format!("{}", translated.detected_language);
    let domain_str = format!("{}", translated.detected_domain);
    let session_for_log = body.session_id.clone();
    let _ = data.db.log_query(
        &query_str,
        &lang_str,
        &domain_str,
        &translated.arabic_terms,
        num_results,
        top_score,
        search_time_ms,
        session_for_log.as_deref(),
    );

    // AI synthesis with translation context
    let ai_answer = data.ai.synthesize_answer(&query_str, &results, Some(&translated)).await
        .unwrap_or_else(|e| format!("AI synthesis error: {}", e));

    // Manage session
    let session_id = body
        .session_id
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let mut session = if let Some(ref sid) = body.session_id {
        // Try cache first, then DB
        let cached_session = if let Some(ref cache) = data.cache {
            cache.get_cached_session(sid).await
        } else {
            None
        };
        cached_session
            .or_else(|| data.db.get_session(sid, claims.sub).ok().flatten())
            .unwrap_or_else(|| ChatSession {
                id: session_id.clone(),
                user_id: claims.sub,
                title: query_str.chars().take(50).collect(),
                messages: Vec::new(),
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            })
    } else {
        ChatSession {
            id: session_id.clone(),
            user_id: claims.sub,
            title: query_str.chars().take(50).collect(),
            messages: Vec::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        }
    };

    // Add messages
    session.messages.push(ChatMessage {
        role: "user".to_string(),
        content: query_str.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });
    session.messages.push(ChatMessage {
        role: "assistant".to_string(),
        content: ai_answer.clone(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    });
    session.updated_at = chrono::Utc::now().to_rfc3339();

    // Save session
    let _ = data.db.save_session(&session);
    if let Some(ref cache) = data.cache {
        cache.cache_session(&session).await;
    }

    HttpResponse::Ok().json(QueryResponse {
        session_id,
        results,
        ai_answer,
        query: query_str,
        detected_language: format!("{}", translated.detected_language),
        detected_domain: format!("{}", translated.detected_domain),
        translated_terms: translated.arabic_terms.clone(),
    })
}

// ─── Book Reader Handler ───

pub async fn read_book(
    req: HttpRequest,
    data: web::Data<AppState>,
    body: web::Json<BookReadRequest>,
) -> HttpResponse {
    if let Err(e) = auth::extract_user_from_request(&req, &data.config) {
        return HttpResponse::Unauthorized().json(serde_json::json!({ "error": e }));
    }

    let book_id = body.book_id;
    let page = body.page.as_deref();

    let toc = match data.db.build_toc_tree(book_id) {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::NotFound().json(serde_json::json!({
                "error": format!("Book not found: {}", e)
            }));
        }
    };

    let pages = match data.db.get_book_pages(book_id, page) {
        Ok(p) => p,
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({
                "error": format!("Failed to get pages: {}", e)
            }));
        }
    };

    let total_pages = data.db.get_total_pages(book_id).unwrap_or(0);
    let current_page = page.unwrap_or("1").to_string();
    let meta = data.db.get_book_metadata(book_id);

    HttpResponse::Ok().json(BookReadResponse {
        book_id,
        book_name: meta.book_name,
        author_name: meta.author_name,
        toc,
        pages,
        current_page,
        total_pages,
    })
}

// ─── Chat History ───

pub async fn get_sessions(req: HttpRequest, data: web::Data<AppState>) -> HttpResponse {
    let claims = match auth::extract_user_from_request(&req, &data.config) {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::Unauthorized().json(serde_json::json!({ "error": e }));
        }
    };

    match data.db.get_user_sessions(claims.sub) {
        Ok(sessions) => HttpResponse::Ok().json(sessions),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

pub async fn get_session(
    req: HttpRequest,
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let claims = match auth::extract_user_from_request(&req, &data.config) {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::Unauthorized().json(serde_json::json!({ "error": e }));
        }
    };

    let session_id = path.into_inner();

    // Check cache first
    if let Some(ref cache) = data.cache {
        if let Some(session) = cache.get_cached_session(&session_id).await {
            if session.user_id == claims.sub {
                return HttpResponse::Ok().json(session);
            }
        }
    }

    match data.db.get_session(&session_id, claims.sub) {
        Ok(Some(session)) => HttpResponse::Ok().json(session),
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Session not found"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

// ─── Session Management ───

pub async fn delete_session_handler(
    req: HttpRequest,
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> HttpResponse {
    let claims = match auth::extract_user_from_request(&req, &data.config) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({ "error": e })),
    };

    let session_id = path.into_inner();

    match data.db.delete_session(&session_id, claims.sub) {
        Ok(true) => {
            // Also clear from cache
            if let Some(ref cache) = data.cache {
                cache.delete_session(&session_id).await;
                cache.invalidate_user_sessions(claims.sub).await;
            }
            HttpResponse::Ok().json(serde_json::json!({ "message": "Session dihapus" }))
        }
        Ok(false) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Session tidak ditemukan"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

pub async fn delete_sessions_batch_handler(
    req: HttpRequest,
    data: web::Data<AppState>,
    body: web::Json<DeleteSessionsRequest>,
) -> HttpResponse {
    let claims = match auth::extract_user_from_request(&req, &data.config) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({ "error": e })),
    };

    if body.session_ids.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "session_ids tidak boleh kosong"
        }));
    }

    match data.db.delete_sessions_batch(&body.session_ids, claims.sub) {
        Ok(count) => {
            // Clear from cache
            if let Some(ref cache) = data.cache {
                cache.delete_sessions(&body.session_ids).await;
                cache.invalidate_user_sessions(claims.sub).await;
            }
            HttpResponse::Ok().json(serde_json::json!({
                "message": format!("{} session dihapus", count),
                "deleted": count
            }))
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

pub async fn rename_session_handler(
    req: HttpRequest,
    data: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<RenameSessionRequest>,
) -> HttpResponse {
    let claims = match auth::extract_user_from_request(&req, &data.config) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({ "error": e })),
    };

    let session_id = path.into_inner();
    let title = body.title.trim();

    if title.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Judul tidak boleh kosong"
        }));
    }

    match data.db.rename_session(&session_id, claims.sub, title) {
        Ok(true) => {
            // Invalidate cache
            if let Some(ref cache) = data.cache {
                cache.delete_session(&session_id).await;
                cache.invalidate_user_sessions(claims.sub).await;
            }
            HttpResponse::Ok().json(serde_json::json!({ "message": "Session diubah namanya" }))
        }
        Ok(false) => HttpResponse::NotFound().json(serde_json::json!({
            "error": "Session tidak ditemukan"
        })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

// ─── Status ───

pub async fn status(data: web::Data<AppState>) -> HttpResponse {
    let (indexed, num_docs) = data.search.status();
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "indexed": indexed,
        "total_docs": num_docs,
        "total_books": data.db.get_book_ids().len(),
    }))
}

pub async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({ "status": "healthy" }))
}

// ─── Produk Hukum Handlers (Public — no auth required) ───

#[derive(Debug, serde::Deserialize)]
pub struct ProdukHukumListQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub category: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ProdukHukumSearchQuery {
    pub q: Option<String>,
    pub limit: Option<i64>,
}

pub async fn list_produk_hukum(
    data: web::Data<AppState>,
    query: web::Query<ProdukHukumListQuery>,
) -> HttpResponse {
    let ph_db = match &data.produk_hukum_db {
        Some(db) => db,
        None => {
            return HttpResponse::ServiceUnavailable().json(serde_json::json!({
                "error": "Produk Hukum feature is not available"
            }));
        }
    };

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100).max(1);
    let category = query.category.as_deref();

    match ph_db.list_documents(page, per_page, category) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

pub async fn get_produk_hukum(
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> HttpResponse {
    let ph_db = match &data.produk_hukum_db {
        Some(db) => db,
        None => {
            return HttpResponse::ServiceUnavailable().json(serde_json::json!({
                "error": "Produk Hukum feature is not available"
            }));
        }
    };

    let id = path.into_inner();
    match ph_db.get_document(id) {
        Ok(doc) => HttpResponse::Ok().json(doc),
        Err(e) => HttpResponse::NotFound().json(serde_json::json!({ "error": e })),
    }
}

pub async fn search_produk_hukum(
    data: web::Data<AppState>,
    query: web::Query<ProdukHukumSearchQuery>,
) -> HttpResponse {
    let ph_db = match &data.produk_hukum_db {
        Some(db) => db,
        None => {
            return HttpResponse::ServiceUnavailable().json(serde_json::json!({
                "error": "Produk Hukum feature is not available"
            }));
        }
    };

    let q = query.q.as_deref().unwrap_or("");
    let limit = query.limit.unwrap_or(20).min(100).max(1);

    if q.trim().is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Search query (q) is required"
        }));
    }

    match ph_db.search_documents(q, limit) {
        Ok(response) => HttpResponse::Ok().json(response),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

pub async fn produk_hukum_stats(
    data: web::Data<AppState>,
) -> HttpResponse {
    let ph_db = match &data.produk_hukum_db {
        Some(db) => db,
        None => {
            return HttpResponse::ServiceUnavailable().json(serde_json::json!({
                "error": "Produk Hukum feature is not available"
            }));
        }
    };

    match ph_db.get_stats() {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

// ─── API Key Management (Task 11) ───

pub async fn create_api_key(
    req: HttpRequest,
    data: web::Data<AppState>,
    body: web::Json<CreateApiKeyRequest>,
) -> HttpResponse {
    let claims = match auth::extract_user_from_request(&req, &data.config) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({ "error": e })),
    };

    if body.name.trim().is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "API key name is required" }));
    }

    // Generate random API key: bm_<32 hex chars>
    let raw_key = format!("bm_{}", uuid::Uuid::new_v4().to_string().replace("-", ""));
    let key_prefix = raw_key[..11].to_string(); // "bm_" + 8 hex chars
    let key_hash = bcrypt::hash(&raw_key, 10).unwrap_or_default();

    let permissions = body.permissions.clone().unwrap_or_else(|| vec!["search".to_string(), "read_book".to_string()]);
    let perms_json = serde_json::to_string(&permissions).unwrap_or_else(|_| "[\"search\"]".to_string());
    let rate_limit = body.rate_limit.unwrap_or(30).min(100).max(1);

    let expires_at = body.expires_in_days.map(|days| {
        (chrono::Utc::now() + chrono::Duration::days(days)).to_rfc3339()
    });

    match data.db.create_api_key(
        claims.sub,
        &key_prefix,
        &key_hash,
        body.name.trim(),
        &perms_json,
        rate_limit,
        expires_at.as_deref(),
    ) {
        Ok(id) => {
            let info = ApiKeyInfo {
                id,
                key_prefix: key_prefix.clone(),
                name: body.name.trim().to_string(),
                permissions,
                rate_limit,
                is_active: true,
                last_used_at: None,
                created_at: chrono::Utc::now().to_rfc3339(),
                expires_at,
            };
            info!("API key created for user {} (prefix: {})", claims.sub, key_prefix);
            HttpResponse::Created().json(CreateApiKeyResponse {
                api_key: raw_key,
                info,
            })
        }
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

pub async fn list_api_keys(
    req: HttpRequest,
    data: web::Data<AppState>,
) -> HttpResponse {
    let claims = match auth::extract_user_from_request(&req, &data.config) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({ "error": e })),
    };

    match data.db.get_api_keys(claims.sub) {
        Ok(keys) => HttpResponse::Ok().json(keys),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

pub async fn revoke_api_key(
    req: HttpRequest,
    data: web::Data<AppState>,
    path: web::Path<i64>,
) -> HttpResponse {
    let claims = match auth::extract_user_from_request(&req, &data.config) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({ "error": e })),
    };

    let key_id = path.into_inner();
    match data.db.revoke_api_key(key_id, claims.sub) {
        Ok(true) => {
            info!("API key {} revoked by user {}", key_id, claims.sub);
            HttpResponse::Ok().json(serde_json::json!({ "message": "API key revoked" }))
        }
        Ok(false) => HttpResponse::NotFound().json(serde_json::json!({ "error": "API key not found" })),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}

/// Public API endpoint — authenticates via API key (X-API-Key header)
pub async fn api_v1_search(
    req: HttpRequest,
    data: web::Data<AppState>,
    body: web::Json<ApiQueryRequest>,
) -> HttpResponse {
    // Extract API key from header
    let api_key = match req.headers().get("X-API-Key").and_then(|v| v.to_str().ok()) {
        Some(k) => k.to_string(),
        None => {
            return HttpResponse::Unauthorized().json(serde_json::json!({
                "error": "Missing X-API-Key header",
                "docs": "https://bahtsulmasail.tech/api-docs"
            }));
        }
    };

    // Validate API key format
    if !api_key.starts_with("bm_") || api_key.len() < 11 {
        return HttpResponse::Unauthorized().json(serde_json::json!({ "error": "Invalid API key format" }));
    }

    let key_prefix = &api_key[..11];

    // Look up key in DB
    let (key_id, _user_id, key_hash, permissions, key_rate_limit) = match data.db.verify_api_key(key_prefix) {
        Ok(Some(info)) => info,
        Ok(None) => {
            return HttpResponse::Unauthorized().json(serde_json::json!({ "error": "Invalid or expired API key" }));
        }
        Err(e) => {
            return HttpResponse::InternalServerError().json(serde_json::json!({ "error": e }));
        }
    };

    // Verify key hash
    if !bcrypt::verify(&api_key, &key_hash).unwrap_or(false) {
        return HttpResponse::Unauthorized().json(serde_json::json!({ "error": "Invalid API key" }));
    }

    // Rate limit per API key
    let rl_key = format!("apikey:{}", key_prefix);
    if !data.rate_limiter.check(&rl_key) {
        return HttpResponse::TooManyRequests().json(serde_json::json!({
            "error": "API key rate limit exceeded",
            "rate_limit": key_rate_limit
        }));
    }

    // Check permissions
    let perms: Vec<String> = serde_json::from_str(&permissions).unwrap_or_default();
    if !perms.contains(&"search".to_string()) {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "API key does not have 'search' permission"
        }));
    }

    // Update last used
    let _ = data.db.update_api_key_last_used(key_id);

    let query_str = body.query.trim().to_string();
    if query_str.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": "Query cannot be empty" }));
    }

    let max_results = body.max_results.unwrap_or(10).min(50).max(1);
    let include_ai = body.include_ai.unwrap_or(true);

    // Translate and search
    let translated = data.search.translate_query(&query_str);
    let results = match data.search.search_with_translated(&translated, max_results) {
        Ok(r) => r,
        Err(e) => {
            error!("API search error: {}", e);
            return HttpResponse::InternalServerError().json(serde_json::json!({ "error": "Search failed" }));
        }
    };

    // AI synthesis (optional)
    let ai_answer = if include_ai && !results.is_empty() {
        data.ai.synthesize_answer(&query_str, &results, Some(&translated)).await
            .unwrap_or_default()
    } else {
        String::new()
    };

    HttpResponse::Ok().json(serde_json::json!({
        "query": query_str,
        "results": results,
        "ai_answer": ai_answer,
        "result_count": results.len(),
        "detected_language": format!("{:?}", translated.detected_language),
        "detected_domain": format!("{:?}", translated.detected_domain),
        "translated_terms": translated.arabic_terms,
    }))
}

// ─── Evaluation Endpoints (Academic Research) ───

/// Batch evaluation: run multiple queries with configurable ablation settings
/// POST /api/eval/batch
pub async fn eval_batch(
    req: HttpRequest,
    data: web::Data<AppState>,
    body: web::Json<EvalBatchRequest>,
) -> HttpResponse {
    // Admin-only endpoint
    let claims = match auth::extract_user_from_request(&req, &data.config) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({ "error": e })),
    };

    // Check admin role
    if claims.role != "admin" {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Evaluation endpoints require admin access"
        }));
    }

    let config = &body.config;
    let max_results = body.max_results;
    let batch_start = Instant::now();
    let mut query_results = Vec::new();

    for eq in &body.queries {
        let q_start = Instant::now();

        let (results, translated) = match data.search.search_eval(&eq.text, max_results, config) {
            Ok(r) => r,
            Err(e) => {
                error!("Eval search error for query '{}': {}", eq.text, e);
                query_results.push(EvalQueryResult {
                    query_id: eq.id.clone(),
                    query_text: eq.text.clone(),
                    translated_terms: vec![],
                    detected_language: "error".to_string(),
                    detected_domain: "error".to_string(),
                    results: vec![],
                    search_time_ms: q_start.elapsed().as_millis() as u64,
                    num_results: 0,
                });
                continue;
            }
        };

        let search_time_ms = q_start.elapsed().as_millis() as u64;

        query_results.push(EvalQueryResult {
            query_id: eq.id.clone(),
            query_text: eq.text.clone(),
            translated_terms: translated.arabic_terms.clone(),
            detected_language: format!("{}", translated.detected_language),
            detected_domain: format!("{}", translated.detected_domain),
            results: results.clone(),
            search_time_ms,
            num_results: results.len(),
        });
    }

    let total_time_ms = batch_start.elapsed().as_millis() as u64;
    let total_queries = query_results.len();

    HttpResponse::Ok().json(EvalBatchResponse {
        results: query_results,
        config_used: body.config.clone(),
        total_time_ms,
        total_queries,
    })
}

/// Get query log statistics
/// GET /api/eval/logs
pub async fn query_log_stats(
    req: HttpRequest,
    data: web::Data<AppState>,
) -> HttpResponse {
    // Admin-only
    let claims = match auth::extract_user_from_request(&req, &data.config) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(serde_json::json!({ "error": e })),
    };

    if claims.role != "admin" {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "error": "Evaluation endpoints require admin access"
        }));
    }

    match data.db.get_query_log_stats() {
        Ok(stats) => HttpResponse::Ok().json(stats),
        Err(e) => HttpResponse::InternalServerError().json(serde_json::json!({ "error": e })),
    }
}
