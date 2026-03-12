use crate::models::SearchResult;
use log::info;
use parking_lot::Mutex;
use rusqlite::{params, Connection, OpenFlags};
use std::sync::Arc;

/// Produk Hukum database — separate SQLite for bahtsul masail documents
pub struct ProdukHukumDb {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProdukHukumItem {
    pub id: i64,
    pub title: String,
    pub category: String,
    pub subcategory: String,
    pub file_type: String,
    pub file_size: i64,
    pub page_count: i64,
    pub source_file: String,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProdukHukumDetail {
    pub id: i64,
    pub title: String,
    pub category: String,
    pub subcategory: String,
    pub file_type: String,
    pub file_size: i64,
    pub page_count: i64,
    pub source_file: String,
    pub content: String,
    pub created_at: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProdukHukumCategory {
    pub name: String,
    pub doc_count: i64,
}

#[derive(Debug, serde::Serialize)]
pub struct ProdukHukumListResponse {
    pub documents: Vec<ProdukHukumItem>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
    pub categories: Vec<ProdukHukumCategory>,
}

#[derive(Debug, serde::Serialize)]
pub struct ProdukHukumSearchResult {
    pub id: i64,
    pub title: String,
    pub category: String,
    pub snippet: String,
    pub file_type: String,
    pub page_count: i64,
    pub source_file: String,
}

#[derive(Debug, serde::Serialize)]
pub struct ProdukHukumSearchResponse {
    pub results: Vec<ProdukHukumSearchResult>,
    pub total: i64,
    pub query: String,
}

#[derive(Debug, serde::Serialize)]
pub struct ProdukHukumStatsResponse {
    pub total_documents: i64,
    pub total_categories: i64,
    pub categories: Vec<ProdukHukumCategory>,
}

impl ProdukHukumDb {
    pub fn new(db_path: &str) -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        let conn = Connection::open_with_flags(
            db_path,
            OpenFlags::SQLITE_OPEN_READ_ONLY,
        )?;

        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA cache_size = -8000;
             PRAGMA mmap_size = 268435456;
             PRAGMA temp_store = MEMORY;
             PRAGMA busy_timeout = 10000;",
        )?;

        // Verify table exists
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM documents",
            [],
            |row| row.get(0),
        )?;
        info!("Produk Hukum DB loaded with {} documents", count);

        Ok(Arc::new(Self {
            conn: Mutex::new(conn),
        }))
    }

    /// List documents with pagination and optional category filter
    pub fn list_documents(
        &self,
        page: i64,
        per_page: i64,
        category: Option<&str>,
    ) -> Result<ProdukHukumListResponse, String> {
        let conn = self.conn.lock();
        let offset = (page - 1) * per_page;

        // Get total count
        let total: i64 = if let Some(cat) = category {
            conn.query_row(
                "SELECT COUNT(*) FROM documents WHERE category = ?",
                params![cat],
                |row| row.get(0),
            )
        } else {
            conn.query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
        }
        .map_err(|e| format!("DB error: {}", e))?;

        // Get documents
        let documents: Vec<ProdukHukumItem> = if let Some(cat) = category {
            let mut stmt = conn
                .prepare(
                    "SELECT id, title, category, subcategory, file_type, file_size, page_count, source_file, created_at
                     FROM documents WHERE category = ?
                     ORDER BY title ASC LIMIT ? OFFSET ?",
                )
                .map_err(|e| format!("Prepare error: {}", e))?;

            let rows = stmt.query_map(params![cat, per_page, offset], |row| {
                Ok(ProdukHukumItem {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    category: row.get(2)?,
                    subcategory: row.get::<_, String>(3).unwrap_or_default(),
                    file_type: row.get(4)?,
                    file_size: row.get(5)?,
                    page_count: row.get(6)?,
                    source_file: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })
            .map_err(|e| format!("Query error: {}", e))?;
            rows.filter_map(|r| r.ok()).collect()
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT id, title, category, subcategory, file_type, file_size, page_count, source_file, created_at
                     FROM documents ORDER BY title ASC LIMIT ? OFFSET ?",
                )
                .map_err(|e| format!("Prepare error: {}", e))?;

            let rows = stmt.query_map(params![per_page, offset], |row| {
                Ok(ProdukHukumItem {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    category: row.get(2)?,
                    subcategory: row.get::<_, String>(3).unwrap_or_default(),
                    file_type: row.get(4)?,
                    file_size: row.get(5)?,
                    page_count: row.get(6)?,
                    source_file: row.get(7)?,
                    created_at: row.get(8)?,
                })
            })
            .map_err(|e| format!("Query error: {}", e))?;
            rows.filter_map(|r| r.ok()).collect()
        };

        // Get categories
        let categories = self.get_categories_inner(&conn)?;

        Ok(ProdukHukumListResponse {
            documents,
            total,
            page,
            per_page,
            categories,
        })
    }

    /// Get a single document with full content
    pub fn get_document(&self, id: i64) -> Result<ProdukHukumDetail, String> {
        let conn = self.conn.lock();
        conn.query_row(
            "SELECT id, title, category, subcategory, file_type, file_size, page_count, source_file, content, created_at
             FROM documents WHERE id = ?",
            params![id],
            |row| {
                Ok(ProdukHukumDetail {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    category: row.get(2)?,
                    subcategory: row.get::<_, String>(3).unwrap_or_default(),
                    file_type: row.get(4)?,
                    file_size: row.get(5)?,
                    page_count: row.get(6)?,
                    source_file: row.get(7)?,
                    content: row.get(8)?,
                    created_at: row.get(9)?,
                })
            },
        )
        .map_err(|e| format!("Document not found: {}", e))
    }

    /// Full-text search across documents
    pub fn search_documents(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<ProdukHukumSearchResponse, String> {
        let conn = self.conn.lock();

        // Clean query for FTS5
        let clean_query = query
            .replace('"', "")
            .replace('\'', "")
            .replace('(', "")
            .replace(')', "")
            .trim()
            .to_string();

        if clean_query.is_empty() {
            return Ok(ProdukHukumSearchResponse {
                results: vec![],
                total: 0,
                query: query.to_string(),
            });
        }

        // Use FTS5 search with snippet
        let fts_query = clean_query
            .split_whitespace()
            .map(|w| format!("\"{}\"", w))
            .collect::<Vec<_>>()
            .join(" OR ");

        let mut stmt = conn
            .prepare(
                "SELECT d.id, d.title, d.category, 
                        snippet(documents_fts, 1, '<mark>', '</mark>', '...', 40),
                        d.file_type, d.page_count, d.source_file
                 FROM documents_fts f
                 JOIN documents d ON d.id = f.rowid
                 WHERE documents_fts MATCH ?
                 ORDER BY rank
                 LIMIT ?",
            )
            .map_err(|e| format!("Prepare error: {}", e))?;

        let rows = stmt
            .query_map(params![fts_query, limit], |row| {
                Ok(ProdukHukumSearchResult {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    category: row.get(2)?,
                    snippet: row.get(3)?,
                    file_type: row.get(4)?,
                    page_count: row.get(5)?,
                    source_file: row.get(6)?,
                })
            })
            .map_err(|e| format!("Search error: {}", e))?;
        let results: Vec<ProdukHukumSearchResult> = rows.filter_map(|r| r.ok()).collect();

        let total = results.len() as i64;

        Ok(ProdukHukumSearchResponse {
            results,
            total,
            query: query.to_string(),
        })
    }

    /// Get statistics
    pub fn get_stats(&self) -> Result<ProdukHukumStatsResponse, String> {
        let conn = self.conn.lock();
        let total_documents: i64 = conn
            .query_row("SELECT COUNT(*) FROM documents", [], |row| row.get(0))
            .map_err(|e| format!("DB error: {}", e))?;

        let categories = self.get_categories_inner(&conn)?;
        let total_categories = categories.len() as i64;

        Ok(ProdukHukumStatsResponse {
            total_documents,
            total_categories,
            categories,
        })
    }

    fn get_categories_inner(
        &self,
        conn: &Connection,
    ) -> Result<Vec<ProdukHukumCategory>, String> {
        let mut stmt = conn
            .prepare(
                "SELECT category, COUNT(*) as cnt FROM documents GROUP BY category ORDER BY cnt DESC",
            )
            .map_err(|e| format!("Prepare error: {}", e))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ProdukHukumCategory {
                    name: row.get(0)?,
                    doc_count: row.get(1)?,
                })
            })
            .map_err(|e| format!("Query error: {}", e))?;
        let categories: Vec<ProdukHukumCategory> = rows.filter_map(|r| r.ok()).collect();

        Ok(categories)
    }

    /// Search produk hukum and return results compatible with the unified SearchResult model.
    /// This allows produk hukum documents to appear alongside kitab results.
    pub fn search_for_unified(
        &self,
        query: &str,
        limit: i64,
    ) -> Result<Vec<SearchResult>, String> {
        let conn = self.conn.lock();

        let clean_query = query
            .replace('"', "")
            .replace('\'', "")
            .replace('(', "")
            .replace(')', "")
            .trim()
            .to_string();

        if clean_query.is_empty() {
            return Ok(vec![]);
        }

        // Build FTS5 query with OR between words
        let fts_query = clean_query
            .split_whitespace()
            .map(|w| format!("\"{}\"", w))
            .collect::<Vec<_>>()
            .join(" OR ");

        let mut stmt = conn
            .prepare(
                "SELECT d.id, d.title, d.category,
                        snippet(documents_fts, 1, '', '', '...', 60),
                        d.page_count, d.source_file, rank
                 FROM documents_fts f
                 JOIN documents d ON d.id = f.rowid
                 WHERE documents_fts MATCH ?
                 ORDER BY rank
                 LIMIT ?",
            )
            .map_err(|e| format!("Prepare error: {}", e))?;

        let results: Vec<SearchResult> = stmt
            .query_map(params![fts_query, limit], |row| {
                let id: i64 = row.get(0)?;
                let title: String = row.get(1)?;
                let category: String = row.get(2)?;
                let snippet: String = row.get(3)?;
                let page_count: i64 = row.get(4)?;
                let source_file: String = row.get(5)?;
                let rank: f64 = row.get(6)?;

                // Convert FTS5 rank to a score in 0-75 range
                // (lower than kitab max 100 to give kitab slight priority)
                let score = ((-rank).min(50.0) / 50.0 * 75.0) as f32;

                Ok(SearchResult {
                    book_id: 0,      // Not a kitab
                    toc_id: id,      // Use document id
                    title,
                    content_snippet: snippet,
                    page: format!("{}", page_count),
                    part: source_file,
                    score: score.max(10.0), // minimum score of 10
                    hierarchy: vec!["Produk Hukum".to_string()],
                    book_name: "Produk Hukum".to_string(),
                    author_name: "Bahtsul Masail".to_string(),
                    source_type: "produk_hukum".to_string(),
                    category,
                    citation: String::new(),
                    similarity_score: 0.0,
                    toc_page: String::new(),
                })
            })
            .map_err(|e| format!("Search error: {}", e))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }
}
