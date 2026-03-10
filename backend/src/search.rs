use crate::arabic_stemmer::ArabicStemmer;
use crate::db::Database;
use crate::models::*;
use crate::query_translator::{QueryTranslator, TranslatedQuery};
use log::{info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, BoostQuery, Occur, PhraseQuery, QueryParser, TermQuery};
use tantivy::schema::*;
use tantivy::{doc, Index, IndexWriter, ReloadPolicy, TantivyDocument, Term};
use parking_lot::RwLock;

/// Maximum results from the same book (diversity limit)
const MAX_PER_BOOK: usize = 3;

pub struct SearchEngine {
    index: Index,
    reader: tantivy::IndexReader,
    schema: Schema,
    field_content: Field,
    field_title: Field,
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

        let field_content = schema_builder.add_text_field("content", text_options.clone());
        let field_title = schema_builder.add_text_field("title", text_options);
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
            field_title,
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
                        let clean_title = strip_html_tags(&entry.content);
                        writer.add_document(doc!(
                            self.field_content => entry.content.clone(),
                            self.field_title => clean_title,
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
    /// Enhanced with: programmatic BooleanQuery, stemmer expansion, term-overlap reranking
    pub fn search_with_translated(&self, translated: &TranslatedQuery, limit: usize) -> Result<Vec<SearchResult>, String> {
        if !self.is_indexed() {
            return self.db.search_toc_fts(&translated.original, limit);
        }

        // ─── Step 1: Expand Arabic terms with stemmer variants ───
        let expanded_terms = if !translated.arabic_terms.is_empty() {
            self.stemmer.expand_query_terms(&translated.arabic_terms)
        } else {
            Vec::new()
        };

        // ─── Step 2: Build programmatic query with field boosting ───
        let query = self.build_boosted_query(&expanded_terms, &translated.arabic_terms, &translated.original);

        let searcher = self.reader.searcher();

        // Fetch more candidates for diversity filtering + reranking
        let fetch_count = limit * 7;
        let top_docs = searcher
            .search(&query, &TopDocs::with_limit(fetch_count))
            .map_err(|e| format!("Search error: {}", e))?;

        let toc_counts = self.book_toc_counts.read();

        // Collect phrase terms for term-overlap reranking
        let phrase_terms: Vec<&String> = translated.arabic_terms.iter()
            .filter(|t| t.contains(' '))
            .collect();
        let single_terms: Vec<&String> = translated.arabic_terms.iter()
            .filter(|t| !t.contains(' '))
            .collect();

        let mut all_results: Vec<SearchResult> = Vec::new();

        for (bm25_score, doc_address) in &top_docs {
            if let Ok(retrieved_doc) = searcher.doc::<TantivyDocument>(*doc_address) {
                let content = retrieved_doc
                    .get_first(self.field_content)
                    .and_then(|v: &tantivy::schema::OwnedValue| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let title_text = retrieved_doc
                    .get_first(self.field_title)
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

                let hierarchy = self
                    .db
                    .get_toc_hierarchy(book_id, toc_id)
                    .unwrap_or_default();

                let (content_snippet, display_page) = self
                    .db
                    .get_content_snippet_with_page_and_terms(book_id, &page, &translated.arabic_terms)
                    .unwrap_or_default();

                let meta = self.db.get_book_metadata(book_id);

                // ─── Scoring: BM25 + structural boosts + term-overlap reranking ───
                
                // Hierarchy depth boost (deeper = more specific)
                let depth_boost = hierarchy.len() as f32 * 0.1;
                
                // Parent relevance boost
                let parent_boost = if parent > 0 { 0.15 } else { 0.0 };

                // Large-book penalty
                let toc_count = toc_counts.get(&book_id).copied().unwrap_or(0);
                let size_factor = if toc_count > 50_000 {
                    0.75
                } else if toc_count > 20_000 {
                    0.85
                } else if toc_count > 10_000 {
                    0.92
                } else {
                    1.0
                };

                // ─── Term-overlap reranking ───
                // Check how many of the original Arabic search terms actually
                // appear in the retrieved title + content. This rewards results
                // that match the specific query intent, not just generic high-TF terms.
                let searchable_text = format!("{} {}", title_text, content_snippet);
                
                let phrase_hit_count = phrase_terms.iter()
                    .filter(|pt| searchable_text.contains(pt.as_str()))
                    .count();
                let single_hit_count = single_terms.iter()
                    .filter(|st| searchable_text.contains(st.as_str()))
                    .count();
                
                let total_query_terms = phrase_terms.len() + single_terms.len();
                let term_overlap_boost = if total_query_terms > 0 {
                    // Phrase hits count 3× more than single-word hits
                    let weighted_hits = (phrase_hit_count * 3 + single_hit_count) as f32;
                    let max_possible = (phrase_terms.len() * 3 + single_terms.len()) as f32;
                    (weighted_hits / max_possible) * 0.5 // up to +0.5 boost
                } else {
                    0.0
                };

                // Title match bonus: if a phrase term appears in the TOC title itself,
                // that's a very strong signal this chapter is directly about the topic
                let title_match_bonus = if !phrase_terms.is_empty() {
                    let title_hits = phrase_terms.iter()
                        .filter(|pt| title_text.contains(pt.as_str()))
                        .count();
                    title_hits as f32 * 0.3
                } else {
                    let title_hits = single_terms.iter()
                        .filter(|st| title_text.contains(st.as_str()))
                        .count();
                    (title_hits as f32 * 0.15).min(0.45)
                };

                let adjusted_score = (bm25_score + depth_boost + parent_boost 
                    + term_overlap_boost + title_match_bonus) * size_factor;

                let clean_title = if !title_text.is_empty() {
                    title_text
                } else {
                    strip_html_tags(&content)
                };

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

        // ─── Per-book diversity limit ───
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
            "Search: {} candidates → {} diverse results (from {} unique books), stemmer expanded {} → {} terms",
            top_docs.len(),
            diverse_results.len(),
            book_count.len(),
            translated.arabic_terms.len(),
            expanded_terms.len(),
        );

        Ok(diverse_results)
    }

    /// Build a programmatic BooleanQuery with field boosting.
    /// - Phrase Arabic terms → PhraseQuery on content (boost 5×) + title (boost 8×)
    /// - Single Arabic terms → TermQuery on content (boost 2×) + title (boost 4×)
    /// - Fallback: parse original query string
    fn build_boosted_query(
        &self,
        expanded_terms: &[String],
        original_arabic_terms: &[String],
        raw_query: &str,
    ) -> Box<dyn tantivy::query::Query> {
        let mut subqueries: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();

        // Group terms: phrases (contain spaces) vs. single words
        let phrase_terms: Vec<&String> = original_arabic_terms.iter()
            .filter(|t| t.contains(' '))
            .collect();
        let single_terms: Vec<&String> = expanded_terms.iter()
            .filter(|t| !t.contains(' '))
            .collect();

        // ─── Phrase queries (highest boost) ───
        for phrase in &phrase_terms {
            let words: Vec<&str> = phrase.split_whitespace().collect();
            if words.len() >= 2 {
                // PhraseQuery on content field (boost 5×)
                let content_terms: Vec<Term> = words.iter()
                    .map(|w| Term::from_field_text(self.field_content, w))
                    .collect();
                let content_phrase = PhraseQuery::new(content_terms);
                let boosted_content = BoostQuery::new(Box::new(content_phrase), 5.0);
                subqueries.push((Occur::Should, Box::new(boosted_content)));

                // PhraseQuery on title field (boost 8×)
                let title_terms: Vec<Term> = words.iter()
                    .map(|w| Term::from_field_text(self.field_title, w))
                    .collect();
                let title_phrase = PhraseQuery::new(title_terms);
                let boosted_title = BoostQuery::new(Box::new(title_phrase), 8.0);
                subqueries.push((Occur::Should, Box::new(boosted_title)));
            }
        }

        // ─── Single-word term queries ───
        for term_str in &single_terms {
            // TermQuery on content (boost 2×)
            let content_term = Term::from_field_text(self.field_content, term_str);
            let content_tq = TermQuery::new(content_term, IndexRecordOption::WithFreqsAndPositions);
            let boosted_content = BoostQuery::new(Box::new(content_tq), 2.0);
            subqueries.push((Occur::Should, Box::new(boosted_content)));

            // TermQuery on title (boost 4×)
            let title_term = Term::from_field_text(self.field_title, term_str);
            let title_tq = TermQuery::new(title_term, IndexRecordOption::WithFreqsAndPositions);
            let boosted_title = BoostQuery::new(Box::new(title_tq), 4.0);
            subqueries.push((Occur::Should, Box::new(boosted_title)));
        }

        if subqueries.is_empty() {
            // Fallback: use query parser on both fields
            let query_parser = QueryParser::for_index(
                &self.index,
                vec![self.field_content, self.field_title],
            );
            match query_parser.parse_query(raw_query) {
                Ok(q) => q,
                Err(_) => {
                    // Last resort: match nothing
                    Box::new(BooleanQuery::new(vec![]))
                }
            }
        } else {
            Box::new(BooleanQuery::new(subqueries))
        }
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
        let query_parser = QueryParser::for_index(&self.index, vec![self.field_content, self.field_title]);

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

                let title_text = retrieved_doc
                    .get_first(self.field_title)
                    .and_then(|v: &tantivy::schema::OwnedValue| v.as_str())
                    .unwrap_or("")
                    .to_string();

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

                let clean_title = if !title_text.is_empty() {
                    title_text
                } else {
                    strip_html_tags(&content)
                };

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
