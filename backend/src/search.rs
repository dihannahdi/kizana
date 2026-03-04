use crate::arabic_stemmer::ArabicStemmer;
use crate::db::Database;
use crate::models::*;
use crate::query_translator::{QueryTranslator, TranslatedQuery};
use log::{info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{doc, Index, IndexWriter, ReloadPolicy, TantivyDocument};
use parking_lot::RwLock;

/// Maximum results from the same book (diversity limit)
const MAX_PER_BOOK: usize = 3;

pub struct SearchEngine {
    index: Index,
    reader: tantivy::IndexReader,
    schema: Schema,
    field_content: Field,
    field_book_id: Field,
    field_toc_id: Field,
    field_page: Field,
    field_parent: Field,
    field_type: Field, // "toc" or "content"
    db: Arc<Database>,
    indexed: RwLock<bool>,
    translator: QueryTranslator,
    stemmer: ArabicStemmer,
    /// Cache: book_id → TOC entry count (loaded at startup for large-book penalties)
    book_toc_counts: RwLock<HashMap<i64, usize>>,
}

impl SearchEngine {
    pub fn new(
        index_path: &str,
        db: Arc<Database>,
    ) -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        let mut schema_builder = Schema::builder();

        let text_options = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("default")
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions),
            )
            .set_stored();

        let field_content = schema_builder.add_text_field("content", text_options);
        let field_book_id =
            schema_builder.add_i64_field("book_id", INDEXED | STORED);
        let field_toc_id =
            schema_builder.add_i64_field("toc_id", INDEXED | STORED);
        let field_page = schema_builder.add_text_field("page", STRING | STORED);
        let field_parent =
            schema_builder.add_i64_field("parent", STORED);
        let field_type = schema_builder.add_text_field("doc_type", STRING | STORED);

        let schema = schema_builder.build();

        // Create or open index
        let index_dir = std::path::Path::new(index_path);
        let index = if index_dir.exists() {
            match Index::open_in_dir(index_dir) {
                Ok(idx) => {
                    info!("Opened existing Tantivy index");
                    idx
                }
                Err(_) => {
                    warn!("Failed to open index, recreating...");
                    std::fs::remove_dir_all(index_dir).ok();
                    std::fs::create_dir_all(index_dir)?;
                    Index::create_in_dir(index_dir, schema.clone())?
                }
            }
        } else {
            std::fs::create_dir_all(index_dir)?;
            Index::create_in_dir(index_dir, schema.clone())?
        };

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        let has_docs = {
            let searcher = reader.searcher();
            searcher.num_docs() > 0
        };

        let translator = QueryTranslator::new();
        let stemmer = ArabicStemmer::new();
        info!("Query translator initialized with multilingual dictionary");
        info!("Arabic stemmer initialized for morphological expansion");

        Ok(Arc::new(SearchEngine {
            index,
            reader,
            schema,
            field_content,
            field_book_id,
            field_toc_id,
            field_page,
            field_parent,
            field_type,
            db,
            indexed: RwLock::new(has_docs),
            translator,
            stemmer,
            book_toc_counts: RwLock::new(HashMap::new()),
        }))
    }

    pub fn is_indexed(&self) -> bool {
        *self.indexed.read()
    }

    /// Build the Tantivy index from all TOC entries. 
    /// This is called once at startup if index is empty.
    pub fn build_index(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_indexed() {
            // Even if index exists, load TOC counts for scoring
            self.load_toc_counts();
            info!("Index already built, skipping...");
            return Ok(());
        }

        info!("Building Tantivy index from TOC data...");
        let mut writer: IndexWriter = self.index.writer(100_000_000)?; // 100MB heap

        let book_ids = self.db.get_book_ids().to_vec();
        let total = book_ids.len();
        let mut indexed_count = 0u64;
        let mut error_count = 0u64;
        let mut toc_counts = HashMap::new();

        for (i, &book_id) in book_ids.iter().enumerate() {
            match self.db.get_toc_entries(book_id) {
                Ok(entries) => {
                    toc_counts.insert(book_id, entries.len());
                    for entry in &entries {
                        writer.add_document(doc!(
                            self.field_content => entry.content.clone(),
                            self.field_book_id => book_id,
                            self.field_toc_id => entry.id,
                            self.field_page => entry.page.clone(),
                            self.field_parent => entry.parent,
                            self.field_type => "toc"
                        ))?;
                        indexed_count += 1;
                    }
                }
                Err(e) => {
                    error_count += 1;
                    if error_count < 10 {
                        warn!("Error indexing book {}: {}", book_id, e);
                    }
                }
            }

            if (i + 1) % 500 == 0 || i + 1 == total {
                info!("Indexing progress: {}/{} books, {} entries", i + 1, total, indexed_count);
                // Intermediate commit every 500 books
                if (i + 1) % 500 == 0 {
                    writer.commit()?;
                }
            }
        }

        writer.commit()?;
        self.reader.reload()?;

        // Store TOC counts for scoring
        {
            let mut counts = self.book_toc_counts.write();
            *counts = toc_counts;
        }

        let mut indexed = self.indexed.write();
        *indexed = true;

        info!(
            "Index built: {} TOC entries from {} books ({} errors)",
            indexed_count, total, error_count
        );
        Ok(())
    }

    /// Load TOC entry counts per book (for scoring penalties on encyclopedic books)
    fn load_toc_counts(&self) {
        let existing = self.book_toc_counts.read().len();
        if existing > 0 {
            return; // Already loaded
        }
        info!("Loading TOC counts for scoring...");
        let book_ids = self.db.get_book_ids().to_vec();
        let mut counts = HashMap::new();
        for &book_id in &book_ids {
            if let Ok(count) = self.db.get_toc_count(book_id) {
                counts.insert(book_id, count);
            }
        }
        let mut cache = self.book_toc_counts.write();
        *cache = counts;
        info!("TOC counts loaded for {} books", cache.len());
    }

    /// Translate a query using the built-in multilingual translator
    pub fn translate_query(&self, raw_query: &str) -> TranslatedQuery {
        self.translator.translate(raw_query)
    }

    /// Search using Tantivy BM25 + hierarchy boost + query translation
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<SearchResult>, String> {
        // First, translate the query
        let translated = self.translator.translate(query_str);
        self.search_with_translated(&translated, limit)
    }

    /// Search using a pre-translated query
    pub fn search_with_translated(&self, translated: &TranslatedQuery, limit: usize) -> Result<Vec<SearchResult>, String> {
        if !self.is_indexed() {
            // Fallback to SQLite LIKE search
            return self.db.search_toc_fts(&translated.original, limit);
        }

        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![self.field_content]);

        // Try translated query first
        let query_text = if !translated.tantivy_query.is_empty() {
            &translated.tantivy_query
        } else {
            &translated.original
        };

        let query = match query_parser.parse_query(query_text) {
            Ok(q) => q,
            Err(_) => {
                // Fallback: try individual Arabic terms
                let fallback = translated.arabic_terms.join(" ");
                if !fallback.is_empty() {
                    query_parser
                        .parse_query(&fallback)
                        .map_err(|e| format!("Query parse error: {}", e))?
                } else {
                    // Final fallback: original query
                    query_parser
                        .parse_query(&translated.original)
                        .map_err(|e| format!("Query parse error: {}", e))?
                }
            }
        };

        // Fetch more candidates than needed for diversity filtering
        let fetch_count = limit * 5;
        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(fetch_count))
            .map_err(|e| format!("Search error: {}", e))?;

        let toc_counts = self.book_toc_counts.read();
        let mut all_results: Vec<SearchResult> = Vec::new();

        for (bm25_score, doc_address) in &top_docs {
            if let Ok(retrieved_doc) = searcher.doc::<TantivyDocument>(*doc_address) {
                let content = retrieved_doc
                    .get_first(self.field_content)
                    .and_then(|v: &tantivy::schema::OwnedValue| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let book_id = retrieved_doc
                    .get_first(self.field_book_id)
                    .and_then(|v: &tantivy::schema::OwnedValue| v.as_i64())
                    .unwrap_or(0);

                let toc_id = retrieved_doc
                    .get_first(self.field_toc_id)
                    .and_then(|v: &tantivy::schema::OwnedValue| v.as_i64())
                    .unwrap_or(0);

                let page = retrieved_doc
                    .get_first(self.field_page)
                    .and_then(|v: &tantivy::schema::OwnedValue| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let parent = retrieved_doc
                    .get_first(self.field_parent)
                    .and_then(|v: &tantivy::schema::OwnedValue| v.as_i64())
                    .unwrap_or(0);

                // Get hierarchy
                let hierarchy = self
                    .db
                    .get_toc_hierarchy(book_id, toc_id)
                    .unwrap_or_default();

                // Get content snippet AND display page from book content table
                let (content_snippet, display_page) = self
                    .db
                    .get_content_snippet_with_page(book_id, &page)
                    .unwrap_or_default();

                // Get book metadata (name + author)
                let meta = self.db.get_book_metadata(book_id);

                // ─── Scoring adjustments ───
                
                // Hierarchy boost: deeper = more specific = higher boost
                let depth_boost = hierarchy.len() as f32 * 0.1;
                
                // Parent relevance boost: if parent is not root, moderate boost
                let parent_boost = if parent > 0 { 0.15 } else { 0.0 };

                // Large-book penalty: encyclopedic fatwa collections with >10K entries
                // get a slight penalty to prevent them from dominating results.
                // Books with >50K entries (like فتاوى الشبكة الإسلامية with 92K) get
                // the strongest penalty.
                let toc_count = toc_counts.get(&book_id).copied().unwrap_or(0);
                let size_factor = if toc_count > 50_000 {
                    0.75  // -25% for massive encyclopedic collections
                } else if toc_count > 20_000 {
                    0.85  // -15% for large collections
                } else if toc_count > 10_000 {
                    0.92  // -8% for big books
                } else {
                    1.0   // No penalty for normal books
                };

                let adjusted_score = (bm25_score + depth_boost + parent_boost) * size_factor;

                // Strip HTML tags from TOC title
                let clean_title = strip_html_tags(&content);

                all_results.push(SearchResult {
                    book_id,
                    toc_id,
                    title: clean_title,
                    content_snippet,
                    page: display_page,  // Now shows actual display page, not row ID
                    part: String::new(),
                    score: adjusted_score,
                    hierarchy,
                    book_name: meta.book_name,
                    author_name: meta.author_name,
                    source_type: "kitab".to_string(),
                    category: String::new(),
                });
            }
        }

        // Sort by adjusted score
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // ─── Per-book diversity limit ───
        // Allow at most MAX_PER_BOOK results from the same book.
        // This ensures users see references from different kitab/authors.
        let mut book_count: HashMap<i64, usize> = HashMap::new();
        let mut diverse_results: Vec<SearchResult> = Vec::new();
        
        for result in all_results {
            let count = book_count.entry(result.book_id).or_insert(0);
            if *count < MAX_PER_BOOK {
                *count += 1;
                diverse_results.push(result);
                if diverse_results.len() >= limit {
                    break;
                }
            }
        }

        // Normalize scores to 0-100
        if let Some(max_score) = diverse_results.first().map(|r| r.score) {
            if max_score > 0.0 {
                for r in &mut diverse_results {
                    r.score = (r.score / max_score * 100.0).min(100.0);
                }
            }
        }

        info!(
            "Search: {} candidates → {} diverse results (from {} unique books)",
            top_docs.len(),
            diverse_results.len(),
            book_count.len()
        );

        Ok(diverse_results)
    }

    // ═══════════════════════════════════════════════════════════════
    // EVALUATION & ABLATION STUDY METHODS
    // These methods support the academic evaluation framework for
    // measuring IR metrics (NDCG, MAP, P@K, MRR) and running
    // ablation experiments where individual scoring components
    // are disabled to measure their contribution.
    // ═══════════════════════════════════════════════════════════════

    /// Search with evaluation configuration — supports ablation study
    /// Each scoring component can be independently disabled via EvalConfig
    pub fn search_eval(
        &self,
        query_str: &str,
        limit: usize,
        config: &EvalConfig,
    ) -> Result<(Vec<SearchResult>, TranslatedQuery), String> {
        // Step 1: Query translation (can be disabled for baseline comparison)
        let translated = if config.disable_query_translation {
            // Baseline: no translation, just pass raw query
            TranslatedQuery {
                original: query_str.to_string(),
                arabic_terms: vec![],
                latin_terms: vec![query_str.to_string()],
                detected_language: crate::query_translator::QueryLang::Unknown,
                detected_domain: crate::query_translator::FiqhDomain::Unknown,
                tantivy_query: query_str.to_string(),
                confidence_note: String::new(),
            }
        } else if config.disable_phrase_mapping || config.disable_multi_variant {
            // Partial ablation: translate but with restricted features
            self.translator.translate_with_config(
                query_str,
                !config.disable_phrase_mapping,
                !config.disable_multi_variant,
            )
        } else {
            self.translator.translate(query_str)
        };

        // Step 2: Optional Arabic stemmer expansion
        let final_translated = if config.enable_arabic_stemmer && !translated.arabic_terms.is_empty() {
            let expanded = self.stemmer.expand_query_terms(&translated.arabic_terms);
            let mut t = translated.clone();
            t.arabic_terms = expanded;
            // Rebuild tantivy query with expanded terms
            t.tantivy_query = self.translator.build_tantivy_query_from_terms(&t.arabic_terms, &t.latin_terms);
            t
        } else {
            translated
        };

        // Step 3: Search with configurable scoring
        let results = self.search_with_eval_config(&final_translated, limit, config)?;
        Ok((results, final_translated))
    }

    /// Internal search with ablation-configurable scoring
    fn search_with_eval_config(
        &self,
        translated: &TranslatedQuery,
        limit: usize,
        config: &EvalConfig,
    ) -> Result<Vec<SearchResult>, String> {
        if !self.is_indexed() {
            return self.db.search_toc_fts(&translated.original, limit);
        }

        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![self.field_content]);

        let query_text = if !translated.tantivy_query.is_empty() {
            &translated.tantivy_query
        } else {
            &translated.original
        };

        let query = match query_parser.parse_query(query_text) {
            Ok(q) => q,
            Err(_) => {
                let fallback = translated.arabic_terms.join(" ");
                if !fallback.is_empty() {
                    query_parser
                        .parse_query(&fallback)
                        .map_err(|e| format!("Query parse error: {}", e))?
                } else {
                    query_parser
                        .parse_query(&translated.original)
                        .map_err(|e| format!("Query parse error: {}", e))?
                }
            }
        };

        let fetch_count = if config.disable_diversity_cap { limit * 2 } else { limit * 5 };
        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(fetch_count))
            .map_err(|e| format!("Search error: {}", e))?;

        let toc_counts = self.book_toc_counts.read();
        let mut all_results: Vec<SearchResult> = Vec::new();

        for (bm25_score, doc_address) in &top_docs {
            if let Ok(retrieved_doc) = searcher.doc::<TantivyDocument>(*doc_address) {
                let content = retrieved_doc
                    .get_first(self.field_content)
                    .and_then(|v: &tantivy::schema::OwnedValue| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let book_id = retrieved_doc
                    .get_first(self.field_book_id)
                    .and_then(|v: &tantivy::schema::OwnedValue| v.as_i64())
                    .unwrap_or(0);

                let toc_id = retrieved_doc
                    .get_first(self.field_toc_id)
                    .and_then(|v: &tantivy::schema::OwnedValue| v.as_i64())
                    .unwrap_or(0);

                let page = retrieved_doc
                    .get_first(self.field_page)
                    .and_then(|v: &tantivy::schema::OwnedValue| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let parent = retrieved_doc
                    .get_first(self.field_parent)
                    .and_then(|v: &tantivy::schema::OwnedValue| v.as_i64())
                    .unwrap_or(0);

                let hierarchy = self.db.get_toc_hierarchy(book_id, toc_id).unwrap_or_default();
                let (content_snippet, display_page) = self.db.get_content_snippet_with_page(book_id, &page).unwrap_or_default();
                let meta = self.db.get_book_metadata(book_id);

                // ─── Configurable scoring ───
                let adjusted_score = if config.raw_bm25_only {
                    // Raw BM25 baseline — no adjustments whatsoever
                    *bm25_score
                } else {
                    let depth_boost = if config.disable_hierarchy_boost {
                        0.0
                    } else {
                        hierarchy.len() as f32 * 0.1
                    };

                    let parent_boost = if config.disable_parent_boost {
                        0.0
                    } else if parent > 0 {
                        0.15
                    } else {
                        0.0
                    };

                    let toc_count = toc_counts.get(&book_id).copied().unwrap_or(0);
                    let size_factor = if config.disable_book_penalty {
                        1.0
                    } else if toc_count > 50_000 {
                        0.75
                    } else if toc_count > 20_000 {
                        0.85
                    } else if toc_count > 10_000 {
                        0.92
                    } else {
                        1.0
                    };

                    (bm25_score + depth_boost + parent_boost) * size_factor
                };

                let clean_title = strip_html_tags(&content);

                all_results.push(SearchResult {
                    book_id,
                    toc_id,
                    title: clean_title,
                    content_snippet,
                    page: display_page,
                    part: String::new(),
                    score: adjusted_score,
                    hierarchy,
                    book_name: meta.book_name,
                    author_name: meta.author_name,
                    source_type: "kitab".to_string(),
                    category: String::new(),
                });
            }
        }

        // Sort by adjusted score
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // Diversity cap (configurable)
        let diverse_results = if config.disable_diversity_cap {
            all_results.into_iter().take(limit).collect()
        } else {
            let mut book_count: HashMap<i64, usize> = HashMap::new();
            let mut filtered: Vec<SearchResult> = Vec::new();
            for result in all_results {
                let count = book_count.entry(result.book_id).or_insert(0);
                if *count < MAX_PER_BOOK {
                    *count += 1;
                    filtered.push(result);
                    if filtered.len() >= limit {
                        break;
                    }
                }
            }
            filtered
        };

        // Normalize scores to 0-100
        let mut final_results = diverse_results;
        if let Some(max_score) = final_results.first().map(|r| r.score) {
            if max_score > 0.0 {
                for r in &mut final_results {
                    r.score = (r.score / max_score * 100.0).min(100.0);
                }
            }
        }

        Ok(final_results)
    }

    /// Get the Arabic stemmer reference (for use in handlers)
    pub fn stemmer(&self) -> &ArabicStemmer {
        &self.stemmer
    }

    /// Get indexing status
    pub fn status(&self) -> (bool, u64) {
        let is_indexed = self.is_indexed();
        let num_docs = if is_indexed {
            self.reader.searcher().num_docs()
        } else {
            0
        };
        (is_indexed, num_docs)
    }
}

/// Strip HTML tags from a string (for cleaning TOC title content)
fn strip_html_tags(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    // Collapse multiple whitespace
    let collapsed: String = result.split_whitespace().collect::<Vec<_>>().join(" ");
    collapsed.trim().to_string()
}
