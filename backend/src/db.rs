use crate::config::Config;
use crate::models::*;
use log::info;
use parking_lot::{Mutex, RwLock};
use rusqlite::{params, Connection, OpenFlags};
use std::collections::HashMap;
use std::sync::Arc;

/// Lightweight Arabic text normalization for snippet matching.
/// Strips diacritics and normalizes common letter variants so that
/// query terms (which are already normalized) can match raw Arabic text.
fn normalize_arabic_light(s: &str) -> String {
    s.chars()
        .filter(|c| {
            // Skip tashkeel (combining marks)
            !matches!(*c, '\u{064B}'..='\u{065F}' | '\u{0670}' | '\u{0640}')
        })
        .map(|c| match c {
            '\u{0623}' | '\u{0625}' | '\u{0622}' | '\u{0671}' => '\u{0627}', // أإآٱ→ا
            '\u{0649}' => '\u{064A}', // ى→ي
            _ => c,
        })
        .collect()
}

/// Book metadata extracted from page 1 content
#[derive(Debug, Clone, Default)]
pub struct BookMetadata {
    pub book_name: String,
    pub author_name: String,
}

pub struct Database {
    conn: Mutex<Connection>,
    book_ids: Vec<i64>,
    book_metadata: RwLock<HashMap<i64, BookMetadata>>,
}

impl Database {
    pub fn new(config: &Config) -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        let conn = Connection::open_with_flags(
            &config.database_path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        )?;

        // Performance pragmas
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA cache_size = -64000;
             PRAGMA mmap_size = 2147483648;
             PRAGMA temp_store = MEMORY;
             PRAGMA busy_timeout = 30000;",
        )?;

        // Create users table (with new enterprise fields)
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                email TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                display_name TEXT DEFAULT '',
                role TEXT DEFAULT 'user',
                is_active INTEGER DEFAULT 1,
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now'))
            );
            CREATE TABLE IF NOT EXISTS chat_sessions (
                id TEXT PRIMARY KEY,
                user_id INTEGER NOT NULL,
                title TEXT DEFAULT '',
                messages TEXT DEFAULT '[]',
                created_at TEXT DEFAULT (datetime('now')),
                updated_at TEXT DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(id)
            );
            CREATE TABLE IF NOT EXISTS api_keys (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                key_prefix TEXT NOT NULL,
                key_hash TEXT NOT NULL,
                name TEXT NOT NULL DEFAULT 'Default',
                permissions TEXT DEFAULT '[\"search\",\"read_book\"]',
                rate_limit INTEGER DEFAULT 30,
                is_active INTEGER DEFAULT 1,
                last_used_at TEXT,
                created_at TEXT DEFAULT (datetime('now')),
                expires_at TEXT,
                FOREIGN KEY (user_id) REFERENCES users(id)
            );
            CREATE TABLE IF NOT EXISTS query_logs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                query_text TEXT NOT NULL,
                detected_language TEXT DEFAULT '',
                detected_domain TEXT DEFAULT '',
                arabic_terms TEXT DEFAULT '[]',
                num_results INTEGER DEFAULT 0,
                top_score REAL DEFAULT 0.0,
                search_time_ms INTEGER DEFAULT 0,
                session_id TEXT,
                created_at TEXT DEFAULT (datetime('now'))
            );
            CREATE INDEX IF NOT EXISTS idx_query_logs_created ON query_logs(created_at);
            CREATE INDEX IF NOT EXISTS idx_query_logs_language ON query_logs(detected_language);
            CREATE INDEX IF NOT EXISTS idx_query_logs_domain ON query_logs(detected_domain);",
        )?;

        // Migrate existing users table — add new columns if missing
        // Safe: ALTER TABLE ADD COLUMN is a no-op if column already exists (we catch the error)
        let migration_columns = [
            "ALTER TABLE users ADD COLUMN display_name TEXT DEFAULT ''",
            "ALTER TABLE users ADD COLUMN role TEXT DEFAULT 'user'",
            "ALTER TABLE users ADD COLUMN is_active INTEGER DEFAULT 1",
            "ALTER TABLE users ADD COLUMN updated_at TEXT",
        ];
        for sql in &migration_columns {
            match conn.execute(sql, []) {
                Ok(_) => info!("Migration applied: {}", sql),
                Err(e) => {
                    let err_str = e.to_string();
                    if !err_str.contains("duplicate column") {
                        log::warn!("Migration warning: {} — {}", sql, err_str);
                    }
                }
            }
        }

        // Backfill updated_at for existing rows
        let _ = conn.execute(
            "UPDATE users SET updated_at = created_at WHERE updated_at IS NULL",
            [],
        );

        // Discover all book IDs
        let book_ids: Vec<i64> = {
            let mut stmt = conn.prepare(
                "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 't%' ORDER BY name",
            )?;
            let ids: Vec<i64> = stmt
                .query_map([], |row| {
                    let name: String = row.get(0)?;
                    Ok(name)
                })?
                .filter_map(|r| r.ok())
                .filter_map(|name| name[1..].parse::<i64>().ok())
                .collect();
            ids
        };

        info!("Discovered {} books in database", book_ids.len());

        let db = Arc::new(Database {
            conn: Mutex::new(conn),
            book_ids,
            book_metadata: RwLock::new(HashMap::new()),
        });

        // Load book metadata in background (non-blocking for startup)
        let db_clone = db.clone();
        std::thread::spawn(move || {
            db_clone.load_all_book_metadata();
        });

        Ok(db)
    }

    pub fn get_book_ids(&self) -> &[i64] {
        &self.book_ids
    }

    /// Load metadata (book name + author) for all books from first page content
    pub fn load_all_book_metadata(&self) {
        info!("Loading book metadata for {} books...", self.book_ids.len());
        let conn = self.conn.lock();
        let mut loaded = 0u32;
        let mut metadata_map = HashMap::new();

        for &book_id in &self.book_ids {
            let table = format!("b{}", book_id);
            // Get first 3 pages content to extract title/author
            let sql = format!(
                "SELECT content FROM \"{}\" WHERE (is_deleted = '0' OR is_deleted IS NULL) ORDER BY id LIMIT 3",
                table
            );

            if let Ok(mut stmt) = conn.prepare(&sql) {
                let contents: Vec<String> = match stmt.query_map([], |row| row.get::<_, String>(0)) {
                    Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
                    Err(_) => Vec::new(),
                };

                let combined = contents.join("\n");
                let meta = extract_book_metadata_from_content(&combined, book_id);
                metadata_map.insert(book_id, meta);
                loaded += 1;
            }

            if loaded % 1000 == 0 && loaded > 0 {
                info!("Book metadata loaded: {}/{}", loaded, self.book_ids.len());
            }
        }

        // Also try to get name from first TOC root entry
        for &book_id in &self.book_ids {
            if let Some(meta) = metadata_map.get_mut(&book_id) {
                if meta.book_name.is_empty() || meta.book_name == format!("كتاب {}", book_id) {
                    // Try TOC root
                    let toc_table = format!("t{}", book_id);
                    let sql = format!(
                        "SELECT content FROM \"{}\" WHERE (parent = '0' OR parent = 0) AND (is_deleted = '0' OR is_deleted IS NULL) ORDER BY id LIMIT 1",
                        toc_table
                    );
                    if let Ok(title) = conn.query_row(&sql, [], |row| row.get::<_, String>(0)) {
                        let clean = strip_html(&title);
                        if !clean.is_empty() && clean.len() > 3 && !is_garbage_book_name(&clean) {
                            meta.book_name = clean;
                        }
                    }
                }
            }
        }

        drop(conn);

        let total = metadata_map.len();
        let mut meta_write = self.book_metadata.write();
        *meta_write = metadata_map;
        info!("Book metadata loaded: {} books", total);
    }

    /// Get book metadata (name, author) for a specific book
    pub fn get_book_metadata(&self, book_id: i64) -> BookMetadata {
        let meta = self.book_metadata.read();
        meta.get(&book_id).cloned().unwrap_or(BookMetadata {
            book_name: format!("كتاب {}", book_id),
            author_name: String::new(),
        })
    }

    // ─── User operations ───
    pub fn create_user(&self, email: &str, password_hash: &str, display_name: &str) -> Result<User, String> {
        let conn = self.conn.lock();
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        conn.execute(
            "INSERT INTO users (email, password_hash, display_name, role, is_active, created_at, updated_at) VALUES (?1, ?2, ?3, 'user', 1, ?4, ?4)",
            params![email, password_hash, display_name, now],
        )
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                "Email sudah terdaftar".to_string()
            } else {
                format!("Database error: {}", e)
            }
        })?;

        let id = conn.last_insert_rowid();
        Ok(User {
            id,
            email: email.to_string(),
            password_hash: password_hash.to_string(),
            display_name: display_name.to_string(),
            role: "user".to_string(),
            is_active: true,
            created_at: now.clone(),
            updated_at: now,
        })
    }

    pub fn get_user_by_email(&self, email: &str) -> Result<Option<User>, String> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, email, password_hash, 
                        COALESCE(display_name, '') as display_name,
                        COALESCE(role, 'user') as role,
                        COALESCE(is_active, 1) as is_active,
                        created_at,
                        COALESCE(updated_at, created_at) as updated_at
                 FROM users WHERE email = ?1"
            )
            .map_err(|e| e.to_string())?;

        let user = stmt
            .query_row(params![email], |row| {
                Ok(User {
                    id: row.get(0)?,
                    email: row.get(1)?,
                    password_hash: row.get(2)?,
                    display_name: row.get(3)?,
                    role: row.get(4)?,
                    is_active: row.get::<_, i32>(5)? != 0,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })
            .ok();

        Ok(user)
    }

    pub fn get_user_by_id(&self, user_id: i64) -> Result<Option<User>, String> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, email, password_hash,
                        COALESCE(display_name, '') as display_name,
                        COALESCE(role, 'user') as role,
                        COALESCE(is_active, 1) as is_active,
                        created_at,
                        COALESCE(updated_at, created_at) as updated_at
                 FROM users WHERE id = ?1"
            )
            .map_err(|e| e.to_string())?;

        let user = stmt
            .query_row(params![user_id], |row| {
                Ok(User {
                    id: row.get(0)?,
                    email: row.get(1)?,
                    password_hash: row.get(2)?,
                    display_name: row.get(3)?,
                    role: row.get(4)?,
                    is_active: row.get::<_, i32>(5)? != 0,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            })
            .ok();

        Ok(user)
    }

    pub fn update_user_profile(&self, user_id: i64, display_name: Option<&str>, email: Option<&str>) -> Result<(), String> {
        let conn = self.conn.lock();
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();

        if let Some(name) = display_name {
            conn.execute(
                "UPDATE users SET display_name = ?1, updated_at = ?2 WHERE id = ?3",
                params![name, now, user_id],
            ).map_err(|e| e.to_string())?;
        }

        if let Some(new_email) = email {
            conn.execute(
                "UPDATE users SET email = ?1, updated_at = ?2 WHERE id = ?3",
                params![new_email, now, user_id],
            ).map_err(|e| {
                if e.to_string().contains("UNIQUE") {
                    "Email sudah digunakan".to_string()
                } else {
                    format!("Database error: {}", e)
                }
            })?;
        }

        Ok(())
    }

    pub fn update_user_password(&self, user_id: i64, new_password_hash: &str) -> Result<(), String> {
        let conn = self.conn.lock();
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        conn.execute(
            "UPDATE users SET password_hash = ?1, updated_at = ?2 WHERE id = ?3",
            params![new_password_hash, now, user_id],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    // ─── Chat session operations ───
    pub fn save_session(&self, session: &ChatSession) -> Result<(), String> {
        let conn = self.conn.lock();
        let messages_json =
            serde_json::to_string(&session.messages).map_err(|e| e.to_string())?;

        conn.execute(
            "INSERT OR REPLACE INTO chat_sessions (id, user_id, title, messages, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                session.id,
                session.user_id,
                session.title,
                messages_json,
                session.created_at,
                session.updated_at,
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_session(&self, session_id: &str, user_id: i64) -> Result<Option<ChatSession>, String> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, user_id, title, messages, created_at, updated_at
                 FROM chat_sessions WHERE id = ?1 AND user_id = ?2",
            )
            .map_err(|e| e.to_string())?;

        let session = stmt
            .query_row(params![session_id, user_id], |row| {
                let messages_str: String = row.get(3)?;
                let messages: Vec<ChatMessage> =
                    serde_json::from_str(&messages_str).unwrap_or_default();
                Ok(ChatSession {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    title: row.get(2)?,
                    messages,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })
            .ok();

        Ok(session)
    }

    pub fn get_user_sessions(&self, user_id: i64) -> Result<Vec<SessionListItem>, String> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare(
                "SELECT id, title, messages, updated_at FROM chat_sessions
                 WHERE user_id = ?1 ORDER BY updated_at DESC LIMIT 50",
            )
            .map_err(|e| e.to_string())?;

        let sessions = stmt
            .query_map(params![user_id], |row| {
                let messages_str: String = row.get(2)?;
                let messages: Vec<ChatMessage> =
                    serde_json::from_str(&messages_str).unwrap_or_default();
                Ok(SessionListItem {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    updated_at: row.get(3)?,
                    message_count: messages.len(),
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        Ok(sessions)
    }

    pub fn delete_session(&self, session_id: &str, user_id: i64) -> Result<bool, String> {
        let conn = self.conn.lock();
        let affected = conn.execute(
            "DELETE FROM chat_sessions WHERE id = ?1 AND user_id = ?2",
            params![session_id, user_id],
        ).map_err(|e| e.to_string())?;
        Ok(affected > 0)
    }

    pub fn delete_sessions_batch(&self, session_ids: &[String], user_id: i64) -> Result<usize, String> {
        let conn = self.conn.lock();
        let mut deleted = 0usize;
        for sid in session_ids {
            let affected = conn.execute(
                "DELETE FROM chat_sessions WHERE id = ?1 AND user_id = ?2",
                params![sid, user_id],
            ).map_err(|e| e.to_string())?;
            deleted += affected;
        }
        Ok(deleted)
    }

    pub fn rename_session(&self, session_id: &str, user_id: i64, title: &str) -> Result<bool, String> {
        let conn = self.conn.lock();
        let now = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let affected = conn.execute(
            "UPDATE chat_sessions SET title = ?1, updated_at = ?2 WHERE id = ?3 AND user_id = ?4",
            params![title, now, session_id, user_id],
        ).map_err(|e| e.to_string())?;
        Ok(affected > 0)
    }

    // ─── Query Logging (for academic evaluation & analysis) ───

    /// Log a search query for research analysis
    pub fn log_query(
        &self,
        query_text: &str,
        detected_language: &str,
        detected_domain: &str,
        arabic_terms: &[String],
        num_results: i32,
        top_score: f32,
        search_time_ms: i64,
        session_id: Option<&str>,
    ) -> Result<(), String> {
        let conn = self.conn.lock();
        let terms_json = serde_json::to_string(arabic_terms).unwrap_or_else(|_| "[]".to_string());
        conn.execute(
            "INSERT INTO query_logs (query_text, detected_language, detected_domain, arabic_terms, num_results, top_score, search_time_ms, session_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![query_text, detected_language, detected_domain, terms_json, num_results, top_score, search_time_ms, session_id],
        ).map_err(|e| format!("Failed to log query: {}", e))?;
        Ok(())
    }

    /// Get query log statistics for research
    pub fn get_query_log_stats(&self) -> Result<QueryLogStats, String> {
        let conn = self.conn.lock();

        let total_queries: i64 = conn.query_row(
            "SELECT COUNT(*) FROM query_logs", [], |row| row.get(0)
        ).unwrap_or(0);

        let unique_queries: i64 = conn.query_row(
            "SELECT COUNT(DISTINCT query_text) FROM query_logs", [], |row| row.get(0)
        ).unwrap_or(0);

        let avg_results: f64 = conn.query_row(
            "SELECT COALESCE(AVG(num_results), 0) FROM query_logs", [], |row| row.get(0)
        ).unwrap_or(0.0);

        let avg_search_time_ms: f64 = conn.query_row(
            "SELECT COALESCE(AVG(search_time_ms), 0) FROM query_logs", [], |row| row.get(0)
        ).unwrap_or(0.0);

        let zero_result_queries: i64 = conn.query_row(
            "SELECT COUNT(*) FROM query_logs WHERE num_results = 0", [], |row| row.get(0)
        ).unwrap_or(0);

        // Language distribution
        let mut stmt = conn.prepare(
            "SELECT detected_language, COUNT(*) as cnt FROM query_logs GROUP BY detected_language ORDER BY cnt DESC"
        ).map_err(|e| e.to_string())?;
        let language_distribution: Vec<(String, i64)> = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

        // Domain distribution
        let mut stmt = conn.prepare(
            "SELECT detected_domain, COUNT(*) as cnt FROM query_logs GROUP BY detected_domain ORDER BY cnt DESC"
        ).map_err(|e| e.to_string())?;
        let domain_distribution: Vec<(String, i64)> = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

        // Queries per day (last 30 days)
        let mut stmt = conn.prepare(
            "SELECT DATE(created_at) as day, COUNT(*) as cnt FROM query_logs
             WHERE created_at >= datetime('now', '-30 days')
             GROUP BY day ORDER BY day DESC"
        ).map_err(|e| e.to_string())?;
        let queries_per_day: Vec<(String, i64)> = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

        // Top queries
        let mut stmt = conn.prepare(
            "SELECT query_text, COUNT(*) as cnt FROM query_logs GROUP BY query_text ORDER BY cnt DESC LIMIT 20"
        ).map_err(|e| e.to_string())?;
        let top_queries: Vec<(String, i64)> = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        }).map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

        Ok(QueryLogStats {
            total_queries,
            unique_queries,
            avg_results,
            avg_search_time_ms,
            language_distribution,
            domain_distribution,
            queries_per_day,
            zero_result_queries,
            top_queries,
        })
    }

    // ─── TOC operations ───

    /// Get the count of TOC entries for a book (efficient SQL COUNT)
    pub fn get_toc_count(&self, book_id: i64) -> Result<usize, String> {
        let conn = self.conn.lock();
        let table = format!("t{}", book_id);
        let sql = format!(
            "SELECT COUNT(*) FROM \"{}\" WHERE is_deleted = '0' OR is_deleted IS NULL",
            table
        );
        conn.query_row(&sql, [], |row| row.get::<_, i64>(0))
            .map(|c| c as usize)
            .map_err(|e| e.to_string())
    }

    pub fn get_toc_entries(&self, book_id: i64) -> Result<Vec<TocEntry>, String> {
        let conn = self.conn.lock();
        let table = format!("t{}", book_id);
        let sql = format!(
            "SELECT id, content, page, parent FROM \"{}\" WHERE is_deleted = '0' OR is_deleted IS NULL ORDER BY id",
            table
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;

        let entries = stmt
            .query_map([], |row| {
                let parent_str: String = row.get::<_, String>(3).unwrap_or_default();
                let parent = parent_str.parse::<i64>().unwrap_or(0);
                Ok(TocEntry {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    page: row.get(2)?,
                    parent,
                    book_id,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        Ok(entries)
    }

    pub fn get_toc_hierarchy(&self, book_id: i64, toc_id: i64) -> Result<Vec<String>, String> {
        let entries = self.get_toc_entries(book_id)?;
        let map: HashMap<i64, &TocEntry> = entries.iter().map(|e| (e.id, e)).collect();

        let mut hierarchy = Vec::new();
        let mut current_id = toc_id;

        for _ in 0..20 {
            // max depth protection
            if let Some(entry) = map.get(&current_id) {
                hierarchy.push(entry.content.clone());
                if entry.parent == 0 {
                    break;
                }
                current_id = entry.parent;
            } else {
                break;
            }
        }

        hierarchy.reverse();
        Ok(hierarchy)
    }

    pub fn build_toc_tree(&self, book_id: i64) -> Result<Vec<TocNode>, String> {
        let entries = self.get_toc_entries(book_id)?;
        let mut nodes: Vec<TocNode> = entries
            .iter()
            .map(|e| TocNode {
                id: e.id,
                content: e.content.clone(),
                page: e.page.clone(),
                parent: e.parent,
                children: Vec::new(),
            })
            .collect();

        // Build tree bottom-up
        let mut children_map: HashMap<i64, Vec<TocNode>> = HashMap::new();
        // Sort by ID descending to process children first
        nodes.sort_by(|a, b| b.id.cmp(&a.id));

        for node in nodes {
            let mut n = node.clone();
            if let Some(children) = children_map.remove(&n.id) {
                n.children = children;
                n.children.sort_by(|a, b| a.id.cmp(&b.id));
            }
            children_map
                .entry(node.parent)
                .or_default()
                .push(n);
        }

        let mut roots = children_map.remove(&0).unwrap_or_default();
        roots.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(roots)
    }

    // ─── Book content ───
    pub fn get_book_pages(
        &self,
        book_id: i64,
        page: Option<&str>,
    ) -> Result<Vec<BookPage>, String> {
        self.get_book_pages_with_row_id(book_id, page, None)
    }

    /// Fetch book pages. When `row_id` is provided, fetches rows around that
    /// row ID anchor (±margin) for precise navigation from search results.
    /// Falls back to display page matching when only `page` is provided.
    pub fn get_book_pages_with_row_id(
        &self,
        book_id: i64,
        page: Option<&str>,
        row_id: Option<i64>,
    ) -> Result<Vec<BookPage>, String> {
        let conn = self.conn.lock();
        let table = format!("b{}", book_id);

        // If row_id is provided, fetch rows around that anchor
        if let Some(rid) = row_id {
            let margin_back = 2i64;
            let margin_forward = 25i64;
            let start = (rid - margin_back).max(1);
            let end = rid + margin_forward;
            let sql = format!(
                "SELECT id, content, page, COALESCE(part, '') FROM \"{}\"
                 WHERE id >= ?1 AND id <= ?2 AND (is_deleted = '0' OR is_deleted IS NULL)
                 ORDER BY id",
                table
            );
            let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
            let pages: Vec<BookPage> = stmt
                .query_map(params![start, end], |row| {
                    Ok(BookPage {
                        id: row.get(0)?,
                        content: row.get(1)?,
                        page: row.get(2)?,
                        part: row.get(3)?,
                    })
                })
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            return Ok(pages);
        }

        let (sql, page_val);
        if let Some(p) = page {
            sql = format!(
                "SELECT id, content, page, COALESCE(part, '') FROM \"{}\"
                 WHERE page = ?1 AND (is_deleted = '0' OR is_deleted IS NULL)
                 ORDER BY id",
                table
            );
            page_val = Some(p.to_string());
        } else {
            sql = format!(
                "SELECT id, content, page, COALESCE(part, '') FROM \"{}\"
                 WHERE (is_deleted = '0' OR is_deleted IS NULL)
                 ORDER BY id LIMIT 5",
                table
            );
            page_val = None;
        }

        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let pages = if let Some(ref pv) = page_val {
            stmt.query_map(params![pv], |row| {
                Ok(BookPage {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    page: row.get(2)?,
                    part: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect()
        } else {
            stmt.query_map([], |row| {
                Ok(BookPage {
                    id: row.get(0)?,
                    content: row.get(1)?,
                    page: row.get(2)?,
                    part: row.get(3)?,
                })
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect()
        };

        Ok(pages)
    }

    pub fn get_total_pages(&self, book_id: i64) -> Result<i64, String> {
        let conn = self.conn.lock();
        let table = format!("b{}", book_id);
        let sql = format!("SELECT COUNT(DISTINCT page) FROM \"{}\"", table);
        conn.query_row(&sql, [], |row| row.get(0))
            .map_err(|e| e.to_string())
    }

    pub fn get_content_snippet(&self, book_id: i64, page: &str) -> Result<String, String> {
        // In the database design: t{N}.page = row ID in b{N} (NOT the display page column)
        // The page stored in Tantivy = toc.page = b.id
        let row_id: i64 = page.trim().parse().unwrap_or(0);
        if row_id == 0 {
            log::warn!("Snippet: book_id={} page='{}' → row_id=0, skip", book_id, page);
            return Ok(String::new());
        }
        let conn = self.conn.lock();
        let table = format!("b{}", book_id);
        // Get this row and up to 4 subsequent rows to capture real chapter content
        let sql = format!(
            "SELECT content FROM \"{}\" WHERE id >= ?1 AND id <= ?2 AND (is_deleted = '0' OR is_deleted IS NULL) ORDER BY id LIMIT 5",
            table
        );
        let rows: Vec<String> = match conn.prepare(&sql) {
            Ok(mut stmt) => {
                match stmt.query_map(params![row_id, row_id + 4], |row| row.get::<_, String>(0)) {
                    Ok(mapped) => mapped.filter_map(|r| r.ok()).collect(),
                    Err(e) => {
                        log::warn!("Snippet query_map error: book_id={} row_id={} err={}", book_id, row_id, e);
                        Vec::new()
                    }
                }
            }
            Err(e) => {
                log::warn!("Snippet prepare error: book_id={} table={} err={}", book_id, table, e);
                Vec::new()
            }
        };

        let combined = rows.join(" ");
        let plain = strip_html(&combined);

        log::debug!("Snippet: book_id={} row_id={} rows={} combined_len={} plain_len={}", 
            book_id, row_id, rows.len(), combined.len(), plain.trim().chars().count());

        // Too short to be useful
        if plain.trim().chars().count() < 5 {
            log::warn!("Snippet too short: book_id={} row_id={} plain_chars={}", 
                book_id, row_id, plain.trim().chars().count());
            return Ok(String::new());
        }

        // Truncate safely (char boundary-aware for Arabic)
        let snippet = if plain.chars().count() > 400 {
            let truncated: String = plain.chars().take(400).collect();
            format!("{}...", truncated)
        } else {
            plain
        };
        Ok(snippet)
    }

    /// Get content snippet AND the display page number.
    /// Returns (snippet_text, display_page_string).
    /// The `toc_page` parameter is the TOC's page field (= b{N}.id row ID).
    /// We look up b{N}.page to get the actual display page number.
    pub fn get_content_snippet_with_page(&self, book_id: i64, toc_page: &str) -> Result<(String, String), String> {
        self.get_content_snippet_with_page_and_terms(book_id, toc_page, &[])
    }

    /// Enhanced snippet extraction: fetches content rows and finds the window
    /// that best matches the given Arabic search terms.
    pub fn get_content_snippet_with_page_and_terms(
        &self,
        book_id: i64,
        toc_page: &str,
        arabic_terms: &[String],
    ) -> Result<(String, String), String> {
        let row_id: i64 = toc_page.trim().parse().unwrap_or(0);
        if row_id == 0 {
            return Ok((String::new(), String::new()));
        }
        let conn = self.conn.lock();
        let table = format!("b{}", book_id);
        
        // Fetch a wider range to find relevant content
        // Forward-biased: TOC entries mark chapter START, so content is ahead
        let fetch_forward = if arabic_terms.is_empty() { 7 } else { 20 };
        let back_rows = if arabic_terms.is_empty() { 0i64 } else { 2 };
        let start_id = (row_id - back_rows).max(1);
        let end_id = row_id + fetch_forward as i64;
        let total_fetch = (fetch_forward + 1 + back_rows as usize) as usize;
        let sql = format!(
            "SELECT id, content, page FROM \"{}\" WHERE id >= ?1 AND id <= ?2 AND (is_deleted = '0' OR is_deleted IS NULL) ORDER BY id LIMIT {}",
            table, total_fetch
        );
        
        let rows: Vec<(i64, String, String)> = match conn.prepare(&sql) {
            Ok(mut stmt) => {
                match stmt.query_map(params![start_id, end_id], |row| {
                    Ok((
                        row.get::<_, i64>(0).unwrap_or(0),
                        row.get::<_, String>(1).unwrap_or_default(),
                        row.get::<_, String>(2).unwrap_or_default(),
                    ))
                }) {
                    Ok(mapped) => mapped.filter_map(|r| r.ok()).collect(),
                    Err(e) => {
                        log::warn!("SnippetPage query_map error: book_id={} row_id={} err={}", book_id, row_id, e);
                        Vec::new()
                    }
                }
            }
            Err(e) => {
                log::warn!("SnippetPage prepare error: book_id={} table={} err={}", book_id, table, e);
                Vec::new()
            }
        };

        if rows.is_empty() {
            return Ok((String::new(), String::new()));
        }

        // Find the index of the anchor row (row_id) in our fetched rows
        let anchor_idx = rows.iter().position(|(id, _, _)| *id == row_id).unwrap_or(0);

        // Find the display page at the anchor row
        let anchor_display_page = rows.get(anchor_idx)
            .map(|(_, _, p)| p.clone())
            .unwrap_or_else(|| rows[0].2.clone());

        // If we have search terms, find the best window of ~8 rows that contains the most matches
        let window_size = 8.min(rows.len());
        let (snippet_text, best_page) = if !arabic_terms.is_empty() && rows.len() > window_size {
            let mut best_score = 0usize;
            // Default to starting at anchor row (not before it)
            let mut best_start = anchor_idx.min(rows.len().saturating_sub(window_size));
            
            for start in 0..=(rows.len() - window_size) {
                let window_text: String = rows[start..start + window_size]
                    .iter()
                    .map(|(_, c, _)| strip_html(c))
                    .collect::<Vec<_>>()
                    .join(" ");
                let normalized_window = normalize_arabic_light(&window_text);
                
                let score: usize = arabic_terms.iter()
                    .map(|term| {
                        let normalized_term = normalize_arabic_light(term);
                        if term.contains(' ') {
                            // Phrase match counts 3×
                            if normalized_window.contains(&normalized_term) { 3 } else { 0 }
                        } else {
                            if normalized_window.contains(&normalized_term) { 1 } else { 0 }
                        }
                    })
                    .sum();
                
                if score > best_score {
                    best_score = score;
                    best_start = start;
                }
            }
            
            let best_window: String = rows[best_start..best_start + window_size]
                .iter()
                .map(|(_, c, _)| c.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            let best_pg = rows[best_start].2.clone();
            (best_window, best_pg)
        } else {
            // No search terms or too few rows: start from anchor row forward
            let start = anchor_idx;
            let end = (start + window_size).min(rows.len());
            let combined: String = rows[start..end].iter().map(|(_, c, _)| c.as_str()).collect::<Vec<_>>().join(" ");
            (combined, anchor_display_page.clone())
        };

        let plain = strip_html(&snippet_text);

        if plain.trim().chars().count() < 5 {
            return Ok((String::new(), if best_page.is_empty() { anchor_display_page } else { best_page }));
        }

        // Truncate at Arabic sentence boundaries when possible (600 chars)
        let snippet = if plain.chars().count() > 1200 {
            let truncated: String = plain.chars().take(1200).collect();
            // Try to break at last Arabic sentence-ending punctuation (. or ، or :)
            if let Some(last_break) = truncated.rfind(|c: char| c == '.' || c == '\u{06D4}' || c == ':' || c == '\n') {
                if last_break > 600 {
                    format!("{}...", &truncated[..last_break + 1])
                } else {
                    format!("{}...", truncated)
                }
            } else {
                format!("{}...", truncated)
            }
        } else {
            plain
        };
        Ok((snippet, if best_page.is_empty() { anchor_display_page } else { best_page }))
    }

    // ─── FTS5 fallback search on TOC ───
    pub fn search_toc_fts(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, String> {
        let conn = self.conn.lock();
        let mut results = Vec::new();

        // Search across all TOC tables using LIKE (FTS5 would be ideal but tables are individual)
        let search_term = format!("%{}%", query);

        for &book_id in &self.book_ids {
            if results.len() >= limit {
                break;
            }
            let table = format!("t{}", book_id);
            let sql = format!(
                "SELECT id, content, page, parent FROM \"{}\"
                 WHERE content LIKE ?1 AND (is_deleted = '0' OR is_deleted IS NULL)
                 LIMIT 5",
                table
            );

            if let Ok(mut stmt) = conn.prepare(&sql) {
                if let Ok(rows) = stmt.query_map(params![&search_term], |row| {
                    let parent_str: String = row.get::<_, String>(3).unwrap_or_default();
                    let parent = parent_str.parse::<i64>().unwrap_or(0);
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        parent,
                    ))
                }) {
                    for row in rows.flatten() {
                        let meta = self.get_book_metadata(book_id);
                        results.push(SearchResult {
                            book_id,
                            toc_id: row.0,
                            title: row.1.clone(),
                            content_snippet: String::new(),
                            page: row.2.clone(),
                            part: String::new(),
                            score: 50.0, // base score for FTS
                            hierarchy: Vec::new(),
                            book_name: meta.book_name,
                            author_name: meta.author_name,
                            source_type: "kitab".to_string(),
                            category: String::new(),
                            citation: String::new(),
                            similarity_score: 0.0,
                            toc_page: row.2,
                        });
                        if results.len() >= limit {
                            break;
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    // ─── API Keys (Task 11) ───
    pub fn create_api_key(
        &self,
        user_id: i64,
        key_prefix: &str,
        key_hash: &str,
        name: &str,
        permissions: &str,
        rate_limit: i64,
        expires_at: Option<&str>,
    ) -> Result<i64, String> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO api_keys (user_id, key_prefix, key_hash, name, permissions, rate_limit, expires_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![user_id, key_prefix, key_hash, name, permissions, rate_limit, expires_at],
        )
        .map_err(|e| format!("Failed to create API key: {}", e))?;
        Ok(conn.last_insert_rowid())
    }

    pub fn get_api_keys(&self, user_id: i64) -> Result<Vec<crate::models::ApiKeyInfo>, String> {
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT id, key_prefix, name, permissions, rate_limit, is_active, last_used_at, created_at, expires_at FROM api_keys WHERE user_id = ?1 ORDER BY created_at DESC")
            .map_err(|e| format!("Prepare error: {}", e))?;
        let keys = stmt
            .query_map(params![user_id], |row| {
                let perms_str: String = row.get(3)?;
                let perms: Vec<String> = serde_json::from_str(&perms_str).unwrap_or_default();
                Ok(crate::models::ApiKeyInfo {
                    id: row.get(0)?,
                    key_prefix: row.get(1)?,
                    name: row.get(2)?,
                    permissions: perms,
                    rate_limit: row.get(4)?,
                    is_active: row.get::<_, i64>(5)? == 1,
                    last_used_at: row.get(6)?,
                    created_at: row.get(7)?,
                    expires_at: row.get(8)?,
                })
            })
            .map_err(|e| format!("Query error: {}", e))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(keys)
    }

    pub fn verify_api_key(&self, key_prefix: &str) -> Result<Option<(i64, i64, String, String, i64)>, String> {
        // Returns (key_id, user_id, key_hash, permissions, rate_limit)
        let conn = self.conn.lock();
        let mut stmt = conn
            .prepare("SELECT id, user_id, key_hash, permissions, rate_limit, expires_at FROM api_keys WHERE key_prefix = ?1 AND is_active = 1")
            .map_err(|e| format!("Prepare error: {}", e))?;
        let result = stmt
            .query_row(params![key_prefix], |row| {
                let expires: Option<String> = row.get(5)?;
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    expires,
                ))
            })
            .ok();

        if let Some((id, user_id, hash, perms, rate_lim, expires)) = result {
            // Check expiry
            if let Some(exp) = expires {
                if let Ok(exp_time) = chrono::DateTime::parse_from_rfc3339(&exp) {
                    if exp_time < chrono::Utc::now() {
                        return Ok(None);
                    }
                }
            }
            Ok(Some((id, user_id, hash, perms, rate_lim)))
        } else {
            Ok(None)
        }
    }

    pub fn update_api_key_last_used(&self, key_id: i64) -> Result<(), String> {
        let conn = self.conn.lock();
        conn.execute(
            "UPDATE api_keys SET last_used_at = datetime('now') WHERE id = ?1",
            params![key_id],
        )
        .map_err(|e| format!("Update error: {}", e))?;
        Ok(())
    }

    pub fn revoke_api_key(&self, key_id: i64, user_id: i64) -> Result<bool, String> {
        let conn = self.conn.lock();
        let affected = conn
            .execute(
                "UPDATE api_keys SET is_active = 0 WHERE id = ?1 AND user_id = ?2",
                params![key_id, user_id],
            )
            .map_err(|e| format!("Revoke error: {}", e))?;
        Ok(affected > 0)
    }

    pub fn cleanup_old_sessions(&self, days: i64) -> Result<usize, String> {
        let conn = self.conn.lock();
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
        let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();
        conn.execute(
            "DELETE FROM chat_sessions WHERE updated_at < ?1",
            params![cutoff_str],
        )
        .map_err(|e| format!("Session cleanup error: {}", e))
    }

    pub fn cleanup_old_query_logs(&self, days: i64) -> Result<usize, String> {
        let conn = self.conn.lock();
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days);
        let cutoff_str = cutoff.format("%Y-%m-%d %H:%M:%S").to_string();
        conn.execute(
            "DELETE FROM query_logs WHERE created_at < ?1",
            params![cutoff_str],
        )
        .map_err(|e| format!("Query log cleanup error: {}", e))
    }

    pub fn get_result_citation(&self, book_id: i64, toc_id: i64) -> Result<String, String> {
        let meta = self.get_book_metadata(book_id);
        let conn = self.conn.lock();
        let page: String = conn
            .query_row(
                &format!("SELECT COALESCE(page, '') FROM t{} WHERE id = ?1", book_id),
                params![toc_id],
                |row| row.get(0),
            )
            .unwrap_or_default();
        let citation = if !meta.author_name.is_empty() {
            format!("{} - {} ص. {}", meta.book_name, meta.author_name, page)
        } else {
            format!("{} ص. {}", meta.book_name, page)
        };
        Ok(citation)
    }

    pub fn add_feedback(
        &self,
        user_id: i64,
        query_text: &str,
        result_book_id: i64,
        result_toc_id: i64,
        feedback_type: &str,
    ) -> Result<i64, String> {
        let conn = self.conn.lock();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS search_feedback (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                query_text TEXT NOT NULL,
                result_book_id INTEGER NOT NULL,
                result_toc_id INTEGER NOT NULL,
                feedback_type TEXT NOT NULL,
                created_at TEXT DEFAULT (datetime('now'))
            )",
            [],
        ).map_err(|e| format!("Create feedback table error: {}", e))?;
        conn.execute(
            "INSERT INTO search_feedback (user_id, query_text, result_book_id, result_toc_id, feedback_type) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![user_id, query_text, result_book_id, result_toc_id, feedback_type],
        ).map_err(|e| format!("Insert feedback error: {}", e))?;
        Ok(conn.last_insert_rowid())
    }
}

fn strip_html(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            '\r' => {
                if !in_tag {
                    result.push('\n');
                }
            }
            _ => {
                if !in_tag {
                    result.push(c);
                }
            }
        }
    }
    result
}

/// Strip Arabic diacritical marks (harakat/tashkil) for robust string comparison
fn strip_arabic_diacritics(text: &str) -> String {
    text.chars()
        .filter(|c| !matches!(*c,
            '\u{064B}'..='\u{065F}' | // fathah, dammah, kasrah, sukun, shadda, etc.
            '\u{0670}'              | // superscript alef
            '\u{06D6}'..='\u{06ED}'   // other Quranic marks
        ))
        .collect()
}

/// Check if a book name is garbage (generic/formula/fragment/etc.)
fn is_garbage_book_name(name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() || name.chars().count() < 4 {
        return true;
    }
    if name.chars().all(|c| c == '.' || c == ' ' || c == '*' || c == '-' || c == '_' || c == 'ـ') {
        return true;
    }
    // Strip brackets for checking (handles [مقدمات], [مقدمة الكتاب], etc.)
    let bracket_stripped = name
        .trim_matches(|c: char| c == '[' || c == ']' || c == '(' || c == ')' || c == ' ')
        .trim();
    let nc = strip_arabic_diacritics(bracket_stripped);
    // After diacritics stripping, re-check length (e.g. "قَالَ" → "قال" = 3 chars)
    if nc.chars().count() < 4 {
        return true;
    }
    // Generic section names
    nc == "المقدمة" || nc == "المقدمة:" || nc == "مقدمة" || nc == "مقدمة:"
    || nc == "مقدمات" || nc == "مقدمات:" || nc == "خاتمة" || nc == "خاتمة:"
    || nc == "وفيه مسائل" || nc == "وفيه مسائل."
    || nc == "مجموعة" || nc == "ويليه"
    || nc.starts_with("المقدمة ") || nc.starts_with("مقدمة ") || nc.starts_with("مقدمات ")
    || nc.starts_with("الجزء ") || nc.starts_with("المجلد ")
    || nc.starts_with("باب ") // Chapter headings (باب الجدة, باب الصلاة)
    || nc.starts_with("فصل ") || nc.starts_with("فصل:") // Section headings
    // Metadata/source/publisher markers
    || nc.starts_with("مصدر الكتاب") || nc.starts_with("موقع")
    || nc.starts_with("http") || nc.starts_with("www")
    || nc.starts_with("تم نسخه") || nc.starts_with("نسخ من")
    || nc.starts_with("الناشر") // الناشر: دار ...
    || nc.starts_with("سلسلة ") // سلسلة منشورات مكتبة...
    || nc.starts_with("دكة ") // Event/date markers: دكة ١٠ محرم...
    // Basmala / Hamdala / Shahada / Dua formulas
    || nc.contains("بسم الله الرحمن")
    || nc.contains("واشهد ان لا اله") || nc.contains("وأشهد أن لا إله")
    || nc.starts_with("الحمد لله") || nc.starts_with("والله الموفق")
    || nc.starts_with("وبعد") || nc.starts_with("أما بعد") || nc.starts_with("أمّا بعد")
    || nc.starts_with("فإن ") || nc.starts_with("فان ")
    || nc.starts_with("اللهم ") || nc.starts_with("اللَّهم") // Dua formulas
    // Author/preface attribution
    || nc.starts_with("المؤلف") || nc.starts_with("تأليف") || nc.starts_with("تصنيف")
    || nc.starts_with("تقديم") // تقديم للأستاذ الدكتور...
    // Quranic markers
    || nc.starts_with('{') || nc.starts_with('﴿')
    || (nc.starts_with('(') && nc.len() > 50)
    || (nc.starts_with('(') && nc.contains("سورة"))
    // Sentence-continuation fragments
    || nc.starts_with("وهذه ") || nc.starts_with("ثم ")
    || nc.starts_with("ولذا ") || nc.starts_with("وقد ")
    || nc.starts_with("ولقد ") || nc.starts_with("ولما ")
    || nc.starts_with("فلما ") || nc.starts_with("ولكن ")
    || nc.starts_with("هذه ") || nc.starts_with("هذا ")
    || nc.starts_with("أما ") // أما بقية..., conditional constructions
    || nc.starts_with("ولا بد") // ولا بد لي من أن أكون...
    || nc.starts_with("والسير ") // والسير مراد به هنا...
    // Commentary/definition markers
    || nc.starts_with("قوله") // قوله: ربه] الرب قيل... 
    || nc.starts_with("ومعنى") // ومعنى: "أقطع": قليل البركة...
    || nc.starts_with("في الاصطلاح") // في الاصطلاح هنا: العلم بقسمة...
    // Attribution sentences
    || nc.starts_with("قال ") || nc.starts_with("وقال ")
    || nc.starts_with("انظر") // انظر: تاريخ الإمامة... (footnote cross-reference)
    || nc.starts_with("من الحنفية") || nc.starts_with("من المالكية")
    || nc.starts_with("من الشافعية") || nc.starts_with("من الحنابلة")
    // Job/position descriptions
    || nc.starts_with("الوظيفة") // الوظيفة: إمام مسجد (سابقا)
    || nc.starts_with("العلامة ") // العلامة محمد بن إسماعيل (author, not title)
    // Shalawat formulas (all variants)
    || nc.contains("وصلى الله على") || nc.contains("صلى الله عليه")
    || nc.contains("وصلى الله وسلم") // variant: وصلى الله وسلم وبارك
    // Biographical text
    || nc.starts_with("من سكان") || nc.starts_with("حاصل على")
    || nc.starts_with("ذكر نسب")
    // Edition/print metadata
    || nc.starts_with("الطبعة") // الطبعة: الرابعة
    || nc.starts_with("عدد الأجزاء") || nc.starts_with("عدد الصفحات") // book metadata
    || nc.starts_with("البريد") // البريد الإلكتروني: ...
    || nc.contains("@") // Email addresses
    // Hadith narration markers
    || nc.starts_with("وفي رواية") || nc.starts_with("في رواية")
    // Government/institutional titles
    || nc.starts_with("وزير ") // وزير الأوقاف
    || nc.starts_with("السيد ") // السيد محمد (personal title prefix)
    // Commentary/editorial markers  
    || nc.starts_with("والجواب ") // والجواب الرابع: أن المراد...
    || nc.starts_with("والتعدية") // والتعدية: وتسمى باء الفعل...
    || nc.starts_with("والاستعانة") // والاستعانة: وقد مر...
    || nc.starts_with("الوجه ") // الوجه الأول: من قبل نفسي...
    || nc.starts_with("أحدهما") // أحدهما: أنه قد روي...
    || nc.starts_with("الثاني:") || nc.starts_with("الثاني ") // الثاني: من قبل طالبه
    || nc.starts_with("الأول:") || nc.starts_with("الأول ") // enumeration
    || nc.starts_with("حققه") || nc.starts_with("حقّقه") // حققه وعلق عليه...
    || nc.starts_with("خطبة ") // خطبة الكتاب, خطبة المؤلف
    // Closing formulas
    || nc.starts_with("والله من وراء") // والله من وراء القصد
    || nc.starts_with("والله أعلم") // والله أعلم بالصواب
    || nc.starts_with("تحريرا ") // تحريرا فى ١٨ من صفر... (date stamps)
    || nc.starts_with("تحريراً ")
    // Names containing Quranic verse markers in their text
    || (nc.contains('{') && nc.contains('}') && nc.chars().count() > 50)
    // Personal statements / mottos
    || nc.starts_with("لا أنتمي") // لا أنتمي لأي حزب...
    // Digit/numbering starts (chapter headings)
    || nc.chars().next().map_or(false, |c| ('\u{0660}'..='\u{0669}').contains(&c) || c.is_ascii_digit())
    // Arabic ordinals (خامسا: التفسير...)
    || nc.starts_with("أولا") || nc.starts_with("ثانيا") || nc.starts_with("ثالثا")
    || nc.starts_with("رابعا") || nc.starts_with("خامسا") || nc.starts_with("سادسا")
    || nc.starts_with("سابعا") || nc.starts_with("ثامنا") || nc.starts_with("تاسعا")
    || nc.starts_with("عاشرا")
    // Colon-ending section headers (short ones only — long titles may legitimately end with ":")
    || (nc.ends_with(':') && nc.chars().count() < 60)
    // Contains comma-relative clause pattern (sentence fragments, not titles)
    || nc.contains("، التي ") || nc.contains("، الذي ")
    || nc.contains("، التى ") // variant without dots
    || nc.contains("، وهي ") || nc.contains("، وهو ")
    || nc.contains("، هي") || nc.contains("، هى") // comma + pronoun without و
    // Contains colon-pronoun definition pattern: "المقابلة: وهي الداخلة..."
    || nc.contains(": وهي ") || nc.contains(": وهو ")
    // (NOTE: trailing period/question mark already stripped at top of function)
    // Lines containing ellipsis — poetry verses or truncated text
    || nc.contains("...") || nc.contains('\u{2026}')
    // Person names (editor/author inline): contains بن + أبو pattern
    || (nc.contains(" بن ") && nc.contains("أبو "))
    // Prepositional fragments (لي بهم..., عليه أن...)
    || nc.starts_with("لي ") || nc.starts_with("لك ")
    // More sentence fragments starting with conjunctions
    || nc.starts_with("فقد ") || nc.starts_with("فهذا ")
    || nc.starts_with("وكان ") || nc.starts_with("وكانت ")
    || nc.starts_with("ومن ") || nc.starts_with("وعن ")
    || nc.starts_with("الشيطان ") // الشيطان وتعصي الرحمن
    || nc.starts_with("وهل ") // وهل يليق... (question fragments)
    || nc.starts_with("وعملنا ") // وعملنا في الكتاب (work description)
    || nc.starts_with("وكتبه") // وكتبه ناجي سويدان (author attribution)
    || nc.starts_with("وكل ") // وكل من فارق... (sentence fragment)
    || nc.starts_with("وأصل ") // وأصل هذا الكتاب... (work description)
    || nc.starts_with("وهذا ") // وهذا الكتاب... (this book...)
    || nc.starts_with("والآي") // والآية... (sentence continuation)
    || nc.starts_with("والمقابلة ") // والمقابلة: وهي الداخلة...
    // Sentence starting with وإنما (وإنما اخترنا مذهبه...)
    || nc.starts_with("وإنما ") || nc.starts_with("وانما ")
    // Incomplete sentence markers / ordinals inside text 
    || nc.starts_with("الأولى:") || nc.starts_with("الأولى ") // الأُولى: في لفظ العقل
    || nc.starts_with("الثانية:") || nc.starts_with("الثانية ")
    || nc.starts_with("الثالثة:") || nc.starts_with("الثالثة ")
    || nc.starts_with("الرابعة:") || nc.starts_with("الرابعة ")
    || nc.starts_with("الخامسة:") || nc.starts_with("الخامسة ")
    // Government/institutional titles used as names
    || nc.starts_with("المفتي العام") // المفتي العام للمملكة العربية السعودية
    || nc.starts_with("الرئاسة العامة") // الرئاسة العامة لإدارات...
    || nc.starts_with("اللجنة ") // اللجنة الدائمة (committee, not title)
    // Bidding/dua fragment
    || nc.starts_with("وبه نستعين") // وبه نستعين (dua phrase)
    || nc.starts_with("بالله التوفيق") || nc.starts_with("والله الهادي")
    || nc.starts_with("نسأل الله") || nc.starts_with("نسال الله")
    // وَمَوْضُوع pattern (topic description, not title)
    || nc.starts_with("وموضوع ") || nc.starts_with("ومَوْضُوع")
    // More sentence fragments starting with conjunctions + verbs
    || nc.starts_with("وسيتضح ") // وسيتضح لك من خلاله عظمة... (promise sentence)
    || nc.starts_with("فالمنصف ") // فالمنصف لا يشتغل... (conditional clause)
    || nc.starts_with("فجاء ") // فجاء الكتاب بفضل الله... (narrative sentence)
    || nc.starts_with("والطهارة في ") // والطهارة في اللغة... (definition sentence)
    || nc.starts_with("وفي المغرب") // وفي المغرب فقه المعنى (cross-reference)
    || nc.starts_with("وفي ") // General: starts with "and in..."
    || nc.starts_with("فهذه ") // فهذه رسالة... (demonstrative prefix)
    || nc.starts_with("والحمد ") // والحمد لله (continuation dua)
    || nc.starts_with("والصلاة ") // والصلاة والسلام (continuation salawat)
    || nc.starts_with("فهو ") // فهو كتاب... (pronoun definition)
    || nc.starts_with("وأما ") // وأما بقية... (conditional)
    || nc.starts_with("فلا ") // فلا يجوز... (negative clause)
    || nc.starts_with("ولعل ") // ولعل هذا... (speculative)
    || nc.starts_with("فإنه ") // فإنه قال... (causal clause)
    || nc.starts_with("وبالله ") // وبالله التوفيق
    // Title that IS a description of content genre, not a book name
    || nc == "المفتي" && nc.chars().count() < 10 // "المفتي" alone is too generic
    // Long descriptions (> 80 chars — real book titles are shorter)
    || nc.chars().count() > 80
}

/// Extract book name and author from first-page content
fn extract_book_metadata_from_content(content: &str, book_id: i64) -> BookMetadata {
    let plain = strip_html(content);
    let mut book_name = String::new();
    let mut author_name = String::new();

    // Common patterns in Islamic book first pages:
    // "تأليف: NAME" or "تأليف NAME" or "للإمام NAME" or "للشيخ NAME"
    // "المؤلف: NAME" or "كتاب NAME"

    let author_markers = [
        "تأليف:", "تأليف :", "تأليف", "المؤلف:", "المؤلف :", "المؤلف",
        "للإمام", "للشيخ", "للعلامة", "للحافظ", "للفقيه",
        "تصنيف:", "تصنيف :", "تصنيف",
        "كتبه:", "كتبه", "جمعه:", "جمعه",
    ];

    let title_markers = [
        "بيانات الكتاب", "عنوان الكتاب:", "عنوان الكتاب :",
        "اسم الكتاب:", "اسم الكتاب :",
    ];

    let lines: Vec<&str> = plain.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Try to find author
        if author_name.is_empty() {
            for marker in &author_markers {
                if let Some(pos) = trimmed.find(marker) {
                    let after = trimmed[pos + marker.len()..].trim();
                    if !after.is_empty() {
                        // Take up to the next newline or 100 chars
                        let name = after.chars().take(100).collect::<String>();
                        let name = name.trim().trim_matches(|c: char| c == ':' || c == '(' || c == ')');
                        if !name.is_empty() && name.len() > 2 {
                            author_name = name.to_string();
                            break;
                        }
                    } else if i + 1 < lines.len() {
                        // Author might be on next line
                        let next = lines[i + 1].trim();
                        if !next.is_empty() && next.len() > 2 {
                            author_name = next.chars().take(100).collect();
                            break;
                        }
                    }
                }
            }
        }

        // Try to find book title
        if book_name.is_empty() {
            for marker in &title_markers {
                if let Some(pos) = trimmed.find(marker) {
                    let after = trimmed[pos + marker.len()..].trim();
                    if !after.is_empty() {
                        book_name = after.chars().take(150).collect::<String>().trim().to_string();
                        break;
                    }
                }
            }
        }
    }

    // Fallback: first non-basmala, non-intro line that looks like a title
    if book_name.is_empty() {
        for line in &lines {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.len() < 4 {
                continue;
            }

            // Strip diacritics for comparison (catches بِسْمِ اللَّهِ etc.)
            let no_dia = strip_arabic_diacritics(trimmed);

            // Skip basmala variants (with/without tashkil)
            if no_dia.contains("بسم الله") {
                continue;
            }
            // Skip hamdala variants
            if no_dia.contains("الحمد لله") {
                continue;
            }
            // Skip common intro/metadata phrases
            if trimmed.contains("اعتنى به")
                || trimmed.contains("مصدر الكتاب")
                || trimmed.contains("موقع")
                || trimmed.contains("(*)")
                || trimmed.contains("تم نسخه")
                || trimmed.contains("نسخ من")
                || trimmed.contains("http")
                || trimmed.contains("www")
            {
                continue;
            }
            // Skip ل[...] pattern
            if trimmed.starts_with("ل[") {
                continue;
            }
            // Skip pure bracket wrappers [...] but NOT -[...] or ـ[...]
            if trimmed.starts_with('[') && trimmed.ends_with(']')
                && !trimmed.starts_with("-[") && !trimmed.starts_with("ـ[")
            {
                continue;
            }
            // Skip lines of only dots/dashes/underscores/spaces/asterisks/kashida/ellipsis
            if trimmed.chars().all(|c: char| {
                c == '.' || c == ' ' || c == '-' || c == '_' || c == '*'
                || c == 'ـ' || c == '…' || c == '\n'
            }) {
                continue;
            }
            // Skip common generic/formula starters
            if no_dia.starts_with("وبعد") || no_dia.starts_with("أما بعد")
                || no_dia.starts_with("أمّا بعد") || no_dia.starts_with("والله الموفق")
                || no_dia.starts_with("فإن ") || no_dia.starts_with("فان ")
            {
                continue;
            }
            // Skip author attribution lines (these are NOT book names)
            if no_dia.starts_with("المؤلف") || no_dia.starts_with("تأليف")
                || no_dia.starts_with("تصنيف") || no_dia.starts_with("كتبه")
                || no_dia.starts_with("جمعه")
            {
                continue;
            }
            // Skip Quranic verse markers (any size)
            if trimmed.starts_with('{') || trimmed.starts_with('﴿') {
                continue;
            }
            if trimmed.starts_with('(') && (trimmed.len() > 50 || no_dia.contains("سورة")) {
                continue;
            }
            // Skip sentence-continuation starters (text fragments, not book titles)
            if no_dia.starts_with("وهذه ") || no_dia.starts_with("ثم ")
                || no_dia.starts_with("ولذا ") || no_dia.starts_with("وقد ")
                || no_dia.starts_with("ولقد ") || no_dia.starts_with("ولما ")
                || no_dia.starts_with("فلما ") || no_dia.starts_with("ولكن ")
                || no_dia.starts_with("هذه ") || no_dia.starts_with("هذا ")
            {
                continue;
            }
            // Skip attribution sentences (قال الإمام..., وقال الشيخ...)
            if no_dia.starts_with("قال ") || no_dia.starts_with("وقال ")
                || no_dia.starts_with("انظر") // انظر: تاريخ... (footnote cross-references)
            {
                continue;
            }
            // Skip shalawat formulas (all variants)
            if no_dia.contains("وصلى الله على") || no_dia.contains("صلى الله عليه")
                || no_dia.contains("وصلى الله وسلم")
            {
                continue;
            }
            // Skip biographical descriptions (من سكان مدينة..., حاصل على درجة..., ذكر نسب...)
            if no_dia.starts_with("من سكان") || no_dia.starts_with("حاصل على")
                || no_dia.starts_with("ذكر نسب")
            {
                continue;
            }
            // Skip conditional/continuation constructions (أما بقية..., أما ما...)
            if no_dia.starts_with("أما ") {
                continue;
            }
            // Skip Arabic ordinals (أولا, ثانيا, ... خامسا: التفسير الفقهي...)
            if no_dia.starts_with("أولا") || no_dia.starts_with("ثانيا")
                || no_dia.starts_with("ثالثا") || no_dia.starts_with("رابعا")
                || no_dia.starts_with("خامسا") || no_dia.starts_with("سادسا")
                || no_dia.starts_with("سابعا") || no_dia.starts_with("ثامنا")
                || no_dia.starts_with("تاسعا") || no_dia.starts_with("عاشرا")
            {
                continue;
            }
            // Skip generic section fragments (وفيه مسائل, مجموعة, ...)
            if no_dia == "وفيه مسائل" || no_dia == "وفيه مسائل."
                || no_dia == "مجموعة" || no_dia == "مقدمات"
                || no_dia == "المقدمة" || no_dia == "مقدمة"
            {
                continue;
            }
            // Skip bracket-only names like [مقدمات], [مقدمة الكتاب]
            {
                let stripped = no_dia.trim_matches(|c: char| c == '[' || c == ']' || c == '(' || c == ')');
                if stripped == "مقدمات" || stripped == "مقدمة" || stripped == "المقدمة"
                    || stripped == "مقدمة الكتاب" || stripped == "خاتمة" || stripped.is_empty()
                {
                    continue;
                }
            }
            // Skip very long lines (> 80 chars) — these are descriptions, not book titles
            if no_dia.chars().count() > 80 {
                continue;
            }
            // Skip commentary/definition markers
            if no_dia.starts_with("قوله") || no_dia.starts_with("ومعنى")
                || no_dia.starts_with("في الاصطلاح")
            {
                continue;
            }
            // Skip job/position descriptions
            if no_dia.starts_with("الوظيفة") {
                continue;
            }
            // Skip school/mazhab attribution fragments
            if no_dia.starts_with("من الحنفية") || no_dia.starts_with("من المالكية")
                || no_dia.starts_with("من الشافعية") || no_dia.starts_with("من الحنابلة")
            {
                continue;
            }
            // Skip shahada, dua formulas
            if no_dia.contains("واشهد ان لا اله") || no_dia.contains("وأشهد أن لا إله") {
                continue;
            }
            if no_dia.starts_with("اللهم ") {
                continue;
            }
            // Skip publisher/series info
            if no_dia.starts_with("الناشر") || no_dia.starts_with("سلسلة ") {
                continue;
            }
            // Skip preface attribution
            if no_dia.starts_with("تقديم") {
                continue;
            }
            // Skip continuation markers and sentence fragments
            if no_dia.starts_with("ولا بد") || no_dia.starts_with("والسير ")
                || no_dia == "ويليه"
                || no_dia.starts_with("وفي رواية") || no_dia.starts_with("في رواية")
                || no_dia.starts_with("والجواب ") || no_dia.starts_with("والتعدية")
                || no_dia.starts_with("الوجه ") || no_dia.starts_with("وأشد ")
            {
                continue;
            }
            // Skip chapter/section headings 
            if no_dia.starts_with("باب ") || no_dia.starts_with("فصل ")
                || no_dia.starts_with("دكة ") || no_dia.starts_with("خطبة ")
            {
                continue;
            }
            // Skip author-as-title patterns (العلامة محمد بن...)
            if no_dia.starts_with("العلامة ") || no_dia.starts_with("السيد ") {
                continue;
            }
            // Skip edition/print metadata
            if no_dia.starts_with("الطبعة") || no_dia.starts_with("وزير ")
                || no_dia.starts_with("عدد الأجزاء") || no_dia.starts_with("عدد الصفحات")
                || no_dia.starts_with("البريد") || no_dia.contains("@")
            {
                continue;
            }
            // Skip editorial/verification credits
            if no_dia.starts_with("حققه") || no_dia.starts_with("حقّقه") {
                continue;
            }
            // Skip commentary/enumeration patterns
            if no_dia.starts_with("أحدهما") || no_dia.starts_with("والاستعانة")
                || no_dia.starts_with("والجواب ")
                || no_dia.starts_with("الثاني:") || no_dia.starts_with("الثاني ")
                || no_dia.starts_with("الأول:") || no_dia.starts_with("الأول ")
            {
                continue;
            }
            // Skip closing formulas
            if no_dia.starts_with("والله من وراء") || no_dia.starts_with("والله أعلم")
                || no_dia.starts_with("تحريرا ") || no_dia.starts_with("تحريراً ")
            {
                continue;
            }
            // Skip content with Quranic verse markers in middle of line
            if no_dia.contains('{') && no_dia.contains('}') && no_dia.len() > 50 {
                continue;
            }
            // Skip personal statements / mottos
            if no_dia.starts_with("لا أنتمي") {
                continue;
            }
            // Skip comma-relative clause patterns (sentence fragments)
            if no_dia.contains("، التي ") || no_dia.contains("، الذي ")
                || no_dia.contains("، وهي ") || no_dia.contains("، وهو ")
            {
                continue;
            }
            // Skip lines starting with Arabic or Latin digits (chapter/section numbering)
            {
                let first_char = trimmed.chars().next().unwrap_or(' ');
                if ('\u{0660}'..='\u{0669}').contains(&first_char) || first_char.is_ascii_digit() {
                    continue;
                }
            }
            // Skip lines ending with colon and short (section headers / definition markers)
            if no_dia.ends_with(':') && no_dia.chars().count() < 60 {
                continue;
            }
            // Skip lines ending with period (sentence fragments — book titles don't end with ".")
            // BUT: strip it first and let other checks decide, to avoid rejecting titles like "تفسير آيات الأحكام."
            // (trailing period stripped in post-extraction cleanup)
            // Note: question mark ending NOT skipped here — some books have ? in title
            // (e.g., "هل علي بن أبي طالب معصوم؟")
            // Trailing ? is stripped in post-extraction cleanup and is_garbage_book_name
            // Skip comma-pronoun patterns (، هي/، هى)
            if no_dia.contains("، هي") || no_dia.contains("، هى") {
                continue;
            }
            // Skip colon-pronoun definition patterns (المقابلة: وهي...)
            if no_dia.contains(": وهي ") || no_dia.contains(": وهو ") {
                continue;
            }
            // Skip sentence-start patterns starting with وال... (والآيتان, والمقابلة, etc.)
            if no_dia.starts_with("والآي") || no_dia.starts_with("والمقابلة") {
                continue;
            }
            // Skip question fragments and work descriptions
            if no_dia.starts_with("وهل ") || no_dia.starts_with("وعملنا ")
                || no_dia.starts_with("وكتبه") || no_dia.starts_with("وكل ")
                || no_dia.starts_with("وأصل ") || no_dia.starts_with("وهذا ")
            {
                continue;
            }
            // Skip lines containing ellipsis (poetry verses or truncated text)
            if no_dia.contains("...") || no_dia.contains('…') {
                continue;
            }
            // Skip person names (contains بن + أبو pattern)
            if no_dia.contains(" بن ") && no_dia.contains("أبو ") {
                continue;
            }
            // Skip prepositional/sentence fragments
            if no_dia.starts_with("لي ") || no_dia.starts_with("لك ")
                || no_dia.starts_with("فقد ") || no_dia.starts_with("فهذا ")
                || no_dia.starts_with("وكان ") || no_dia.starts_with("وكانت ")
                || no_dia.starts_with("ومن ") || no_dia.starts_with("وعن ")
                || no_dia.starts_with("الشيطان ")
            {
                continue;
            }
            // Skip comma-relative clause pattern with undotted variant
            if no_dia.contains("، التى ") {
                continue;
            }

            // ── Extract inner text from bracket patterns ──
            // Kashida-bracket: ـ[الموسوعة الفقهية]ـ → الموسوعة الفقهية
            if trimmed.starts_with("ـ[") {
                let inner = trimmed
                    .trim_start_matches("ـ[")
                    .trim_end_matches("]ـ")
                    .trim_end_matches(']')
                    .trim();
                if !inner.is_empty() && inner.len() > 4 {
                    book_name = inner.to_string();
                    break;
                }
                continue;
            }
            // Dash-bracket: -[خزانة التراث]- → خزانة التراث
            if trimmed.starts_with("-[") {
                let inner = trimmed
                    .trim_start_matches("-[")
                    .trim_end_matches("]-")
                    .trim_end_matches(']')
                    .trim();
                if !inner.is_empty() && inner.len() > 4 {
                    book_name = inner.to_string();
                    break;
                }
                continue;
            }

            // If line is short enough, use as title
            if trimmed.len() < 200 {
                book_name = trimmed.to_string();
                break;
            }
        }
    }

    // ── Post-extraction cleanup ──
    // Clean ل[...] prefix
    if book_name.starts_with("ل[") || book_name.starts_with('[') {
        book_name = book_name
            .trim_start_matches('ل')
            .trim_start_matches('[')
            .trim_end_matches(']')
            .trim()
            .to_string();
    }
    // Clean ـ[...]ـ kashida-bracket (may come from title_markers path)
    if book_name.starts_with("ـ[") {
        book_name = book_name
            .trim_start_matches("ـ[")
            .trim_end_matches("]ـ")
            .trim_end_matches(']')
            .trim()
            .to_string();
    }
    // Clean -[...]- dash-bracket
    if book_name.starts_with("-[") {
        book_name = book_name
            .trim_start_matches("-[")
            .trim_end_matches("]-")
            .trim_end_matches(']')
            .trim()
            .to_string();
    }
    // Strip leading/trailing kashida and brackets
    book_name = book_name.trim_matches(|c: char| c == 'ـ' || c == '[' || c == ']' || c == '-').trim().to_string();

    // Strip trailing punctuation (period, question mark, comma) — not part of book titles
    book_name = book_name.trim_end_matches('.').trim_end_matches('?')
        .trim_end_matches('\u{061F}').trim_end_matches(',').trim_end_matches('\u{060C}').trim().to_string();

    // Strip leading non-Arabic encoding artifacts (e.g., 舄 before Arabic text)
    // Keep only Arabic letters (U+0600-U+06FF, U+0750-U+077F, U+FB50-U+FDFF, U+FE70-U+FEFF), spaces, and common punctuation
    if !book_name.is_empty() {
        let first_char = book_name.chars().next().unwrap();
        let is_arabic_or_common = |c: char| -> bool {
            ('\u{0600}'..='\u{06FF}').contains(&c) || ('\u{0750}'..='\u{077F}').contains(&c)
            || ('\u{FB50}'..='\u{FDFF}').contains(&c) || ('\u{FE70}'..='\u{FEFF}').contains(&c)
            || c == ' ' || c == '(' || c == ')' || c == '[' || c == ']' || c == '{' || c == '}'
            || c == '﴿' || c == '﴾'
        };
        if !is_arabic_or_common(first_char) {
            book_name = book_name.chars().skip_while(|c| !is_arabic_or_common(*c)).collect::<String>().trim().to_string();
        }
    }

    // Strip trailing " [N..." number bracket patterns (e.g., "كتاب الأم [١")
    if let Some(bracket_pos) = book_name.rfind(" [") {
        let after = &book_name[bracket_pos + 2..]; // after " ["
        let inner = after.trim_end_matches(']').trim();
        let inner_clean = strip_arabic_diacritics(inner);
        // Strip if: empty, all digits, or section labels like المقدمة
        if inner.is_empty() || inner.chars().all(|c| {
            ('\u{0660}'..='\u{0669}').contains(&c) || c.is_ascii_digit() || c == ' ' || c == '-'
        }) || inner_clean == "المقدمة" || inner_clean == "مقدمة" || inner_clean == "المقدمة:"
        {
            book_name = book_name[..bracket_pos].trim().to_string();
        }
    }

    // Strip trailing " - المقدمة" section label patterns
    {
        let name_check = strip_arabic_diacritics(&book_name);
        if name_check.ends_with("المقدمة") || name_check.ends_with("المقدمه") {
            if let Some(dash_pos) = book_name.rfind(" - ") {
                let before = &book_name[..dash_pos];
                if !before.trim().is_empty() && before.trim().chars().count() > 4 {
                    book_name = before.trim().to_string();
                }
            }
        }
    }

    // ── Garbage name detection (uses consolidated function) ──
    let is_garbage = is_garbage_book_name(&book_name);

    // Also clean author_name with same bracket patterns
    if author_name.starts_with("ل[") || author_name.starts_with('[') {
        author_name = author_name
            .trim_start_matches('ل')
            .trim_start_matches('[')
            .trim_end_matches(']')
            .trim()
            .to_string();
    }
    if author_name.starts_with("ـ[") {
        author_name = author_name
            .trim_start_matches("ـ[")
            .trim_end_matches("]ـ")
            .trim_end_matches(']')
            .trim()
            .to_string();
    }
    author_name = author_name.trim_matches(|c: char| c == 'ـ' || c == '[' || c == ']' || c == '-').trim().to_string();

    // Final fallback
    if book_name.is_empty() || is_garbage {
        book_name = format!("كتاب {}", book_id);
    }

    BookMetadata {
        book_name,
        author_name,
    }
}
