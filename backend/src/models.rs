use serde::{Deserialize, Serialize};

// ─── User ───
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub display_name: String,
    pub role: String, // "user" | "admin"
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserInfo {
    pub id: i64,
    pub email: String,
    pub display_name: String,
    pub role: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub display_name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

// ─── JWT Claims ───
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: i64,
    pub email: String,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}

// ─── Chat ───
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,   // "user" | "assistant"
    pub content: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub user_id: i64,
    pub title: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub query: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub session_id: String,
    pub results: Vec<SearchResult>,
    pub ai_answer: String,
    pub query: String,
    pub detected_language: String,
    pub detected_domain: String,
    pub translated_terms: Vec<String>,
}

// ─── Search ───
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub book_id: i64,
    pub toc_id: i64,
    pub title: String,
    pub content_snippet: String,
    pub page: String,
    pub part: String,
    pub score: f32,
    pub hierarchy: Vec<String>,
    pub book_name: String,
    pub author_name: String,
    #[serde(default)]
    pub source_type: String, // "kitab" | "produk_hukum"
    #[serde(default)]
    pub category: String,    // For produk_hukum results
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocEntry {
    pub id: i64,
    pub content: String,
    pub page: String,
    pub parent: i64,
    pub book_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookPage {
    pub id: i64,
    pub content: String,
    pub page: String,
    pub part: String,
}

// ─── Book reading ───
#[derive(Debug, Deserialize)]
pub struct BookReadRequest {
    pub book_id: i64,
    pub page: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BookReadResponse {
    pub book_id: i64,
    pub book_name: String,
    pub author_name: String,
    pub toc: Vec<TocNode>,
    pub pages: Vec<BookPage>,
    pub current_page: String,
    pub total_pages: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocNode {
    pub id: i64,
    pub content: String,
    pub page: String,
    pub parent: i64,
    pub children: Vec<TocNode>,
}

// ─── Chat History ───
#[derive(Debug, Deserialize)]
pub struct ChatHistoryRequest {
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SessionListItem {
    pub id: String,
    pub title: String,
    pub updated_at: String,
    pub message_count: usize,
}

#[derive(Debug, Deserialize)]
pub struct RenameSessionRequest {
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct DeleteSessionsRequest {
    pub session_ids: Vec<String>,
}

// ─── API Keys (Task 11) ───
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: i64,
    pub user_id: i64,
    pub key_prefix: String,     // first 8 chars for display (bm_xxxx...)
    pub key_hash: String,       // bcrypt hash of full key
    pub name: String,           // user-given label
    pub permissions: String,    // JSON: ["search","read_book"]
    pub rate_limit: i64,        // requests per minute
    pub is_active: bool,
    pub last_used_at: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyInfo {
    pub id: i64,
    pub key_prefix: String,
    pub name: String,
    pub permissions: Vec<String>,
    pub rate_limit: i64,
    pub is_active: bool,
    pub last_used_at: Option<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub permissions: Option<Vec<String>>,
    pub rate_limit: Option<i64>,
    pub expires_in_days: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub api_key: String, // full key, shown only once
    pub info: ApiKeyInfo,
}

#[derive(Debug, Deserialize)]
pub struct ApiQueryRequest {
    pub query: String,
    pub max_results: Option<usize>,
    pub include_ai: Option<bool>,
}

// ─── Evaluation Framework (Journal Q1/Q2) ───

/// Configuration for ablation study experiments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvalConfig {
    /// Disable large-book penalty (search.rs size_factor)
    #[serde(default)]
    pub disable_book_penalty: bool,
    /// Disable hierarchy depth boost
    #[serde(default)]
    pub disable_hierarchy_boost: bool,
    /// Disable parent relevance boost
    #[serde(default)]
    pub disable_parent_boost: bool,
    /// Disable per-book diversity cap (MAX_PER_BOOK)
    #[serde(default)]
    pub disable_diversity_cap: bool,
    /// Disable query translation entirely (raw query → Tantivy)
    #[serde(default)]
    pub disable_query_translation: bool,
    /// Disable multi-word phrase mapping in query_translator
    #[serde(default)]
    pub disable_phrase_mapping: bool,
    /// Disable multi-variant term expansion (single Arabic term per concept)
    #[serde(default)]
    pub disable_multi_variant: bool,
    /// Enable Arabic stemmer/normalizer for query expansion
    #[serde(default)]
    pub enable_arabic_stemmer: bool,
    /// Raw BM25 only — no custom scoring adjustments at all
    #[serde(default)]
    pub raw_bm25_only: bool,
}

impl Default for EvalConfig {
    fn default() -> Self {
        EvalConfig {
            disable_book_penalty: false,
            disable_hierarchy_boost: false,
            disable_parent_boost: false,
            disable_diversity_cap: false,
            disable_query_translation: false,
            disable_phrase_mapping: false,
            disable_multi_variant: false,
            enable_arabic_stemmer: false,
            raw_bm25_only: false,
        }
    }
}

/// Batch evaluation request
#[derive(Debug, Deserialize)]
pub struct EvalBatchRequest {
    pub queries: Vec<EvalQuery>,
    #[serde(default)]
    pub config: EvalConfig,
    #[serde(default = "default_eval_max_results")]
    pub max_results: usize,
}

fn default_eval_max_results() -> usize {
    20
}

#[derive(Debug, Deserialize)]
pub struct EvalQuery {
    pub id: String,
    pub text: String,
}

/// Result for a single query in batch evaluation
#[derive(Debug, Serialize)]
pub struct EvalQueryResult {
    pub query_id: String,
    pub query_text: String,
    pub translated_terms: Vec<String>,
    pub detected_language: String,
    pub detected_domain: String,
    pub results: Vec<SearchResult>,
    pub search_time_ms: u64,
    pub num_results: usize,
}

/// Batch evaluation response
#[derive(Debug, Serialize)]
pub struct EvalBatchResponse {
    pub results: Vec<EvalQueryResult>,
    pub config_used: EvalConfig,
    pub total_time_ms: u64,
    pub total_queries: usize,
}

/// Query log entry for research analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryLog {
    pub id: i64,
    pub query_text: String,
    pub detected_language: String,
    pub detected_domain: String,
    pub arabic_terms: String,        // JSON array
    pub num_results: i32,
    pub top_score: f32,
    pub search_time_ms: i64,
    pub session_id: Option<String>,
    pub created_at: String,
}

/// Query log statistics
#[derive(Debug, Serialize)]
pub struct QueryLogStats {
    pub total_queries: i64,
    pub unique_queries: i64,
    pub avg_results: f64,
    pub avg_search_time_ms: f64,
    pub language_distribution: Vec<(String, i64)>,
    pub domain_distribution: Vec<(String, i64)>,
    pub queries_per_day: Vec<(String, i64)>,
    pub zero_result_queries: i64,
    pub top_queries: Vec<(String, i64)>,
}
