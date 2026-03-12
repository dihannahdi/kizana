use crate::arabic_stemmer::ArabicStemmer;
use crate::db::Database;
use crate::models::*;
use crate::query_translator::{QueryTranslator, TranslatedQuery};
use log::{info, warn};
use std::collections::HashMap;
use std::sync::Arc;
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, BoostQuery, FuzzyTermQuery, Occur, PhraseQuery, QueryParser, TermQuery};
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
                        // ═══ NOVEL v15: Arabic normalization at index time ═══
                        // Normalize Arabic text before indexing to ensure consistent
                        // token matching. SimpleTokenizer treats diacritics (tashkeel)
                        // as word boundaries, fragmenting Arabic tokens. By stripping
                        // diacritics, normalizing hamza variants (أ/إ/آ→ا), and
                        // removing tatweel before indexing, we get clean tokens that
                        // match query terms produced by the translator.
                        let normalized_content = self.stemmer.normalize(&entry.content);
                        let normalized_title = self.stemmer.normalize(&clean_title);
                        writer.add_document(doc!(
                            self.field_content => normalized_content,
                            self.field_title => normalized_title,
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

        // Pre-normalize query terms for scoring (avoid re-normalizing per doc)
        let normalized_phrases: Vec<String> = phrase_terms.iter()
            .map(|t| self.stemmer.normalize(t))
            .collect();
        let normalized_singles: Vec<String> = single_terms.iter()
            .map(|t| self.stemmer.normalize(t))
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

                // ─── Scoring: BM25 × relevance multiplier × size factor ───
                // ═══ NOVEL v15: Multiplicative scoring instead of additive ═══
                // Additive boosts (+0.15, +0.4, +0.3) are weak compared to BM25 scores
                // that range 0-20+. Multiplicative ensures high-BM25 docs get proportionally
                // larger benefit from structural/term-overlap signals, while low-BM25 docs
                // can't be promoted by boosts alone.

                // Hierarchy depth boost (deeper = more specific)
                let depth_boost = (hierarchy.len() as f32 * 0.08).min(0.4);
                
                // Parent relevance boost
                let parent_boost = if parent > 0 { 0.12 } else { 0.0 };

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

                // ─── Term-overlap reranking (specificity-weighted) ───
                // ═══ v15: Also normalize searchable_text for consistent matching ═══
                let normalized_searchable = self.stemmer.normalize(&format!("{} {}", title_text, content_snippet));
                
                let term_overlap_boost = {
                    let mut weighted_hits = 0.0f32;
                    let mut max_weights = 0.0f32;
                    
                    // Phrase matches: weight = word_count × 2.0 (multi-word = more specific)
                    for (i, _pt) in phrase_terms.iter().enumerate() {
                        let weight = phrase_terms[i].split_whitespace().count() as f32 * 2.0;
                        max_weights += weight;
                        if normalized_searchable.contains(&normalized_phrases[i]) {
                            weighted_hits += weight;
                        }
                    }
                    
                    // Single-word matches: weight by Arabic word length (longer = more specific)
                    for (i, st) in single_terms.iter().enumerate() {
                        let char_count = st.chars().count();
                        let weight = if char_count >= 6 { 1.5 } else if char_count >= 4 { 1.0 } else { 0.5 };
                        max_weights += weight;
                        if normalized_searchable.contains(&normalized_singles[i]) {
                            weighted_hits += weight;
                        }
                    }
                    
                    if max_weights > 0.0 {
                        (weighted_hits / max_weights) * 0.8 // up to +0.8 multiplier component
                    } else {
                        0.0
                    }
                };

                // Title match bonus: if a phrase term appears in the TOC title itself,
                // that's a very strong signal this chapter is directly about the topic
                let normalized_title_check = self.stemmer.normalize(&title_text);
                let title_match_bonus = if !normalized_phrases.is_empty() {
                    let title_hits = normalized_phrases.iter()
                        .filter(|npt| normalized_title_check.contains(npt.as_str()))
                        .count();
                    (title_hits as f32 * 0.35).min(0.7)
                } else {
                    let title_hits = normalized_singles.iter()
                        .filter(|nst| normalized_title_check.contains(nst.as_str()))
                        .count();
                    (title_hits as f32 * 0.15).min(0.45)
                };

                // ═══ v15 NOVEL: Continuous query coverage scoring ═══
                // Replaces step-function with smooth curve for more granular ranking.
                // Uses sqrt(coverage) to give diminishing returns — matching the first
                // few terms matters more than the last few.
                let query_coverage_boost = if !normalized_singles.is_empty() {
                    let total_terms = normalized_singles.len();
                    let matched_terms = normalized_singles.iter()
                        .filter(|nst| normalized_searchable.contains(nst.as_str()))
                        .count();
                    let coverage_ratio = matched_terms as f32 / total_terms as f32;
                    // Smooth curve: sqrt gives diminishing returns
                    coverage_ratio.sqrt() * 0.6
                } else {
                    0.0
                };

                // ═══ v15 NOVEL: Term proximity scoring ═══
                // When multiple query terms appear close together in the text,
                // the passage discusses their combined concept (e.g., صلاة + جماعة
                // within 100 chars = "congregational prayer"). This is a stronger
                // relevance signal than terms scattered across a long document.
                let proximity_boost = if normalized_singles.len() >= 2 {
                    let mut positions: Vec<usize> = Vec::new();
                    for nst in &normalized_singles {
                        if let Some(pos) = normalized_searchable.find(nst.as_str()) {
                            positions.push(pos);
                        }
                    }
                    if positions.len() >= 2 {
                        let min_pos = *positions.iter().min().unwrap();
                        let max_pos = *positions.iter().max().unwrap();
                        let span = max_pos - min_pos;
                        if span <= 100 {
                            0.4 // terms very close: strong topical focus
                        } else if span <= 300 {
                            0.2
                        } else if span <= 600 {
                            0.1
                        } else {
                            0.0
                        }
                    } else {
                        0.0
                    }
                } else {
                    0.0
                };

                // ═══ v15 NOVEL: Multiplicative scoring formula ═══
                // Score = BM25 × (1 + sum_of_boosts) × size_factor
                // This preserves BM25 ordering while amplifying by relevance signals
                let relevance_multiplier = 1.0 + depth_boost + parent_boost 
                    + term_overlap_boost + title_match_bonus + query_coverage_boost
                    + proximity_boost;
                let adjusted_score = bm25_score * relevance_multiplier * size_factor;

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
                    citation: String::new(),
                    similarity_score: 0.0,
                    toc_page: page.clone(),
                });
            }
        }

        // Sort by adjusted score
        all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // ─── Per-book diversity limit ───
        // ═══ v15 NOVEL: Adaptive diversity — narrow queries allow more per book ═══
        // For narrow/specific queries (few unique terms), the matching book is likely
        // the authoritative source. For broad queries, we need more book diversity.
        let effective_max_per_book = if single_terms.len() <= 2 && phrase_terms.len() <= 1 {
            MAX_PER_BOOK + 2  // narrow: allow 5 per book
        } else if single_terms.len() <= 4 {
            MAX_PER_BOOK + 1  // medium: allow 4 per book
        } else {
            MAX_PER_BOOK      // broad: standard 3 per book
        };
        let mut book_count: HashMap<i64, usize> = HashMap::new();
        let mut diverse_results: Vec<SearchResult> = Vec::new();
        
        for result in all_results {
            let count = book_count.entry(result.book_id).or_insert(0);
            if *count < effective_max_per_book {
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

    /// Build a programmatic BooleanQuery with tiered field boosting.
    /// 
    /// Boost tiers (novel: differentiated original vs. stemmer-variant scoring):
    /// - Phrase Arabic terms → PhraseQuery on content (5×) + title (8×)
    /// - Original single Arabic terms → TermQuery content (3×) + title (5×) + FuzzyTermQuery (1×)
    /// - Stemmer-variant terms → TermQuery content (1.5×) + title (2.5×)
    /// - Fallback: parse original query string
    fn build_boosted_query(
        &self,
        expanded_terms: &[String],
        original_arabic_terms: &[String],
        raw_query: &str,
    ) -> Box<dyn tantivy::query::Query> {
        let mut subqueries: Vec<(Occur, Box<dyn tantivy::query::Query>)> = Vec::new();

        // ═══ NOVEL v15: Normalize all terms to match index-time normalization ═══
        let normalized_originals: Vec<String> = original_arabic_terms.iter()
            .map(|t| self.stemmer.normalize(t))
            .collect();
        let normalized_expanded: Vec<String> = expanded_terms.iter()
            .map(|t| self.stemmer.normalize(t))
            .collect();

        // ─── Separate original terms by type ───
        let phrase_terms: Vec<&String> = normalized_originals.iter()
            .filter(|t| t.contains(' '))
            .collect();
        let original_singles: Vec<&String> = normalized_originals.iter()
            .filter(|t| !t.contains(' '))
            .collect();

        // Variant terms = stemmer-expanded terms NOT in original set
        let original_set: std::collections::HashSet<&str> = normalized_originals.iter()
            .map(|s| s.as_str())
            .collect();
        let variant_singles: Vec<&String> = normalized_expanded.iter()
            .filter(|t| !t.contains(' ') && !original_set.contains(t.as_str()))
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
                subqueries.push((Occur::Should, Box::new(BoostQuery::new(Box::new(content_phrase), 5.0))));

                // PhraseQuery on title field (boost 8×)
                let title_terms: Vec<Term> = words.iter()
                    .map(|w| Term::from_field_text(self.field_title, w))
                    .collect();
                let title_phrase = PhraseQuery::new(title_terms);
                subqueries.push((Occur::Should, Box::new(BoostQuery::new(Box::new(title_phrase), 8.0))));
            }
        }

        // ─── Original single-word terms (HIGH boost — direct translations) ───
        for term_str in &original_singles {
            // TermQuery on content (boost 3×)
            let content_term = Term::from_field_text(self.field_content, term_str);
            let content_tq = TermQuery::new(content_term, IndexRecordOption::WithFreqsAndPositions);
            subqueries.push((Occur::Should, Box::new(BoostQuery::new(Box::new(content_tq), 3.0))));

            // TermQuery on title (boost 5×)
            let title_term = Term::from_field_text(self.field_title, term_str);
            let title_tq = TermQuery::new(title_term, IndexRecordOption::WithFreqsAndPositions);
            subqueries.push((Occur::Should, Box::new(BoostQuery::new(Box::new(title_tq), 5.0))));

            // FuzzyTermQuery for Arabic words ≥ 5 chars (catches orthographic/scribal variants)
            // Edit distance 1 with transposition — safe for longer Arabic words
            if term_str.chars().count() >= 5
                && term_str.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c))
            {
                let fuzzy_term = Term::from_field_text(self.field_content, term_str);
                let fuzzy_q = FuzzyTermQuery::new(fuzzy_term, 1, true);
                subqueries.push((Occur::Should, Box::new(BoostQuery::new(Box::new(fuzzy_q), 1.0))));
            }
        }

        // ─── Variant/stemmer terms (LOWER boost — derived morphological forms) ───
        for term_str in &variant_singles {
            // TermQuery on content (boost 1.5×)
            let content_term = Term::from_field_text(self.field_content, term_str);
            let content_tq = TermQuery::new(content_term, IndexRecordOption::WithFreqsAndPositions);
            subqueries.push((Occur::Should, Box::new(BoostQuery::new(Box::new(content_tq), 1.5))));

            // TermQuery on title (boost 2.5×)
            let title_term = Term::from_field_text(self.field_title, term_str);
            let title_tq = TermQuery::new(title_term, IndexRecordOption::WithFreqsAndPositions);
            subqueries.push((Occur::Should, Box::new(BoostQuery::new(Box::new(title_tq), 2.5))));
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

        // Step 3: Search — use full boosted search pipeline unless ablation flags are set
        let is_default_config = !config.disable_book_penalty
            && !config.disable_hierarchy_boost
            && !config.disable_parent_boost
            && !config.disable_diversity_cap
            && !config.raw_bm25_only;

        let results = if is_default_config {
            // Use the same boosted query + scoring as production search
            self.search_with_translated(&final_translated, limit)?
        } else {
            // Ablation mode: simpler search for comparison
            self.search_with_eval_config(&final_translated, limit, config)?
        };
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
                    citation: String::new(),
                    similarity_score: 0.0,
                    toc_page: page.clone(),
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
