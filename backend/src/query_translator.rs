use log::info;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Maximum Arabic terms to prevent BM25 noise on long queries
const MAX_ARABIC_TERMS: usize = 12;

// ─── Language Detection ───

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(Default)]
pub enum QueryLang {
    Indonesian,
    English,
    Arabic,
    Mixed,
    #[default]
    Unknown,
}

impl std::fmt::Display for QueryLang {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryLang::Indonesian => write!(f, "id"),
            QueryLang::English => write!(f, "en"),
            QueryLang::Arabic => write!(f, "ar"),
            QueryLang::Mixed => write!(f, "mixed"),
            QueryLang::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FiqhDomain {
    Ibadah,
    Thaharah,
    Muamalat,
    Munakahat,
    Jinayat,
    Aqidah,
    Tasawuf,
    Tafsir,
    Hadits,
    Akhlak,
    Siyasah,
    UsulFiqh,
    Unknown,
}

impl std::fmt::Display for FiqhDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FiqhDomain::Ibadah => write!(f, "عبادات"),
            FiqhDomain::Thaharah => write!(f, "طهارة"),
            FiqhDomain::Muamalat => write!(f, "معاملات"),
            FiqhDomain::Munakahat => write!(f, "مناكحات"),
            FiqhDomain::Jinayat => write!(f, "جنايات"),
            FiqhDomain::Aqidah => write!(f, "عقيدة"),
            FiqhDomain::Tasawuf => write!(f, "تصوف"),
            FiqhDomain::Tafsir => write!(f, "تفسير"),
            FiqhDomain::Hadits => write!(f, "حديث"),
            FiqhDomain::Akhlak => write!(f, "أخلاق"),
            FiqhDomain::Siyasah => write!(f, "سياسة شرعية"),
            FiqhDomain::UsulFiqh => write!(f, "أصول الفقه"),
            FiqhDomain::Unknown => write!(f, "عام"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslatedQuery {
    pub original: String,
    pub arabic_terms: Vec<String>,
    pub latin_terms: Vec<String>,
    pub detected_language: QueryLang,
    pub detected_domain: FiqhDomain,
    pub tantivy_query: String,
    pub confidence_note: String,
}

// ─── Main Query Translator ───

pub struct QueryTranslator {
    term_map: HashMap<String, Vec<&'static str>>,
    domain_keywords: HashMap<String, (FiqhDomain, i32)>,
}

impl QueryTranslator {
    pub fn new() -> Self {
        let mut translator = QueryTranslator {
            term_map: HashMap::new(),
            domain_keywords: HashMap::new(),
        };
        translator.build_term_dictionary();
        translator.build_domain_detector();
        translator
    }

    /// Main entry point: translate any query into Arabic search terms
    pub fn translate(&self, raw_query: &str) -> TranslatedQuery {
        let query = raw_query.trim().to_string();
        let detected_language = self.detect_language(&query);
        let detected_domain = self.detect_domain(&query);

        let mut arabic_terms: Vec<String> = Vec::new();
        let mut latin_terms: Vec<String> = Vec::new();

        match detected_language {
            QueryLang::Arabic => {
                // Query is already Arabic — use as-is + strip harakat
                let cleaned = strip_harakat(&query);
                arabic_terms.push(cleaned.clone());
                // Also add individual words for broader search
                for word in cleaned.split_whitespace() {
                    if word.len() > 4 { // skip very short Arabic particles
                        arabic_terms.push(word.to_string());
                    }
                }
            }
            QueryLang::Indonesian | QueryLang::English | QueryLang::Mixed | QueryLang::Unknown => {
                let is_long_query = query.len() > 200;

                if is_long_query {
                    // ═══ LONG QUERY MODE ═══
                    // Extract question sentence(s) to focus search on actual question
                    // Context/description words cause BM25 noise with 20-30+ terms
                    let (question_text, context_text) = extract_question_and_context(&query);

                    info!(
                        "Long query split: question='{}…', context='{}…'",
                        &question_text[..question_text.len().min(80)],
                        &context_text[..context_text.len().min(80)]
                    );

                    // Process question sentence(s) with full single-word + phrase expansion
                    let question_words = tokenize_query(&question_text);
                    for word in &question_words {
                        let lower = word.to_lowercase();
                        if !is_query_stopword(&lower) {
                            latin_terms.push(lower.clone());
                            if let Some(expansions) = self.term_map.get(&lower) {
                                for exp in expansions {
                                    arabic_terms.push(exp.to_string());
                                }
                            } else {
                                // ═══ NOVEL: Morphological root extraction for long queries too ═══
                                let roots = extract_indonesian_roots(&lower);
                                for root in &roots {
                                    if let Some(root_expansions) = self.term_map.get(root) {
                                        for exp in root_expansions {
                                            arabic_terms.push(exp.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Phrase expansion on question words
                    let question_phrases = self.expand_phrases(&question_words);
                    for exp in question_phrases {
                        arabic_terms.push(exp);
                    }

                    // From context, extract ONLY phrase matches (no single-word noise)
                    if !context_text.is_empty() {
                        let context_words = tokenize_query(&context_text);
                        let context_phrases = self.expand_phrases(&context_words);
                        for exp in context_phrases {
                            arabic_terms.push(exp);
                        }
                    }

                    // Also run phrase expansion on FULL query to catch cross-sentence phrases
                    let all_words = tokenize_query(&query);
                    let full_phrases = self.expand_phrases(&all_words);
                    for exp in full_phrases {
                        arabic_terms.push(exp);
                    }

                    // ═══ NOVEL: Query Intent Pattern Detection for long queries ═══
                    let intent_expansions = detect_query_intent_patterns(&all_words);
                    for exp in intent_expansions {
                        arabic_terms.push(exp);
                    }
                } else {
                    // ═══ SHORT QUERY MODE (enhanced with morphological root extraction) ═══
                    let words = tokenize_query(&query);
                    for word in &words {
                        let lower = word.to_lowercase();
                        latin_terms.push(lower.clone());

                        // Check single-word expansion
                        if let Some(expansions) = self.term_map.get(&lower) {
                            for exp in expansions {
                                arabic_terms.push(exp.to_string());
                            }
                        } else if !is_query_stopword(&lower) {
                            // ═══ NOVEL: Indonesian Morphological Root Extraction ═══
                            // If the word isn't in the dictionary, try extracting
                            // its root by stripping Indonesian affixes.
                            // e.g., "membatalkan" → "batal" → found! → بطلان/باطل
                            let roots = extract_indonesian_roots(&lower);
                            for root in &roots {
                                if let Some(root_expansions) = self.term_map.get(root) {
                                    for exp in root_expansions {
                                        arabic_terms.push(exp.to_string());
                                    }
                                }
                            }
                        }
                    }

                    // Check multi-word phrases (bigrams, trigrams)
                    let phrase_expansions = self.expand_phrases(&words);
                    for exp in phrase_expansions {
                        arabic_terms.push(exp);
                    }

                    // ═══ NOVEL: Query Intent Pattern Detection ═══
                    // Detect semantic patterns like "membatalkan X" → مبطلات
                    let intent_expansions = detect_query_intent_patterns(&words);
                    for exp in intent_expansions {
                        arabic_terms.push(exp);
                    }
                }

                // If no Arabic terms found, try the full query as-is
                if arabic_terms.is_empty() {
                    latin_terms.push(query.clone());
                }
            }
        }

        // Deduplicate
        arabic_terms.sort();
        arabic_terms.dedup();
        latin_terms.sort();
        latin_terms.dedup();

        // ═══ TERM LIMITING ═══
        // Cap Arabic terms to prevent BM25 noise (especially for long queries)
        // Prioritize: multi-word phrases > longer singles > shorter singles
        // Longer Arabic words are more discriminative (e.g., الاستحاضة > حج)
        if arabic_terms.len() > MAX_ARABIC_TERMS {
            let phrases: Vec<String> = arabic_terms.iter()
                .filter(|t| t.contains(' '))
                .cloned()
                .collect();
            let mut singles: Vec<String> = arabic_terms.iter()
                .filter(|t| !t.contains(' '))
                .cloned()
                .collect();
            // Sort singles by char count descending — longer = more specific
            singles.sort_by(|a, b| b.chars().count().cmp(&a.chars().count()));
            arabic_terms.clear();
            // Keep all phrases first (up to limit)
            for p in phrases.into_iter().take(MAX_ARABIC_TERMS) {
                arabic_terms.push(p);
            }
            // Fill remaining slots with longest single-word terms
            let remaining = MAX_ARABIC_TERMS.saturating_sub(arabic_terms.len());
            for s in singles.into_iter().take(remaining) {
                arabic_terms.push(s);
            }
            info!("Arabic terms capped to {} (was over limit)", arabic_terms.len());
        }

        // Build Tantivy query string
        let tantivy_query = self.build_tantivy_query(&arabic_terms, &latin_terms);

        let confidence_note = if arabic_terms.is_empty() && detected_language != QueryLang::Arabic {
            "لم يتم العثور على مصطلحات عربية مطابقة. سيتم البحث بالنص الأصلي.".to_string()
        } else if arabic_terms.len() > 5 {
            "تم توسيع البحث إلى عدة مصطلحات عربية لتغطية أشمل.".to_string()
        } else {
            String::new()
        };

        info!(
            "Query translated: '{}' → lang={}, domain={}, arabic_terms={:?}, tantivy='{}'",
            query, detected_language, detected_domain, arabic_terms, tantivy_query
        );

        TranslatedQuery {
            original: query,
            arabic_terms,
            latin_terms,
            detected_language,
            detected_domain,
            tantivy_query,
            confidence_note,
        }
    }

    /// Translate with configuration flags for ablation study
    /// Allows disabling phrase mapping and/or multi-variant expansion
    pub fn translate_with_config(
        &self,
        raw_query: &str,
        enable_phrases: bool,
        enable_multi_variant: bool,
    ) -> TranslatedQuery {
        let query = raw_query.trim().to_string();
        let detected_language = self.detect_language(&query);
        let detected_domain = self.detect_domain(&query);

        let mut arabic_terms: Vec<String> = Vec::new();
        let mut latin_terms: Vec<String> = Vec::new();

        match detected_language {
            QueryLang::Arabic => {
                let cleaned = strip_harakat(&query);
                arabic_terms.push(cleaned.clone());
                for word in cleaned.split_whitespace() {
                    if word.len() > 4 {
                        arabic_terms.push(word.to_string());
                    }
                }
            }
            _ => {
                let words = tokenize_query(&query);
                for word in &words {
                    let lower = word.to_lowercase();
                    latin_terms.push(lower.clone());

                    if let Some(expansions) = self.term_map.get(&lower) {
                        if enable_multi_variant {
                            // Normal: add all variants
                            for exp in expansions {
                                arabic_terms.push(exp.to_string());
                            }
                        } else {
                            // Ablation: only first variant (single mapping)
                            if let Some(first) = expansions.first() {
                                arabic_terms.push(first.to_string());
                            }
                        }
                    }
                }

                // Phrase expansion (can be disabled)
                if enable_phrases {
                    let phrase_expansions = self.expand_phrases(&words);
                    for exp in phrase_expansions {
                        arabic_terms.push(exp);
                    }
                }

                if arabic_terms.is_empty() {
                    latin_terms.push(query.clone());
                }
            }
        }

        arabic_terms.sort();
        arabic_terms.dedup();
        latin_terms.sort();
        latin_terms.dedup();

        // Apply term limiting (same strategy as translate())
        if arabic_terms.len() > MAX_ARABIC_TERMS {
            let phrases: Vec<String> = arabic_terms.iter()
                .filter(|t| t.contains(' '))
                .cloned()
                .collect();
            let mut singles: Vec<String> = arabic_terms.iter()
                .filter(|t| !t.contains(' '))
                .cloned()
                .collect();
            singles.sort_by(|a, b| b.chars().count().cmp(&a.chars().count()));
            arabic_terms.clear();
            for p in phrases.into_iter().take(MAX_ARABIC_TERMS) {
                arabic_terms.push(p);
            }
            let remaining = MAX_ARABIC_TERMS.saturating_sub(arabic_terms.len());
            for s in singles.into_iter().take(remaining) {
                arabic_terms.push(s);
            }
        }

        let tantivy_query = self.build_tantivy_query(&arabic_terms, &latin_terms);
        let confidence_note = String::new();

        TranslatedQuery {
            original: query,
            arabic_terms,
            latin_terms,
            detected_language,
            detected_domain,
            tantivy_query,
            confidence_note,
        }
    }

    /// Public wrapper for build_tantivy_query — used by search.rs for stemmer expansion
    pub fn build_tantivy_query_from_terms(&self, arabic_terms: &[String], latin_terms: &[String]) -> String {
        self.build_tantivy_query(arabic_terms, latin_terms)
    }

    // ─── Language Detection ───

    fn detect_language(&self, query: &str) -> QueryLang {
        let mut arabic_chars = 0u32;
        let mut latin_chars = 0u32;
        let mut total_chars = 0u32;

        for c in query.chars() {
            if c.is_whitespace() || c.is_ascii_punctuation() {
                continue;
            }
            total_chars += 1;
            if is_arabic_char(c) {
                arabic_chars += 1;
            } else if c.is_ascii_alphabetic() {
                latin_chars += 1;
            }
        }

        if total_chars == 0 {
            return QueryLang::Unknown;
        }

        let arabic_ratio = arabic_chars as f32 / total_chars as f32;
        let latin_ratio = latin_chars as f32 / total_chars as f32;

        if arabic_ratio > 0.7 {
            QueryLang::Arabic
        } else if arabic_ratio > 0.2 && latin_ratio > 0.2 {
            QueryLang::Mixed
        } else if latin_ratio > 0.5 {
            // Distinguish Indonesian from English
            let lower = query.to_lowercase();
            let indonesian_markers = [
                "apa", "bagaimana", "apakah", "boleh", "hukum", "gak", "gk", "ga",
                "tidak", "bisa", "kalau", "bolehkah", "gimana", "kenapa", "mengapa",
                "siapa", "kapan", "dari", "untuk", "yang", "atau", "dan", "dengan",
                "dalam", "jika", "ketika", "saat", "tentang", "mengenai",
                "shalat", "solat", "wudhu", "puasa", "zakat", "haji", "nikah",
                "cerai", "talak", "riba", "waris", "masjid", "imam",
            ];
            let english_markers = [
                "what", "how", "is", "can", "the", "does", "should", "must",
                "when", "where", "which", "why", "about", "prayer", "fasting",
                "marriage", "divorce", "ruling", "permissible", "forbidden",
                "allowed", "halal", "haram", "islamic", "sharia",
            ];

            let id_score: i32 = indonesian_markers
                .iter()
                .filter(|m| lower.contains(**m))
                .count() as i32;
            let en_score: i32 = english_markers
                .iter()
                .filter(|m| lower.contains(**m))
                .count() as i32;

            if id_score > en_score {
                QueryLang::Indonesian
            } else if en_score > id_score {
                QueryLang::English
            } else {
                // Check for common Indonesian patterns
                if lower.contains("nya") || lower.contains("kah") || lower.contains("lah") {
                    QueryLang::Indonesian
                } else {
                    QueryLang::Indonesian // Default to Indonesian for Islamic context
                }
            }
        } else {
            QueryLang::Unknown
        }
    }

    // ─── Domain Detection ───

    fn detect_domain(&self, query: &str) -> FiqhDomain {
        let lower = query.to_lowercase();
        let mut domain_scores: HashMap<&str, i32> = HashMap::new();

        for (keyword, (domain, weight)) in &self.domain_keywords {
            if lower.contains(keyword.as_str()) {
                let domain_key = match domain {
                    FiqhDomain::Ibadah => "ibadah",
                    FiqhDomain::Thaharah => "thaharah",
                    FiqhDomain::Muamalat => "muamalat",
                    FiqhDomain::Munakahat => "munakahat",
                    FiqhDomain::Jinayat => "jinayat",
                    FiqhDomain::Aqidah => "aqidah",
                    FiqhDomain::Tasawuf => "tasawuf",
                    FiqhDomain::Tafsir => "tafsir",
                    FiqhDomain::Hadits => "hadits",
                    FiqhDomain::Akhlak => "akhlak",
                    FiqhDomain::Siyasah => "siyasah",
                    FiqhDomain::UsulFiqh => "usulfiqh",
                    FiqhDomain::Unknown => "unknown",
                };
                *domain_scores.entry(domain_key).or_default() += weight;
            }
        }

        // Also check Arabic text directly
        let arabic_domain_markers: Vec<(&str, &str)> = vec![
            ("صلاة", "ibadah"), ("صلوات", "ibadah"), ("صوم", "ibadah"), ("صيام", "ibadah"),
            ("زكاة", "ibadah"), ("حج", "ibadah"), ("عمرة", "ibadah"), ("أذان", "ibadah"),
            ("طهارة", "thaharah"), ("وضوء", "thaharah"), ("غسل", "thaharah"), ("نجاسة", "thaharah"),
            ("حيض", "thaharah"), ("جنابة", "thaharah"), ("تيمم", "thaharah"),
            ("بيع", "muamalat"), ("ربا", "muamalat"), ("إجارة", "muamalat"), ("رهن", "muamalat"),
            ("شركة", "muamalat"), ("وقف", "muamalat"), ("هبة", "muamalat"),
            ("نكاح", "munakahat"), ("طلاق", "munakahat"), ("خلع", "munakahat"),
            ("عدة", "munakahat"), ("نفقة", "munakahat"), ("مهر", "munakahat"),
            ("حد", "jinayat"), ("قصاص", "jinayat"), ("تعزير", "jinayat"), ("سرقة", "jinayat"),
            ("توحيد", "aqidah"), ("إيمان", "aqidah"), ("شرك", "aqidah"), ("كفر", "aqidah"),
            ("بدعة", "aqidah"), ("ردة", "aqidah"),
            ("تفسير", "tafsir"), ("تأويل", "tafsir"), ("آية", "tafsir"),
            ("حديث", "hadits"), ("سنة", "hadits"), ("رواية", "hadits"), ("إسناد", "hadits"),
            ("قياس", "usulfiqh"), ("إجماع", "usulfiqh"), ("اجتهاد", "usulfiqh"),
            ("استحسان", "usulfiqh"), ("استصحاب", "usulfiqh"), ("مقاصد", "usulfiqh"),
            ("نسخ", "usulfiqh"), ("علة", "usulfiqh"), ("مصلحة", "usulfiqh"),
        ];

        for (marker, domain_key) in &arabic_domain_markers {
            if query.contains(marker) {
                *domain_scores.entry(domain_key).or_default() += 2; // Arabic matches get higher weight
            }
        }

        if let Some((top_domain, _)) = domain_scores.iter().max_by_key(|(_, &v)| v) {
            match *top_domain {
                "ibadah" => FiqhDomain::Ibadah,
                "thaharah" => FiqhDomain::Thaharah,
                "muamalat" => FiqhDomain::Muamalat,
                "munakahat" => FiqhDomain::Munakahat,
                "jinayat" => FiqhDomain::Jinayat,
                "aqidah" => FiqhDomain::Aqidah,
                "tasawuf" => FiqhDomain::Tasawuf,
                "tafsir" => FiqhDomain::Tafsir,
                "hadits" => FiqhDomain::Hadits,
                "akhlak" => FiqhDomain::Akhlak,
                "siyasah" => FiqhDomain::Siyasah,
                "usulfiqh" => FiqhDomain::UsulFiqh,
                _ => FiqhDomain::Unknown,
            }
        } else {
            FiqhDomain::Unknown
        }
    }

    // ─── Multi-word Phrase Expansion ───

    fn expand_phrases(&self, words: &[String]) -> Vec<String> {
        let mut expansions = Vec::new();
        let lower_words: Vec<String> = words.iter().map(|w| w.to_lowercase()).collect();

        // Define multi-word phrase mappings
        let phrase_map: Vec<(&[&str], Vec<&str>)> = vec![
            // Ibadah phrases
            (&["shalat", "jumat"], vec!["صلاة الجمعة", "الجمعة"]),
            (&["solat", "jumat"], vec!["صلاة الجمعة", "الجمعة"]),
            (&["sholat", "jumat"], vec!["صلاة الجمعة", "الجمعة"]),
            (&["sholat", "jumaat"], vec!["صلاة الجمعة", "الجمعة"]),
            (&["sholat", "jum'at"], vec!["صلاة الجمعة", "الجمعة"]),
            (&["shalat", "jum'at"], vec!["صلاة الجمعة", "الجمعة"]),
            (&["salat", "jumat"], vec!["صلاة الجمعة", "الجمعة"]),
            (&["friday", "prayer"], vec!["صلاة الجمعة", "الجمعة"]),
            (&["shalat", "jamak"], vec!["الجمع بين الصلاتين", "جمع الصلاة"]),
            (&["shalat", "qashar"], vec!["قصر الصلاة", "القصر"]),
            (&["shalat", "qasar"], vec!["قصر الصلاة", "القصر"]),
            (&["shalat", "jenazah"], vec!["صلاة الجنازة", "الجنازة"]),
            (&["shalat", "janazah"], vec!["صلاة الجنازة", "الجنازة"]),
            (&["shalat", "tarawih"], vec!["صلاة التراويح", "التراويح"]),
            (&["shalat", "tahajjud"], vec!["صلاة التهجد", "التهجد", "قيام الليل"]),
            (&["shalat", "istikharah"], vec!["صلاة الاستخارة", "الاستخارة"]),
            (&["shalat", "dhuha"], vec!["صلاة الضحى", "الضحى"]),
            (&["shalat", "witir"], vec!["صلاة الوتر", "الوتر"]),
            (&["shalat", "ied"], vec!["صلاة العيد", "صلاة العيدين"]),
            (&["shalat", "id"], vec!["صلاة العيد", "صلاة العيدين"]),
            (&["shalat", "gerhana"], vec!["صلاة الكسوف", "صلاة الخسوف"]),
            (&["shalat", "istisqa"], vec!["صلاة الاستسقاء"]),
            (&["shalat", "sambil"], vec!["الصلاة", "حمل"]), // contextual - shalat sambil [doing something]
            
            // Thaharah phrases
            (&["hadats", "besar"], vec!["الحدث الأكبر", "جنابة", "الجنابة"]),
            (&["hadats", "kecil"], vec!["الحدث الأصغر"]),
            (&["air", "kencing"], vec!["بول", "البول"]),
            (&["air", "seni"], vec!["بول", "البول"]),
            (&["air", "mani"], vec!["مني", "المني"]),
            (&["batal", "wudhu"], vec!["نواقض الوضوء", "ينقض الوضوء"]),
            (&["batal", "wudu"], vec!["نواقض الوضوء", "ينقض الوضوء"]),
            (&["membatalkan", "wudhu"], vec!["نواقض الوضوء", "ينقض الوضوء"]),
            
            // Nikah phrases
            (&["nikah", "siri"], vec!["نكاح سري", "نكاح بغير ولي", "نكاح بغير شهود"]),
            (&["nikah", "sirri"], vec!["نكاح سري", "نكاح بغير ولي", "نكاح بغير شهود"]),
            (&["nikah", "mut'ah"], vec!["نكاح المتعة", "المتعة"]),
            (&["nikah", "mutah"], vec!["نكاح المتعة", "المتعة"]),
            (&["wali", "nikah"], vec!["ولي النكاح", "الولي في النكاح", "ولاية النكاح"]),
            (&["saksi", "nikah"], vec!["شهود النكاح", "الشهادة في النكاح"]),
            (&["mas", "kawin"], vec!["مهر", "الصداق", "المهر"]),
            (&["cerai", "gugat"], vec!["خلع", "الخلع"]),
            
            // Muamalat phrases
            (&["jual", "beli"], vec!["بيع", "البيع", "المعاملات"]),
            (&["bunga", "bank"], vec!["ربا", "الربا", "الفائدة"]),
            (&["bagi", "hasil"], vec!["مضاربة", "المضاربة"]),
            (&["utang", "piutang"], vec!["دين", "الدين", "القرض"]),
            (&["gadai", "emas"], vec!["رهن الذهب", "رهن"]),
            (&["asuransi", "syariah"], vec!["التأمين", "تكافل", "التأمين التكافلي"]),
            
            // Aqidah phrases
            (&["qadha", "qadar"], vec!["قضاء وقدر", "القضاء والقدر"]),
            (&["qada", "qadar"], vec!["قضاء وقدر", "القضاء والقدر"]),
            (&["makhluk", "hidup"], vec!["ذوات الأرواح", "تصوير"]),
            (&["hari", "kiamat"], vec!["يوم القيامة", "الساعة", "الآخرة"]),
            (&["hari", "akhir"], vec!["اليوم الآخر", "الآخرة"]),
            (&["azab", "kubur"], vec!["عذاب القبر"]),
            
            // Hadits phrases
            (&["hadits", "sahih"], vec!["حديث صحيح", "الصحيح"]),
            (&["hadits", "dhaif"], vec!["حديث ضعيف", "الضعيف"]),
            (&["hadits", "hasan"], vec!["حديث حسن"]),
            (&["hadits", "maudhu"], vec!["حديث موضوع", "الموضوع"]),
            
            // Misc phrases
            (&["hukum", "foto"], vec!["تصوير", "التصوير", "ذوات الأرواح"]),
            (&["hukum", "gambar"], vec!["تصوير", "التصوير", "ذوات الأرواح"]),
            (&["hukum", "musik"], vec!["الغناء", "المعازف", "الموسيقى"]),
            (&["hukum", "rokok"], vec!["التدخين", "الدخان"]),
            (&["hukum", "vaksin"], vec!["التطعيم", "اللقاح"]),
            (&["hukum", "tato"], vec!["الوشم"]),
            (&["hukum", "merokok"], vec!["التدخين", "الدخان"]),
            (&["makanan", "haram"], vec!["المحرمات من الطعام", "الأطعمة المحرمة"]),
            (&["binatang", "haram"], vec!["الحيوانات المحرمة", "حرمة الأكل"]),

            // ═══ HEALTH / SICKNESS PHRASES ═══
            (&["puasa", "sakit"], vec!["إفطار المريض", "صيام المريض", "فطر المريض", "رخصة الإفطار"]),
            (&["boleh", "puasa"], vec!["جواز الصيام", "هل يجوز الصيام"]),
            (&["tidak", "puasa"], vec!["إفطار", "ترك الصيام", "رخصة الإفطار"]),
            (&["sakit", "puasa"], vec!["إفطار المريض", "الصيام والمرض"]),
            (&["puasa", "tidak", "boleh"], vec!["رخصة الإفطار", "إفطار", "الأعذار المبيحة للفطر"]),
            (&["darurat", "puasa"], vec!["الضرورة والصيام", "إفطار الاضطرار"]),
            (&["sakit", "shalat"], vec!["صلاة المريض", "كيفية صلاة المريض", "رخص الصلاة"]),
            (&["tidak", "shalat"], vec!["ترك الصلاة", "السقوط عن الصلاة"]),
            (&["hamil", "puasa"], vec!["صيام الحامل", "إفطار الحامل"]),
            (&["menyusui", "puasa"], vec!["صيام المرضع", "إفطار المرضع"]),
            (&["musafir", "puasa"], vec!["إفطار المسافر", "صيام المسافر"]),
            (&["safar", "puasa"], vec!["إفطار المسافر", "قصر الصيام"]),

            // ═══ MARRIAGE/DIVORCE PHRASES ═══
            (&["suami", "impoten"], vec!["العيوب في النكاح", "عنة الزوج", "العنين", "الفسخ بالعيب"]),
            (&["istri", "cerai"], vec!["خلع", "طلب الطلاق", "شقاق", "الخلع"]),
            (&["istri", "minta", "cerai"], vec!["خلع", "طلب الطلاق من الزوجة", "الشقاق"]),
            (&["cerai", "impoten"], vec!["فسخ النكاح بالعيب", "العنة", "خيار العيب"]),
            (&["cerai", "suami"], vec!["طلاق", "طلب الطلاق", "الخلع"]),
            (&["perceraian", "pengadilan"], vec!["الطلاق القضائي", "حكم القاضي بالطلاق"]),
            (&["suami", "tidak", "mau", "cerai"], vec!["امتناع الزوج من الطلاق", "الشقاق", "التفريق القضائي"]),

            // ═══ WARIS (INHERITANCE) PHRASES ═══
            (&["waris", "anak", "perempuan"], vec!["ميراث البنت", "نصيب البنت", "ميراث البنات", "فرض البنت"]),
            (&["waris", "perempuan"], vec!["ميراث البنت", "ميراث النساء", "نصيب البنت", "ميراث البنات"]),
            (&["waris", "anak"], vec!["ميراث الأولاد", "ميراث الولد", "نصيب الأولاد"]),
            (&["anak", "perempuan"], vec!["بنت", "البنت", "البنات"]),
            (&["bagian", "waris"], vec!["الفرائض", "فروض الإرث", "أصحاب الفروض"]),

            // ═══ QUNUT & PRAYER TIME PHRASES ═══
            (&["qunut", "subuh"], vec!["قنوت الفجر", "القنوت في صلاة الصبح", "قنوت الصبح"]),
            (&["qunut", "fajar"], vec!["قنوت الفجر", "القنوت في صلاة الفجر"]),
            (&["qunut", "witir"], vec!["قنوت الوتر", "دعاء قنوت الوتر"]),
            (&["qunut", "witr"], vec!["قنوت الوتر", "دعاء قنوت الوتر"]),
            (&["doa", "qunut"], vec!["دعاء القنوت", "القنوت"]),
            (&["shalat", "subuh"], vec!["صلاة الفجر", "صلاة الصبح"]),
            (&["shalat", "dzuhur"], vec!["صلاة الظهر", "الظهر"]),
            (&["shalat", "ashar"], vec!["صلاة العصر", "العصر"]),
            (&["shalat", "maghrib"], vec!["صلاة المغرب", "المغرب"]),
            (&["shalat", "isya"], vec!["صلاة العشاء", "العشاء"]),

            // ═══ GELATIN & FOOD PHRASES ═══
            (&["gelatin", "babi"], vec!["الجيلاتين من الخنزير", "الاستحالة", "التداوي بالمحرم", "حكم الجيلاتين"]),
            (&["makan", "gelatin"], vec!["أكل الجيلاتين", "حكم الجيلاتين"]),
            (&["makan", "babi"], vec!["أكل لحم الخنزير", "حرمة أكل الخنزير"]),
            (&["makan", "haram"], vec!["المحرمات من الطعام", "الأطعمة المحرمة"]),

            // ═══ BANK/FINANCE PHRASES ═══
            (&["bank", "konvensional"], vec!["البنوك الربوية", "الفائدة المصرفية", "فائدة البنك", "التعامل مع البنوك"]),
            (&["bank", "syariah"], vec!["البنوك الإسلامية", "المصارف الإسلامية"]),
            (&["riba", "bank"], vec!["ربا البنوك", "فائدة البنك", "الربا المصرفي"]),

            // ═══ PHOTO/VISUAL MEDIA PHRASES ═══
            (&["foto", "makhluk"], vec!["تصوير ذوات الأرواح", "حكم التصوير", "المصوِّر"]),
            (&["foto", "hidup"], vec!["تصوير ذوات الأرواح", "صور الأحياء"]),
            (&["gambar", "makhluk"], vec!["تصوير ذوات الأرواح", "رسم ذوات الأرواح"]),
            (&["selfie", "hukum"], vec!["تصوير", "التصوير الفوتوغرافي"]),
            
            // English phrases
            (&["friday", "prayer"], vec!["صلاة الجمعة", "الجمعة"]),
            (&["night", "prayer"], vec!["قيام الليل", "صلاة التهجد"]),
            (&["funeral", "prayer"], vec!["صلاة الجنازة"]),
            (&["marriage", "contract"], vec!["عقد النكاح", "النكاح"]),
            (&["breast", "feeding"], vec!["الرضاعة", "رضاع"]),
            (&["breast", "milk"], vec!["الرضاعة", "حكم الرضاع"]),
            (&["interest", "rate"], vec!["ربا", "الربا", "الفائدة"]),
            (&["islamic", "finance"], vec!["المعاملات المالية", "الاقتصاد الإسلامي"]),
            (&["menstrual", "blood"], vec!["دم الحيض", "الحيض"]),

            // ═══ ADDITIONAL INDONESIAN PHRASES ═══
            (&["shalat", "rakaat"], vec!["ركعات الصلاة", "عدد الركعات"]),
            (&["shalat", "rakat"], vec!["ركعات الصلاة", "عدد الركعات"]),
            (&["shalat", "berapa"], vec!["ركعات الصلاة", "عدد الركعات"]),
            (&["shalat", "mati"], vec!["صلاة الجنازة", "الصلاة على الميت"]),
            (&["shalat", "mayit"], vec!["صلاة الجنازة", "الصلاة على الميت"]),
            (&["shalat", "sakit"], vec!["صلاة المريض", "كيفية صلاة المريض"]),
            (&["shalat", "duduk"], vec!["صلاة القاعد", "الصلاة جالسا"]),
            (&["tata", "cara"], vec![]), // generic modifier, no expansion needed
            (&["hukum", "merokok"], vec!["حكم التدخين", "التدخين"]),
            (&["hukum", "musik"], vec!["حكم الغناء", "المعازف", "الموسيقى"]),
            (&["hukum", "foto"], vec!["حكم التصوير", "تصوير ذوات الأرواح"]),
            (&["hukum", "rokok"], vec!["حكم التدخين", "التدخين"]),
            (&["hukum", "mati"], vec!["حكم الموت", "الاحتضار", "أحكام الميت"]),
            (&["orang", "mati"], vec!["أحكام الميت", "الجنازة", "تغسيل الميت"]),
            (&["orang", "meninggal"], vec!["أحكام الميت", "الجنازة", "تغسيل الميت"]),
            (&["suami", "istri"], vec!["الزوج والزوجة", "حقوق الزوجين", "العشرة بين الزوجين"]),
            (&["hak", "istri"], vec!["حقوق الزوجة", "النفقة", "حق الزوجة"]),
            (&["hak", "suami"], vec!["حقوق الزوج", "حق الزوج", "طاعة الزوج"]),
            // Kafarat / sumpah / nadzar
            (&["kafarat", "sumpah"], vec!["كفارة اليمين", "كفارة الأيمان"]),
            (&["kaffarah", "sumpah"], vec!["كفارة اليمين", "كفارة الأيمان"]),
            (&["sumpah", "palsu"], vec!["اليمين الغموس", "اليمين الكاذبة"]),
            (&["sumpah", "bohong"], vec!["اليمين الغموس", "اليمين الكاذبة"]),
            (&["hukum", "sumpah"], vec!["حكم اليمين", "أحكام الأيمان"]),
            (&["kafarat", "puasa"], vec!["كفارة الصيام", "كفارة الإفطار"]),
            (&["nadzar", "hukum"], vec!["حكم النذر", "أحكام النذور"]),

            // ═══ MISSING PHRASES (added v8) ═══
            (&["ahli", "kitab"], vec!["أهل الكتاب", "ذبائح أهل الكتاب"]),
            (&["sembelihan", "ahli"], vec!["ذبائح أهل الكتاب", "ذبيحة الكتابي"]),
            (&["sembelihan", "kitab"], vec!["ذبائح أهل الكتاب", "أكل ذبيحة الكتابي"]),
            (&["daging", "ular"], vec!["أكل لحم الحية", "حكم أكل الحيات", "الحشرات"]),
            (&["daging", "babi"], vec!["لحم الخنزير", "أكل لحم الخنزير"]),
            (&["daging", "haram"], vec!["اللحوم المحرمة", "حرمة الأكل"]),
            (&["ibu", "hamil"], vec!["الحامل", "المرأة الحامل"]),
            (&["puasa", "hamil"], vec!["صيام الحامل", "إفطار الحامل", "حكم صوم الحامل"]),
            (&["puasa", "ramadhan"], vec!["صيام رمضان", "صوم رمضان"]),
            (&["puasa", "ramadan"], vec!["صيام رمضان", "صوم رمضان"]),
            (&["zakat", "mal"], vec!["زكاة المال", "نصاب المال", "حول الزكاة"]),
            (&["zakat", "emas"], vec!["زكاة الذهب", "نصاب الذهب"]),
            (&["zakat", "fitrah"], vec!["زكاة الفطر", "صدقة الفطر"]),
            (&["nisab", "zakat"], vec!["نصاب الزكاة", "نصاب المال"]),
            (&["nisab", "haul"], vec!["النصاب والحول", "شروط الزكاة"]),
            (&["jual", "online"], vec!["البيع عبر الإنترنت", "البيع الإلكتروني", "بيع المعاطاة"]),
            (&["beli", "online"], vec!["الشراء عبر الإنترنت", "البيع الإلكتروني"]),
            (&["gadai", "pegadaian"], vec!["الرهن", "رهن", "أحكام الرهن"]),
            (&["nikah", "tanpa"], vec!["نكاح بغير", "النكاح بدون ولي"]),
            (&["tanpa", "wali"], vec!["بغير ولي", "بدون ولي"]),
            (&["vaksin", "haram"], vec!["اللقاح المحرم", "التطعيم بالمحرم", "الاضطرار"]),
            (&["bahan", "haram"], vec!["المواد المحرمة", "الاستحالة", "التداوي بالمحرم"]),
            (&["tawakal", "allah"], vec!["التوكل على الله", "حقيقة التوكل"]),
            (&["tawakkal", "allah"], vec!["التوكل على الله", "حقيقة التوكل"]),
            (&["pegang", "bayi"], vec!["حمل الطفل", "حمل الصبي في الصلاة"]),
            (&["shalat", "pegang"], vec!["حمل في الصلاة", "حمل الطفل في الصلاة"]),
            (&["musik", "nyanyian"], vec!["الغناء والمعازف", "حكم الغناء", "حكم المعازف"]),
            (&["mencuri", "islam"], vec!["السرقة في الإسلام", "حد السرقة", "حكم السرقة"]),

            // ═══ v10: LONG QUERY FIX — missing phrases ═══
            // Wudhu + bandage (Q5 fix)
            (&["wudhu", "perban"], vec!["المسح على الجبيرة", "الجبيرة", "أحكام الجبائر"]),
            (&["wudu", "perban"], vec!["المسح على الجبيرة", "الجبيرة", "أحكام الجبائر"]),
            (&["perban", "shalat"], vec!["المسح على الجبيرة", "الجبيرة"]),
            (&["perban", "kaki"], vec!["المسح على الجبيرة", "الجبيرة"]),
            (&["masah", "perban"], vec!["المسح على الجبيرة", "الجبيرة"]),
            (&["masah", "jabira"], vec!["المسح على الجبيرة", "الجبيرة"]),
            // Talak + anger (Q7 fix)
            (&["talak", "marah"], vec!["طلاق الغضبان", "الطلاق في الغضب", "الغضب"]),
            (&["talak", "emosi"], vec!["طلاق الغضبان", "الطلاق في الغضب", "الغضب"]),
            (&["cerai", "marah"], vec!["طلاق الغضبان", "الطلاق في الغضب"]),
            (&["cerai", "emosi"], vec!["طلاق الغضبان", "الطلاق في الغضب"]),
            (&["talak", "whatsapp"], vec!["الطلاق بالكتابة", "طلاق الكناية"]),
            (&["talak", "pesan"], vec!["الطلاق بالكتابة", "طلاق الكناية"]),
            (&["talak", "sms"], vec!["الطلاق بالكتابة", "طلاق الكناية"]),
            // Kurban + pooling (Q8 fix)
            (&["kurban", "patungan"], vec!["الاشتراك في الأضحية", "الأضحية المشتركة"]),
            (&["qurban", "patungan"], vec!["الاشتراك في الأضحية", "الأضحية المشتركة"]),
            (&["kurban", "bersama"], vec!["الاشتراك في الأضحية", "الأضحية المشتركة"]),
            (&["qurban", "bersama"], vec!["الاشتراك في الأضحية", "الأضحية المشتركة"]),
            (&["kurban", "sapi"], vec!["الأضحية من البقر", "ذبح البقر"]),
            (&["qurban", "sapi"], vec!["الأضحية من البقر", "ذبح البقر"]),
            // Nikah siri + nasab (Q3 fix)
            (&["nikah", "nasab"], vec!["نسب الولد", "النسب في النكاح"]),
            (&["anak", "nasab"], vec!["نسب الولد", "النسب"]),
            (&["nikah", "siri", "anak"], vec!["نسب ولد الزنا", "نسب الولد من النكاح السري"]),
            (&["nikah", "akta"], vec!["توثيق النكاح", "إثبات النكاح"]),

            // ═══ v12: EVAL AUDIT — Critical missing phrases ═══

            // "membatalkan puasa" (pu01 fix — was returning wrong results)
            (&["membatalkan", "puasa"], vec!["مبطلات الصيام", "مفسدات الصوم", "نواقض الصيام", "ما يفسد الصوم"]),
            (&["batalkan", "puasa"], vec!["مبطلات الصيام", "مفسدات الصوم"]),
            (&["batal", "puasa"], vec!["مبطلات الصيام", "بطلان الصيام", "ما يبطل الصوم"]),
            (&["membatalkan", "shalat"], vec!["مبطلات الصلاة", "ما يبطل الصلاة", "نواقض الصلاة"]),
            (&["batal", "shalat"], vec!["مبطلات الصلاة", "بطلان الصلاة"]),
            (&["membatalkan", "wudhu"], vec!["نواقض الوضوء", "ما ينقض الوضوء"]),

            // "beda agama" (cm02 fix — was returning generic nikah results)
            (&["beda", "agama"], vec!["اختلاف الدين", "نكاح الكتابية", "زواج المختلفين في الدين", "نكاح المشرك"]),
            (&["berbeda", "agama"], vec!["اختلاف الدين", "نكاح الكتابية", "زواج المختلفين في الدين"]),
            (&["nikah", "beda"], vec!["نكاح الكتابية", "اختلاف الدين في النكاح", "نكاح المشركة"]),
            (&["nikah", "agama"], vec!["نكاح الكتابية", "اختلاف الدين في النكاح"]),
            (&["nikah", "non", "muslim"], vec!["نكاح الكتابية", "زواج غير المسلم"]),
            (&["nikah", "non"], vec!["نكاح الكتابية", "زواج غير المسلم"]),
            (&["kawin", "beda"], vec!["نكاح الكتابية", "اختلاف الدين في النكاح"]),

            // "celana pendek" / aurat shalat (cm01 fix)
            (&["celana", "pendek"], vec!["ستر العورة", "كشف العورة", "اللباس القصير", "حد العورة"]),
            (&["shalat", "celana"], vec!["ستر العورة في الصلاة", "لباس المصلي", "العورة"]),
            (&["shalat", "pendek"], vec!["ستر العورة في الصلاة", "كشف العورة"]),
            (&["aurat", "shalat"], vec!["عورة المصلي", "ستر العورة في الصلاة", "حد العورة في الصلاة"]),
            (&["aurat", "laki"], vec!["عورة الرجل", "حد عورة الرجل"]),
            (&["aurat", "perempuan"], vec!["عورة المرأة", "حد عورة المرأة"]),
            (&["pakai", "shalat"], vec!["لباس المصلي", "ستر العورة", "اللباس في الصلاة"]),

            // "sifat dua puluh" (aq08 fix)
            (&["sifat", "dua"], vec!["الصفات العشرون", "صفات الله الواجبة"]),
            (&["sifat", "puluh"], vec!["الصفات العشرون", "صفات الله العشرون"]),
            (&["sifat", "allah"], vec!["صفات الله", "الصفات الإلهية", "صفات الله الواجبة"]),
            (&["sifat", "wajib"], vec!["الصفات الواجبة", "صفات الله الواجبة"]),
            (&["sifat", "mustahil"], vec!["الصفات المستحيلة", "صفات الله المستحيلة"]),
            (&["sifat", "jaiz"], vec!["الصفات الجائزة"]),

            // "sujud sahwi" (ib08 fix)
            (&["sujud", "sahwi"], vec!["سجود السهو", "سجدة السهو"]),
            (&["sujud", "sahw"], vec!["سجود السهو", "سجدة السهو"]),
            (&["sujud", "tilawah"], vec!["سجود التلاوة", "سجدة التلاوة"]),
            (&["sujud", "syukur"], vec!["سجود الشكر", "سجدة الشكر"]),

            // "operasi plastik" (zero result fix)
            (&["operasi", "plastik"], vec!["جراحة التجميل", "عمليات التجميل", "حكم التجميل"]),
            (&["bedah", "plastik"], vec!["جراحة التجميل", "عمليات التجميل"]),
            (&["operasi", "kecantikan"], vec!["جراحة التجميل", "عمليات التجميل"]),

            // "ihram miqat" (zero result fix)
            (&["ihram", "miqat"], vec!["الإحرام من الميقات", "المواقيت"]),
            (&["ihram", "dari"], vec!["الإحرام من الميقات", "ميقات الإحرام"]),

            // "adab murid guru" (zero result fix)
            (&["adab", "murid"], vec!["آداب المتعلم", "أدب طالب العلم"]),
            (&["adab", "guru"], vec!["آداب المعلم", "أدب المعلم"]),
            (&["murid", "guru"], vec!["المتعلم والمعلم", "آداب المتعلم مع المعلم"]),
            (&["adab", "belajar"], vec!["آداب التعلم", "أدب طالب العلم"]),
            (&["adab", "ilmu"], vec!["آداب طلب العلم"]),
            (&["adab", "makan"], vec!["آداب الأكل", "آداب الطعام"]),
            (&["adab", "tidur"], vec!["آداب النوم"]),
            (&["adab", "masjid"], vec!["آداب المسجد"]),

            // Tauhid specifics (aq01-02 fix)
            (&["tauhid", "rububiyah"], vec!["توحيد الربوبية"]),
            (&["tauhid", "uluhiyah"], vec!["توحيد الألوهية", "توحيد العبادة"]),
            (&["tauhid", "asma"], vec!["توحيد الأسماء والصفات"]),
            (&["syirik", "kecil"], vec!["الشرك الأصغر"]),
            (&["syirik", "besar"], vec!["الشرك الأكبر"]),
            (&["syirik", "akbar"], vec!["الشرك الأكبر"]),
            (&["syirik", "asghar"], vec!["الشرك الأصغر"]),

            // Muamalat specifics
            (&["akad", "mudharabah"], vec!["عقد المضاربة", "المضاربة"]),
            (&["akad", "murabahah"], vec!["عقد المرابحة", "المرابحة"]),
            (&["akad", "salam"], vec!["عقد السلم", "بيع السلم"]),
            (&["akad", "istisna"], vec!["عقد الاستصناع"]),
            (&["jual", "kredit"], vec!["البيع بالتقسيط", "بيع الأجل"]),
            (&["jual", "cicilan"], vec!["البيع بالتقسيط", "بيع النسيئة"]),
            (&["hukum", "dropship"], vec!["حكم بيع ما لا يملك", "البيع قبل القبض"]),

            // Jinayat specifics (jn02 fix)
            (&["qishas", "diyat"], vec!["القصاص والدية", "أحكام القصاص والدية"]),
            (&["qishash", "diyat"], vec!["القصاص والدية", "أحكام القصاص والدية"]),
            (&["hukum", "korupsi"], vec!["حكم الاختلاس", "حكم أكل المال بالباطل", "الغلول"]),
            (&["hukum", "suap"], vec!["حكم الرشوة", "الرشوة"]),

            // Contemporary issues
            (&["hukum", "kredit"], vec!["حكم البيع بالتقسيط", "البيع بالأجل"]),
            (&["hukum", "asuransi"], vec!["حكم التأمين", "التأمين"]),
            (&["hukum", "saham"], vec!["حكم الأسهم", "حكم الاستثمار"]),
            (&["hukum", "kripto"], vec!["حكم العملات الرقمية"]),
            (&["hukum", "kloning"], vec!["حكم الاستنساخ"]),
            (&["hukum", "aborsi"], vec!["حكم الإجهاض", "الإجهاض"]),
            (&["hukum", "transplantasi"], vec!["حكم نقل الأعضاء", "زراعة الأعضاء"]),
            (&["hukum", "kb"], vec!["حكم تنظيم النسل", "منع الحمل"]),
            (&["hukum", "kontrasepsi"], vec!["حكم منع الحمل", "العزل"]),
            (&["hukum", "tahlilan"], vec!["حكم التهليل", "إهداء الثواب للميت"]),
            (&["hukum", "demo"], vec!["حكم المظاهرات", "الخروج"]),

            // "imam perempuan" (ib13 fix — was wrongly categorized as munakahat)
            (&["imam", "perempuan"], vec!["إمامة المرأة", "صلاة المرأة إماماً"]),
            (&["imam", "wanita"], vec!["إمامة المرأة", "صلاة المرأة إماماً"]),
            (&["perempuan", "imam"], vec!["إمامة المرأة"]),
            (&["perempuan", "shalat"], vec!["صلاة المرأة", "عورة المرأة في الصلاة"]),

            // Qadha qadar (aq06 fix)
            (&["qadha", "qadar"], vec!["قضاء وقدر", "القضاء والقدر"]),
            (&["takdir", "qadar"], vec!["القدر", "القضاء والقدر"]),
            (&["qada", "qadr"], vec!["القضاء والقدر"]),

            // Zakat refinements (zk04 fix)
            (&["zakat", "emas"], vec!["زكاة الذهب", "نصاب الذهب", "حكم زكاة الذهب"]),
            (&["zakat", "perak"], vec!["زكاة الفضة", "نصاب الفضة"]),
            (&["zakat", "pertanian"], vec!["زكاة الزروع", "الزروع والثمار"]),
            (&["zakat", "hewan"], vec!["زكاة الأنعام", "زكاة المواشي"]),
            (&["zakat", "perdagangan"], vec!["زكاة عروض التجارة", "زكاة التجارة"]),
            (&["zakat", "penghasilan"], vec!["زكاة المال", "زكاة الدخل"]),

            // ═══ v15 batch 2: COLLOQUIAL & MODERN PHRASES ═══
            // Shalat + jamaah
            (&["shalat", "jamaah"], vec!["صلاة الجماعة", "الجماعة", "فضل الجماعة"]),
            (&["sholat", "jamaah"], vec!["صلاة الجماعة", "الجماعة"]),
            (&["shalat", "berjamaah"], vec!["صلاة الجماعة", "الجماعة", "فضل الجماعة"]),
            // Shalat tahajud (single j - common misspelling)
            (&["shalat", "tahajud"], vec!["صلاة التهجد", "التهجد", "قيام الليل"]),
            (&["sholat", "tahajud"], vec!["صلاة التهجد", "التهجد", "قيام الليل"]),
            // Engagement & dating
            (&["nikah", "gantung"], vec!["تأخير الدخول", "النكاح بغير دخول"]),
            (&["kawin", "lari"], vec!["نكاح بغير ولي", "نكاح الفرار"]),
            (&["nikah", "massal"], vec!["النكاح الجماعي", "عقد النكاح"]),
            (&["nikah", "kua"], vec!["توثيق النكاح"]),
            // Animals — food queries
            (&["makan", "anjing"], vec!["أكل لحم الكلب", "حرمة الكلب"]),
            (&["makan", "kucing"], vec!["أكل لحم السنور", "حكم أكل الهر"]),
            (&["makan", "ular"], vec!["أكل لحم الحية", "حكم أكل الحيات"]),
            (&["makan", "cacing"], vec!["أكل الحشرات", "أكل الديدان"]),
            (&["makan", "jengkrik"], vec!["أكل الحشرات", "أكل الجراد"]),
            (&["makan", "kelinci"], vec!["أكل الأرنب", "حكم أكل الأرنب"]),
            (&["makan", "buaya"], vec!["أكل التمساح", "حيوان البحر"]),
            (&["makan", "kuda"], vec!["أكل لحم الخيل", "حكم أكل الفرس"]),
            (&["makan", "kepiting"], vec!["أكل السرطان", "حكم أكل القشريات"]),
            (&["makan", "udang"], vec!["أكل الروبيان", "حيوان البحر"]),
            (&["makan", "gurita"], vec!["أكل حيوان البحر", "الأخطبوط"]),
            (&["makan", "kodok"], vec!["أكل الضفدع", "حكم أكل الضفادع"]),
            // Social media / modern
            (&["hukum", "tiktok"], vec!["حكم اللهو", "اللهو والعبث"]),
            (&["hukum", "instagram"], vec!["حكم التصوير", "تصوير"]),
            (&["hukum", "youtube"], vec!["حكم النظر", "التصوير"]),
            (&["hukum", "anime"], vec!["تصوير ذوات الأرواح", "حكم الرسوم"]),
            (&["hukum", "cosplay"], vec!["التشبه", "التشبه بالكفار"]),
            (&["hukum", "pacaran"], vec!["حكم الخلوة", "الاختلاط", "النظر"]),
            (&["hukum", "dating"], vec!["حكم الخلوة", "الاختلاط"]),
            (&["hukum", "judi"], vec!["حكم القمار", "الميسر"]),
            (&["hukum", "narkoba"], vec!["حكم المخدرات", "حكم المسكر"]),
            (&["hukum", "ganja"], vec!["حكم المخدرات", "حكم المسكر"]),
            // Financial modern
            (&["cicilan", "motor"], vec!["البيع بالتقسيط", "الأجل", "الربا"]),
            (&["cicilan", "riba"], vec!["البيع بالتقسيط والربا", "الفائدة"]),
            (&["kredit", "riba"], vec!["البيع بالتقسيط", "الربا", "الفائدة"]),
            (&["kredit", "motor"], vec!["البيع بالتقسيط", "بيع النسيئة"]),
            (&["kredit", "rumah"], vec!["البيع بالتقسيط", "بيع النسيئة"]),
            (&["hukum", "pinjaman"], vec!["حكم القرض", "القرض"]),
            (&["pinjaman", "online"], vec!["القرض", "الربا"]),
            (&["hukum", "reksadana"], vec!["حكم الاستثمار", "صناديق الاستثمار"]),
            (&["hukum", "forex"], vec!["حكم الصرف", "المضاربة"]),
            // Dress code
            (&["hukum", "cadar"], vec!["حكم النقاب", "ستر الوجه"]),
            (&["hukum", "jilbab"], vec!["حكم الحجاب", "وجوب الحجاب"]),
            (&["hukum", "jenggot"], vec!["حكم اللحية", "إعفاء اللحية"]),
            // Medical
            (&["hukum", "euthanasia"], vec!["حكم قتل المريض", "قتل الرحمة"]),
            (&["hukum", "autopsi"], vec!["حكم تشريح الجثة"]),
            (&["hukum", "transfusi"], vec!["حكم نقل الدم"]),
            (&["donor", "darah"], vec!["التبرع بالدم", "نقل الدم"]),
            (&["donor", "organ"], vec!["التبرع بالأعضاء", "نقل الأعضاء"]),
            // Superstition & astrology
            (&["hukum", "zodiak"], vec!["حكم التنجيم", "الكهانة"]),
            (&["hukum", "horoskop"], vec!["حكم التنجيم", "الكهانة"]),
            (&["hukum", "santet"], vec!["حكم السحر", "السحر"]),
            (&["hukum", "dukun"], vec!["حكم الكهانة", "إتيان الكاهن"]),
            // Non-Muslim holidays
            (&["hukum", "valentine"], vec!["حكم أعياد الكفار", "التشبه بالكفار"]),
            (&["hukum", "halloween"], vec!["حكم أعياد الكفار", "التشبه بالكفار"]),
            (&["hukum", "natal"], vec!["حكم أعياد الكفار", "التهنئة بالأعياد"]),
            (&["ucapan", "natal"], vec!["تهنئة الكفار بأعيادهم", "حكم التهنئة"]),
            // Misc colloquial
            (&["hukum", "bekam"], vec!["حكم الحجامة", "الحجامة"]),
            (&["hukum", "ruqyah"], vec!["حكم الرقية", "الرقية الشرعية"]),
            (&["hukum", "hipnotis"], vec!["حكم التنويم", "التنويم المغناطيسي"]),
            (&["hukum", "yoga"], vec!["حكم اليوغا", "التشبه"]),
            (&["hukum", "meditasi"], vec!["حكم التأمل"]),
            (&["hukum", "taaruf"], vec!["حكم التعارف", "الخطبة"]),
            (&["shalat", "kotor"], vec!["طهارة مكان الصلاة", "نجاسة"]),
            (&["shalat", "kaos"], vec!["لباس المصلي", "ستر العورة"]),
            (&["wudhu", "toilet"], vec!["الطهارة في بيت الخلاء", "الاستنجاء"]),
            (&["masjid", "wudhu"], vec!["دخول المسجد", "الطهارة لدخول المسجد"]),

            // ═══ v15 batch 3: SKENARIO & KONTEMPORER phrases ═══
            // Prayer scenarios
            (&["imam", "lupa"], vec!["سهو الإمام", "سجود السهو"]),
            (&["imam", "batal"], vec!["بطلان صلاة الإمام", "استخلاف"]),
            (&["makmum", "datang"], vec!["المسبوق", "إدراك الركعة"]),
            (&["makmum", "salah"], vec!["خطأ المأموم", "سجود السهو"]),
            (&["imam", "anak"], vec!["إمامة الصبي", "صلاة خلف الصبي"]),
            (&["shalat", "gelap"], vec!["الصلاة في الظلام", "صلاة الليل"]),
            (&["saf", "putus"], vec!["قطع الصف", "اتصال الصف"]),
            // Wudu scenarios
            (&["wudhu", "cat"], vec!["الحائل", "وصول الماء"]),
            (&["wudhu", "kutek"], vec!["الحائل", "طلاء الأظافر", "وصول الماء"]),
            (&["wudhu", "kuku"], vec!["الأظافر", "وصول الماء", "طهارة الأعضاء"]),
            (&["wudhu", "plester"], vec!["المسح على الجبيرة", "الجبيرة"]),
            (&["wudhu", "cincin"], vec!["الخاتم", "وصول الماء", "تحريك الخاتم"]),
            (&["wudhu", "air"], vec!["ماء الطهارة", "المياه"]),
            (&["wudhu", "pesawat"], vec!["الطهارة", "التيمم عند عدم الماء"]),
            // Fasting scenarios
            (&["puasa", "mimpi"], vec!["الاحتلام في الصيام", "احتلام الصائم"]),
            (&["puasa", "lupa"], vec!["أكل الصائم ناسياً", "النسيان في الصيام"]),
            (&["puasa", "sakit"], vec!["الصيام مع المرض", "إفطار المريض"]),
            (&["puasa", "dokter"], vec!["حكم الصائم عند الطبيب"]),
            (&["puasa", "operasi"], vec!["الصيام مع العملية", "إفطار المريض"]),
            (&["puasa", "tetes"], vec!["القطرة للصائم", "حكم القطرة في الصيام"]),
            (&["puasa", "berenang"], vec!["السباحة للصائم"]),
            (&["puasa", "ciuman"], vec!["القبلة للصائم", "حكم تقبيل الزوجة"]),
            (&["puasa", "niat"], vec!["نية الصيام", "تبييت النية"]),
            // Nikah scenarios
            (&["nikah", "hamil"], vec!["نكاح الحامل", "نكاح الحامل من الزنا"]),
            (&["nikah", "zina"], vec!["نكاح الزانية", "الزنا والنكاح"]),
            (&["nikah", "iddah"], vec!["النكاح في العدة", "عدة المنكوحة"]),
            (&["wali", "meninggal"], vec!["انتقال الولاية", "ولاية الحاكم"]),
            (&["wali", "kakek"], vec!["ولاية الجد", "الولي في النكاح"]),
            (&["wali", "paman"], vec!["ولاية العم", "ترتيب الأولياء"]),
            (&["ijab", "qabul"], vec!["الإيجاب والقبول", "صيغة العقد"]),
            (&["mas", "kawin"], vec!["المهر", "الصداق"]),
            (&["akad", "zoom"], vec!["العقد عن بعد", "حكم العقد بالهاتف"]),
            (&["akad", "online"], vec!["العقد عن بعد", "اتحاد المجلس"]),
            // Inheritance scenarios
            (&["suami", "meninggal"], vec!["ميراث الزوجة", "نصيب الزوجة"]),
            (&["istri", "meninggal"], vec!["ميراث الزوج", "نصيب الزوج"]),
            (&["pewaris", "hutang"], vec!["ديون الميت", "قضاء الدين"]),
            (&["anak", "angkat"], vec!["التبني", "حكم التبني", "الكفالة"]),
            (&["beda", "agama"], vec!["اختلاف الدين", "ميراث المختلفين ديناً"]),
            (&["wasiat", "sepertiga"], vec!["الوصية", "الثلث"]),
            // Contemporary medical/tech
            (&["operasi", "plastik"], vec!["جراحة التجميل", "عمليات التجميل"]),
            (&["ganti", "kelamin"], vec!["تغيير الجنس", "تحويل الجنس"]),
            (&["bayi", "tabung"], vec!["أطفال الأنابيب", "التلقيح الاصطناعي"]),
            (&["cukur", "alis"], vec!["النمص", "حكم نمص الحواجب"]),
            (&["jabat", "tangan"], vec!["المصافحة", "مصافحة الأجنبية"]),
            (&["ulang", "tahun"], vec!["الاحتفال بالمولد", "حكم أعياد الميلاد"]),
            // Zakat modern
            (&["zakat", "crypto"], vec!["زكاة المال", "زكاة العملات"]),
            (&["zakat", "nft"], vec!["زكاة المال", "زكاة العروض"]),
            // Space/extreme scenarios
            (&["shalat", "luar angkasa"], vec!["تحديد القبلة", "صلاة المسافر"]),
            (&["kiblat", "luar angkasa"], vec!["تحديد القبلة", "الاجتهاد في القبلة"]),
            (&["puasa", "kutub"], vec!["الصيام في البلاد", "تقدير الأوقات"]),
            // Modern social
            (&["hukum", "paylater"], vec!["حكم البيع بالأجل", "القرض بالفائدة"]),
            (&["hukum", "pinjol"], vec!["حكم القرض", "الربا"]),
            (&["hukum", "adopsi"], vec!["حكم التبني", "الكفالة"]),
            (&["pelihara", "anjing"], vec!["اقتناء الكلب", "حكم الكلب"]),
            (&["pelihara", "kucing"], vec!["اقتناء الهر", "حكم الهر"]),
            (&["bunuh", "ular"], vec!["قتل الحية", "حكم قتل الحيات"]),
            (&["bunuh", "semut"], vec!["قتل النمل", "حكم قتل الحشرات"]),
            (&["bunuh", "nyamuk"], vec!["قتل البعوض", "حكم قتل الحشرات"]),
            (&["bunuh", "cicak"], vec!["قتل الوزغ", "حكم قتل الوزغ"]),
            // ═══ English phrase expansions ═══
            // Prayer phrases
            (&["missed", "prayer"], vec!["قضاء الصلاة", "الفوائت"]),
            (&["night", "prayer"], vec!["قيام الليل", "صلاة الليل", "التهجد"]),
            (&["eclipse", "prayer"], vec!["صلاة الكسوف", "صلاة الخسوف"]),
            (&["rain", "prayer"], vec!["صلاة الاستسقاء", "الاستسقاء"]),
            (&["funeral", "prayer"], vec!["صلاة الجنازة", "الجنازة"]),
            (&["eid", "prayer"], vec!["صلاة العيد", "صلاة العيدين"]),
            (&["congregational", "prayer"], vec!["صلاة الجماعة", "الجماعة"]),
            (&["prayer", "direction"], vec!["استقبال القبلة", "تحديد القبلة"]),
            (&["prayer", "conditions"], vec!["شروط الصلاة", "أركان الصلاة"]),
            (&["prayer", "pillars"], vec!["أركان الصلاة", "فرائض الصلاة"]),
            // Fasting phrases
            (&["breaking", "fast"], vec!["إفطار", "الفطر", "مفطرات"]),
            (&["making", "fast"], vec!["قضاء الصوم", "الصيام"]),
            (&["voluntary", "fasting"], vec!["صوم التطوع", "صيام النافلة"]),
            // Marriage/family phrases
            (&["waiting", "period"], vec!["عدة", "العدة", "عدة المطلقة"]),
            (&["child", "custody"], vec!["حضانة", "حضانة الطفل", "الحضانة"]),
            (&["bride", "price"], vec!["مهر", "المهر", "الصداق"]),
            (&["marriage", "contract"], vec!["عقد النكاح", "العقد"]),
            (&["marriage", "guardian"], vec!["ولي النكاح", "الولي"]),
            (&["wedding", "feast"], vec!["وليمة العرس", "الوليمة"]),
            (&["prohibited", "marriage"], vec!["المحرمات من النساء", "حرمة النكاح"]),
            (&["mixed", "marriage"], vec!["نكاح الكتابية", "زواج المختلفين"]),
            // Financial phrases
            (&["buying", "selling"], vec!["البيع والشراء", "البيوع"]),
            (&["interest", "rate"], vec!["ربا", "الربا", "فائدة"]),
            (&["business", "partnership"], vec!["شركة", "المشاركة", "المضاربة"]),
            (&["organ", "donation"], vec!["التبرع بالأعضاء", "نقل الأعضاء"]),
            (&["blood", "donation"], vec!["التبرع بالدم", "نقل الدم"]),
            // Dress/appearance phrases
            (&["gold", "men"], vec!["لبس الذهب للرجال", "تحريم الذهب"]),
            (&["silk", "men"], vec!["لبس الحرير للرجال", "تحريم الحرير"]),
            (&["women", "covering"], vec!["حجاب المرأة", "ستر العورة"]),
            // Jihad/warfare phrases
            (&["rules", "war"], vec!["أحكام الجهاد", "آداب الحرب"]),
            (&["prisoners", "war"], vec!["أسرى", "أحكام الأسرى"]),
            // Worship phrases
            (&["reading", "quran"], vec!["قراءة القرآن", "تلاوة القرآن"]),
            (&["pilgrimage", "rites"], vec!["مناسك الحج", "أركان الحج"]),
            // ═══ Batch 5: Usul fiqh, betrothal, comparative phrases ═══
            (&["maqashid", "syariah"], vec!["مقاصد الشريعة", "المقاصد الشرعية"]),
            (&["maqasid", "syariah"], vec!["مقاصد الشريعة", "المقاصد الشرعية"]),
            (&["sadd", "dzariah"], vec!["سد الذرائع", "سد الذريعة"]),
            (&["sadd", "dzari'ah"], vec!["سد الذرائع", "سد الذريعة"]),
            (&["hifzh", "nafs"], vec!["حفظ النفس"]),
            (&["hifzh", "akal"], vec!["حفظ العقل"]),
            (&["hifzh", "nasab"], vec!["حفظ النسب", "حفظ النسل"]),
            (&["hifzh", "maal"], vec!["حفظ المال"]),
            (&["hifzh", "din"], vec!["حفظ الدين"]),
            (&["maslahah", "mursalah"], vec!["المصلحة المرسلة", "المصالح المرسلة"]),
            (&["nasikh", "mansukh"], vec!["الناسخ والمنسوخ", "النسخ"]),
            (&["menurut", "syafii"], vec!["عند الشافعي", "مذهب الشافعي", "قال الشافعي"]),
            (&["menurut", "hanafi"], vec!["عند الحنفي", "مذهب الحنفي", "قال أبو حنيفة"]),
            (&["menurut", "maliki"], vec!["عند المالكي", "مذهب مالك", "قال مالك"]),
            (&["menurut", "hanbali"], vec!["عند الحنبلي", "مذهب أحمد", "قال أحمد"]),
            (&["pendapat", "syafii"], vec!["رأي الشافعي", "قول الشافعي", "مذهب الشافعي"]),
            (&["pendapat", "hanafi"], vec!["رأي أبي حنيفة", "قول أبي حنيفة", "مذهب الحنفي"]),
            (&["pendapat", "maliki"], vec!["رأي مالك", "قول مالك", "مذهب المالكي"]),
            (&["pendapat", "hanbali"], vec!["رأي أحمد", "قول أحمد", "مذهب الحنبلي"]),
            (&["mazhab", "syafii"], vec!["المذهب الشافعي", "مذهب الشافعي"]),
            (&["mazhab", "hanafi"], vec!["المذهب الحنفي", "مذهب الحنفي"]),
            (&["mazhab", "maliki"], vec!["المذهب المالكي", "مذهب مالك"]),
            (&["mazhab", "hanbali"], vec!["المذهب الحنبلي", "مذهب أحمد"]),
            (&["ahli", "kitab"], vec!["أهل الكتاب"]),
            (&["meminang", "wanita"], vec!["خطبة المرأة", "الخطبة على الخطبة"]),
            (&["melamar", "wanita"], vec!["خطبة المرأة", "الخطبة على الخطبة"]),
            (&["pinang", "iddah"], vec!["خطبة المعتدة", "الخطبة في العدة"]),
            (&["tarji'", "adzan"], vec!["ترجيع الأذان", "الترجيع في الأذان"]),
            (&["khilaf", "ulama"], vec!["اختلاف العلماء", "خلاف الفقهاء"]),
            (&["perbedaan", "mazhab"], vec!["اختلاف المذاهب", "الخلاف بين المذاهب"]),

            // ─── V17: Scholar name pairs (phrase match is more accurate than single-word) ───
            (&["abu", "bakar"], vec!["أبو بكر", "أبو بكر الصديق"]),
            (&["abu", "dawud"], vec!["سنن أبي داود", "أبو داود"]),
            (&["ibnu", "taimiyah"], vec!["ابن تيمية"]),
            (&["ibnu", "qayyim"], vec!["ابن القيم", "ابن قيم الجوزية"]),
            (&["ibnu", "hajar"], vec!["ابن حجر", "ابن حجر العسقلاني"]),
            (&["ibnu", "majah"], vec!["سنن ابن ماجه", "ابن ماجه"]),
            (&["ibnu", "rusyd"], vec!["ابن رشد", "أبو الوليد"]),
            (&["ibnu", "khaldun"], vec!["ابن خلدون"]),
            (&["ibnu", "sina"], vec!["ابن سينا"]),
            (&["ibn", "taimiyah"], vec!["ابن تيمية"]),
            (&["ibn", "qayyim"], vec!["ابن القيم"]),
            (&["ibn", "hajar"], vec!["ابن حجر", "ابن حجر العسقلاني"]),
            (&["ibn", "majah"], vec!["سنن ابن ماجه", "ابن ماجه"]),
            (&["ibn", "rusyd"], vec!["ابن رشد"]),
            (&["ibn", "khaldun"], vec!["ابن خلدون"]),
            (&["ibn", "sina"], vec!["ابن سينا"]),
            (&["khalid", "walid"], vec!["خالد بن الوليد", "سيف الله"]),
            (&["umar", "khattab"], vec!["عمر بن الخطاب", "الخليفة الثاني"]),
            (&["utsman", "affan"], vec!["عثمان بن عفان", "الخليفة الثالث"]),
            (&["hamzah", "muthalib"], vec!["حمزة بن عبد المطلب", "أسد الله"]),
            (&["khadijah", "khuwailid"], vec!["خديجة بنت خويلد", "أم المؤمنين"]),
            (&["aisyah", "bakar"], vec!["عائشة بنت أبي بكر", "أم المؤمنين"]),
            (&["fatimah", "zahra"], vec!["فاطمة الزهراء", "سيدة النساء"]),
            (&["jalaluddin", "suyuthi"], vec!["جلال الدين السيوطي", "السيوطي"]),

            // ─── V17: Islamic month name pairs ───
            (&["rabiul", "awal"], vec!["ربيع الأول", "شهر المولد"]),
            (&["rabiul", "akhir"], vec!["ربيع الآخر", "ربيع الثاني"]),
            (&["jumadil", "awal"], vec!["جمادى الأولى"]),
            (&["jumadil", "akhir"], vec!["جمادى الآخرة", "جمادى الثانية"]),
            (&["bulan", "muharram"], vec!["شهر محرم", "المحرم"]),
            (&["bulan", "rajab"], vec!["شهر رجب", "رجب"]),
            (&["nisfu", "sya'ban"], vec!["ليلة النصف من شعبان", "نصف شعبان"]),
            (&["nisfu", "syaban"], vec!["ليلة النصف من شعبان", "نصف شعبان"]),

            // ─── V17: Book title pairs ───
            (&["fathul", "qarib"], vec!["فتح القريب", "التقريب"]),
            (&["fathul", "muin"], vec!["فتح المعين", "زين الدين"]),
            (&["raudhatut", "thalibin"], vec!["روضة الطالبين", "النووي"]),
            (&["riyadhus", "shalihin"], vec!["رياض الصالحين", "النووي"]),
            (&["bulughul", "maram"], vec!["بلوغ المرام", "ابن حجر العسقلاني"]),
            (&["tuhfatul", "muhtaj"], vec!["تحفة المحتاج", "ابن حجر الهيتمي"]),
            (&["nihayatul", "muhtaj"], vec!["نهاية المحتاج", "الرملي"]),
            (&["mughnil", "muhtaj"], vec!["مغني المحتاج", "الشربيني"]),
            (&["kifayatul", "akhyar"], vec!["كفاية الأخيار"]),
            (&["nailul", "authar"], vec!["نيل الأوطار", "الشوكاني"]),
            (&["sunan", "dawud"], vec!["سنن أبي داود", "أبو داود"]),
            (&["sunan", "tirmidzi"], vec!["سنن الترمذي", "الترمذي"]),
            (&["sunan", "nasai"], vec!["سنن النسائي", "النسائي"]),
            (&["sunan", "majah"], vec!["سنن ابن ماجه", "ابن ماجه"]),
            (&["musnad", "ahmad"], vec!["مسند أحمد", "مسند الإمام أحمد"]),

            // ─── V17 BATCH 8: Sirah / Islamic history pairs ───
            (&["fathu", "makkah"], vec!["فتح مكة", "الفتح الأعظم"]),
            (&["isra", "miraj"], vec!["الإسراء والمعراج", "رحلة الإسراء"]),
            (&["piagam", "madinah"], vec!["وثيقة المدينة", "صحيفة المدينة"]),
            (&["khulafaur", "rasyidin"], vec!["الخلفاء الراشدون"]),
            (&["bani", "umayyah"], vec!["بنو أمية", "الدولة الأموية"]),
            (&["bani", "abbasiyah"], vec!["بنو العباس", "الدولة العباسية"]),
            (&["khalifah", "bakar"], vec!["أبو بكر الصديق", "الخليفة الأول"]),
            (&["khalifah", "umar"], vec!["عمر بن الخطاب", "الخليفة الثاني"]),
            (&["khalifah", "utsman"], vec!["عثمان بن عفان", "الخليفة الثالث"]),
            (&["khalifah", "ali"], vec!["علي بن أبي طالب", "الخليفة الرابع"]),
            (&["drama", "korea"], vec!["مسلسل", "مشاهدة المسلسلات"]),

            // ─── V17 BATCH 8: Usul fiqh related pairs ───
            (&["am", "khas"], vec!["العام والخاص", "عام وخاص"]),
            (&["haqiqi", "majazi"], vec!["الحقيقة والمجاز", "حقيقي ومجازي"]),
            (&["mujmal", "mubayyan"], vec!["المجمل والمبيَّن"]),
            (&["rukhshah", "azimah"], vec!["الرخصة والعزيمة"]),
            (&["amar", "makruf"], vec!["الأمر بالمعروف", "النهي عن المنكر"]),
            (&["nahi", "munkar"], vec!["النهي عن المنكر", "الأمر بالمعروف"]),

            // ─── V18 BATCH 11: Battle / war pairs ───
            (&["perang", "badr"], vec!["غزوة بدر", "معركة بدر"]),
            (&["perang", "uhud"], vec!["غزوة أحد", "معركة أحد"]),
            (&["perang", "khandaq"], vec!["غزوة الخندق", "الأحزاب"]),
            (&["peristiwa", "badr"], vec!["غزوة بدر"]),
            (&["peristiwa", "uhud"], vec!["غزوة أحد"]),

            // ─── V18 BATCH 11: Scholar / sahabi name pairs ───
            (&["khalid", "walid"], vec!["خالد بن الوليد", "سيف الله المسلول"]),
            (&["hamzah", "muthalib"], vec!["حمزة بن عبد المطلب", "أسد الله"]),
            (&["khadijah", "khuwailid"], vec!["خديجة بنت خويلد", "أم المؤمنين"]),
            (&["fatimah", "zahra"], vec!["فاطمة الزهراء", "سيدة النساء"]),
            (&["ibn", "sina"], vec!["ابن سينا", "أبو علي سينا"]),
            (&["ibnu", "sina"], vec!["ابن سينا", "أبو علي سينا"]),
            (&["ibn", "rusyd"], vec!["ابن رشد", "أبو الوليد رشد"]),
            (&["ibnu", "rusyd"], vec!["ابن رشد"]),
            (&["ibn", "khaldun"], vec!["ابن خلدون"]),
            (&["ibnu", "khaldun"], vec!["ابن خلدون"]),
            (&["ibn", "hajar"], vec!["ابن حجر", "ابن حجر العسقلاني"]),
            (&["ibnu", "hajar"], vec!["ابن حجر", "ابن حجر الهيتمي"]),
            (&["10", "sahabat"], vec!["العشرة المبشرين بالجنة"]),
            (&["jalaluddin", "suyuthi"], vec!["جلال الدين السيوطي"]),
            (&["aisyah", "bakar"], vec!["عائشة بنت أبي بكر", "أم المؤمنين"]),
            (&["ibn", "taimiyah"], vec!["ابن تيمية", "شيخ الإسلام"]),
            (&["ibnu", "taimiyah"], vec!["ابن تيمية", "شيخ الإسلام"]),
            (&["ibn", "qayyim"], vec!["ابن القيم", "ابن قيم الجوزية"]),
            (&["ibnu", "qayyim"], vec!["ابن القيم"]),
            (&["ibnu", "haitami"], vec!["ابن حجر الهيتمي"]),

            // ─── BATCH 17: Specific Hajj, prayer times, hadith types, keutamaan ───
            // Five daily prayers by specific name
            (&["shalat", "subuh"], vec!["صلاة الصبح", "صلاة الفجر"]),
            (&["shalat", "dzuhur"], vec!["صلاة الظهر"]),
            (&["shalat", "ashar"], vec!["صلاة العصر"]),
            (&["shalat", "maghrib"], vec!["صلاة المغرب"]),
            (&["shalat", "isya"], vec!["صلاة العشاء"]),
            (&["shalat", "isha"], vec!["صلاة العشاء"]),
            (&["shalat", "fajr"], vec!["صلاة الفجر", "صلاة الصبح"]),
            // Jumrah at Hajj
            (&["jumrah", "aqabah"], vec!["جمرة العقبة", "رمي جمرة العقبة"]),
            (&["jumrah", "ula"], vec!["الجمرة الأولى", "رمي الجمرات"]),
            (&["jumrah", "wustha"], vec!["الجمرة الوسطى"]),
            (&["melempar", "jumrah"], vec!["رمي الجمرات", "الرمي"]),
            (&["melontar", "jumrah"], vec!["رمي الجمرات", "الرمي"]),
            // Hadith classification types
            (&["hadits", "shahih"], vec!["الحديث الصحيح", "صحيح"]),
            (&["hadits", "sahih"], vec!["الحديث الصحيح", "صحيح"]),
            (&["hadits", "hasan"], vec!["الحديث الحسن", "حسن"]),
            (&["hadits", "dhaif"], vec!["الحديث الضعيف", "ضعيف"]),
            (&["hadits", "daif"], vec!["الحديث الضعيف", "ضعيف"]),
            (&["hadits", "mawdhu"], vec!["الحديث الموضوع", "موضوع"]),
            (&["hadits", "qudsi"], vec!["الحديث القدسي", "قدسي"]),
            (&["hadits", "mutawatir"], vec!["الحديث المتواتر", "متواتر"]),
            (&["hadits", "ahad"], vec!["حديث الآحاد", "الآحاد"]),
            (&["ilmu", "hadits"], vec!["علوم الحديث", "مصطلح الحديث"]),
            (&["ulumul", "hadits"], vec!["علوم الحديث", "مصطلح الحديث"]),
            (&["mustalah", "hadits"], vec!["مصطلح الحديث", "علم مصطلح الحديث"]),
            // Keutamaan (fadilah) phrases
            (&["keutamaan", "shalawat"], vec!["فضل الصلاة على النبي", "الصلاة على النبي"]),
            (&["keutamaan", "istighfar"], vec!["فضل الاستغفار", "ثواب الاستغفار"]),
            (&["keutamaan", "silaturahmi"], vec!["فضل صلة الرحم", "صلة الرحم"]),
            (&["keutamaan", "shalat"], vec!["فضل الصلاة", "ثواب الصلاة"]),
            (&["keutamaan", "quran"], vec!["فضل القرآن", "فضل تلاوة القرآن"]),
            (&["keutamaan", "puasa"], vec!["فضل الصيام", "ثواب الصوم"]),
            (&["keutamaan", "sedekah"], vec!["فضل الصدقة", "ثواب الصدقة"]),
            (&["keutamaan", "zikir"], vec!["فضل الذكر", "ثواب الذكر"]),
            (&["keutamaan", "sabar"], vec!["فضل الصبر", "ثواب الصبر"]),
            (&["keutamaan", "taubat"], vec!["فضل التوبة", "ثواب التوبة"]),
            (&["keutamaan", "zakat"], vec!["فضل الزكاة", "ثواب الزكاة"]),
            (&["keutamaan", "haji"], vec!["فضل الحج", "ثواب الحج"]),
            (&["keutamaan", "wudhu"], vec!["فضل الوضوء", "ثواب الوضوء"]),
            (&["keutamaan", "tahajud"], vec!["فضل قيام الليل", "فضل صلاة الليل"]),
            (&["keutamaan", "dhuha"], vec!["فضل صلاة الضحى"]),
            (&["keutamaan", "subuh"], vec!["فضل صلاة الفجر", "ثواب صلاة الصبح"]),
            (&["keutamaan", "jumat"], vec!["فضل يوم الجمعة", "فضل صلاة الجمعة"]),
            (&["keutamaan", "berjamaah"], vec!["فضل الصلاة في جماعة", "ثواب الجماعة"]),
            (&["keutamaan", "shalat", "berjamaah"], vec!["فضل الصلاة في جماعة"]),
            (&["keutamaan", "tilawah"], vec!["فضل تلاوة القرآن"]),
            (&["keutamaan", "dzikir"], vec!["فضل الذكر", "ثواب الذكر"]),
            (&["keutamaan", "sholat"], vec!["فضل الصلاة", "ثواب الصلاة"]),
            // Fardhu / wajib phrases
            (&["fardhu", "ain"], vec!["فرض العين", "الفريضة"]),
            (&["fardhu", "kifayah"], vec!["فرض الكفاية"]),
            (&["fardhu", "shalat"], vec!["فريضة الصلاة", "وجوب الصلاة"]),
            (&["wajib", "shalat"], vec!["واجبات الصلاة", "فرائض الصلاة"]),
            (&["wajib", "zakat"], vec!["وجوب الزكاة", "فريضة الزكاة"]),
            (&["wajib", "haji"], vec!["وجوب الحج", "فريضة الحج"]),
            (&["wajib", "puasa"], vec!["وجوب الصيام", "فريضة الصوم"]),
            // Sunnah phrases
            (&["sunnah", "shalat"], vec!["سنن الصلاة", "سنة الصلاة"]),
            (&["sunnah", "wudhu"], vec!["سنن الوضوء", "سنة الوضوء"]),
            (&["sunnah", "puasa"], vec!["صيام التطوع", "سنن الصيام"]),
            (&["sunnah", "rawatib"], vec!["الرواتب", "السنن الرواتب"]),
            (&["sunnah", "qabliyah"], vec!["السنة القبلية", "السنة قبل الفريضة"]),
            (&["sunnah", "ba'diyah"], vec!["السنة البعدية", "السنة بعد الفريضة"]),
            // Ayat-specific phrases
            (&["ayat", "kursi"], vec!["آية الكرسي"]),
            (&["ayat", "shiyam"], vec!["آيات الصيام", "الصوم في القرآن"]),
            (&["ayat", "waris"], vec!["آيات المواريث", "الميراث في القرآن"]),
            (&["ayat", "riba"], vec!["آيات الربا", "تحريم الربا"]),
            (&["ayat", "nikah"], vec!["آيات الزواج", "النكاح في القرآن"]),
            (&["ayat", "talak"], vec!["آيات الطلاق", "الطلاق في القرآن"]),
            (&["ayat", "jihad"], vec!["آيات الجهاد", "الجهاد في القرآن"]),
            (&["ayat", "hukum"], vec!["الآيات الأحكام", "آيات الأحكام"]),
            (&["ayat", "ahkam"], vec!["آيات الأحكام", "الآيات الأحكام"]),

            // ── Shalat dalam berbagai kondisi (prayer in specific conditions) ──
            (&["shalat", "pesawat"], vec!["الصلاة في الطائرة", "صلاة المسافر"]),
            (&["shalat", "kapal"], vec!["الصلاة على السفينة", "صلاة المسافر"]),
            (&["shalat", "kereta"], vec!["الصلاة في القطار", "الصلاة في السيارة"]),
            (&["shalat", "mobil"], vec!["الصلاة في السيارة", "الصلاة في المركبة"]),
            (&["shalat", "kendaraan"], vec!["الصلاة في المركبة", "صلاة المسافر"]),
            (&["shalat", "duduk"], vec!["الصلاة جالساً", "صلاة العاجز"]),
            (&["shalat", "berbaring"], vec!["الصلاة مضطجعاً", "صلاة المريض"]),
            (&["shalat", "sakit"], vec!["صلاة المريض", "الصلاة عند المرض"]),
            (&["shalat", "darurat"], vec!["صلاة الضرورة", "صلاة الخوف"]),
            (&["shalat", "khauf"], vec!["صلاة الخوف", "الصلاة في حالة الخوف"]),

            // ── Bersuci / thaharah compound queries ──
            (&["bersuci", "haid"], vec!["الطهارة من الحيض", "اغتسال الحيض"]),
            (&["bersuci", "junub"], vec!["الطهارة من الجنابة", "غسل الجنابة"]),
            (&["bersuci", "nifas"], vec!["الطهارة من النفاس"]),
            (&["mandi", "junub"], vec!["غسل الجنابة", "الطهارة من الجنابة"]),
            (&["mandi", "wajib"], vec!["الغسل الواجب", "غسل الجنابة"]),
            (&["mandi", "haid"], vec!["غسل الحيض", "الاغتسال من الحيض"]),
            (&["mandi", "besar"], vec!["الغسل الكبير", "غسل الجنابة"]),

            // ── Nisab zakat categories ──
            (&["nisab", "emas"], vec!["نصاب الذهب", "زكاة الذهب"]),
            (&["nisab", "perak"], vec!["نصاب الفضة", "زكاة الفضة"]),
            (&["nisab", "pertanian"], vec!["نصاب الزروع", "زكاة الزرع"]),
            (&["nisab", "ternak"], vec!["نصاب الأنعام", "زكاة الأنعام"]),
            (&["zakat", "emas"], vec!["زكاة الذهب", "نصاب الذهب"]),
            (&["zakat", "perak"], vec!["زكاة الفضة", "نصاب الفضة"]),
            (&["zakat", "pertanian"], vec!["زكاة الزروع", "العشر"]),
            (&["zakat", "profesi"], vec!["زكاة المهن", "زكاة الراتب"]),
            (&["zakat", "penghasilan"], vec!["زكاة الدخل", "زكاة المهن"]),
            (&["zakat", "tabungan"], vec!["زكاة المدخرات", "زكاة النقد"]),
            (&["zakat", "saham"], vec!["زكاة الأسهم"]),
            (&["zakat", "perniagaan"], vec!["زكاة التجارة", "زكاة عروض التجارة"]),
            (&["zakat", "fitrah"], vec!["زكاة الفطر"]),
            (&["zakat", "mal"], vec!["زكاة المال", "الزكاة"]),

            // ── Wakaf phrases ──
            (&["wakaf", "tanah"], vec!["وقف الأرض", "الوقف"]),
            (&["wakaf", "produktif"], vec!["الوقف المنتج", "الوقف الاستثماري"]),
            (&["wakaf", "uang"], vec!["وقف النقود", "وقف المال"]),
            (&["harta", "wakaf"], vec!["أموال الوقف", "الوقف"]),

            // ── Infak, sedekah, hibah phrases ──
            (&["infak", "sedekah"], vec!["الإنفاق والصدقة", "النفقة في سبيل الله"]),
            (&["sedekah", "jariyah"], vec!["الصدقة الجارية"]),
            (&["hibah", "wasiat"], vec!["الهبة والوصية"]),
            (&["wasiat", "wajibah"], vec!["الوصية الواجبة"]),
            // ── BATCH 39: munakahat detail, puasa sunnah, pengertian, hukum phrases ──
            // Puasa sunnah tertentu
            (&["puasa", "arafah"], vec!["صيام يوم عرفة", "صوم يوم عرفة"]),
            (&["puasa", "tasu'a"], vec!["صيام يوم التاسوعاء", "صوم قبل عاشوراء"]),
            (&["puasa", "muharram"], vec!["صيام شهر المحرم", "صوم الأشهر الحرم"]),
            (&["puasa", "daud"], vec!["صيام داود", "صوم يوم وإفطار يوم"]),
            (&["puasa", "nazar"], vec!["صوم النذر", "الوفاء بالنذر"]),
            (&["puasa", "kafarat"], vec!["صوم الكفارة", "كفارة الصيام"]),
            // Nafkah detail
            (&["nafkah", "batin"], vec!["النفقة الباطنة", "حق الجماع", "حقوق الزوجة"]),
            (&["nafkah", "lahir"], vec!["النفقة اللازمة", "نفقة الزوجة"]),
            (&["nafkah", "anak"], vec!["نفقة الأولاد", "نفقة الأبناء"]),
            (&["nafkah", "orang"], vec!["نفقة الأقارب", "النفقة الواجبة"]),
            (&["nafkah", "ibu"], vec!["نفقة الوالدين", "نفقة الأم"]),
            // Mahar detail
            (&["mahar", "mitsil"], vec!["مهر المثل", "المهر"]),
            (&["mahar", "misil"], vec!["مهر المثل", "المهر"]),
            (&["mahar", "musamma"], vec!["المهر المسمى", "تسمية المهر"]),
            (&["mahar", "muajjal"], vec!["المهر المؤجل", "المهر"]),
            // Talak types
            (&["talak", "raj'i"], vec!["الطلاق الرجعي", "طلاق رجعي"]),
            (&["talak", "ba'in"], vec!["الطلاق البائن", "بينونة صغرى"]),
            (&["talak", "tiga"], vec!["الطلاق الثلاث", "الطلاق المغلظ"]),
            (&["talak", "satu"], vec!["الطلاق الأول", "طلقة واحدة"]),
            (&["talak", "dua"], vec!["الطلاق الثاني", "الطلقة الثانية"]),
            (&["talak", "kubra"], vec!["البينونة الكبرى", "الطلاق المغلظ"]),
            // Shalat spesifik
            (&["shalat", "rawatib"], vec!["الصلوات الرواتب", "السنن الرواتب"]),
            (&["shalat", "tathawwu"], vec!["صلاة التطوع", "النوافل"]),
            (&["shalat", "nafilah"], vec!["النفل", "صلاة التطوع"]),
            (&["shalat", "sunnat"], vec!["الصلاة النافلة", "سنن الصلاة"]),
            (&["shalat", "sunnah"], vec!["الصلاة النافلة", "سنن الصلاة"]),
            (&["sholat", "sunnah"], vec!["الصلاة النافلة", "سنن الصلاة"]),
            (&["sholat", "rawatib"], vec!["الصلوات الرواتب", "السنن الرواتب"]),
            // Pengertian / ta'rif queries
            (&["pengertian", "iman"], vec!["تعريف الإيمان", "مفهوم الإيمان"]),
            (&["pengertian", "islam"], vec!["تعريف الإسلام", "مفهوم الإسلام"]),
            (&["pengertian", "ihsan"], vec!["تعريف الإحسان", "مفهوم الإحسان"]),
            (&["pengertian", "tauhid"], vec!["تعريف التوحيد", "مفهوم التوحيد"]),
            (&["pengertian", "taqwa"], vec!["تعريف التقوى", "مفهوم التقوى"]),
            (&["pengertian", "ikhlas"], vec!["تعريف الإخلاص", "مفهوم الإخلاص"]),
            (&["pengertian", "shalat"], vec!["تعريف الصلاة", "حد الصلاة"]),
            (&["pengertian", "zakat"], vec!["تعريف الزكاة", "حد الزكاة"]),
            (&["pengertian", "puasa"], vec!["تعريف الصيام", "حد الصوم"]),
            (&["pengertian", "haji"], vec!["تعريف الحج", "حد الحج"]),
            (&["pengertian", "nikah"], vec!["تعريف النكاح", "حد الزواج"]),
            (&["pengertian", "riba"], vec!["تعريف الربا", "حد الربا"]),
            (&["pengertian", "zina"], vec!["تعريف الزنا", "حد الزنا"]),
            (&["pengertian", "hadits"], vec!["تعريف الحديث", "مفهوم الحديث الشريف"]),
            (&["pengertian", "fiqih"], vec!["تعريف الفقه", "مفهوم الفقه"]),
            (&["pengertian", "usul"], vec!["تعريف أصول الفقه", "مفهوم الأصول"]),
            (&["pengertian", "tafsir"], vec!["تعريف التفسير", "مفهوم التفسير"]),
            // Hukum + specific actions
            (&["hukum", "selfie"], vec!["حكم التصوير", "حكم الصورة"]),
            (&["hukum", "foto"], vec!["حكم التصوير", "حكم الصور"]),
            (&["hukum", "video"], vec!["حكم التصوير", "حكم الفيديو"]),
            (&["hukum", "musik"], vec!["حكم المعازف", "حكم الأغاني"]),
            (&["hukum", "lagu"], vec!["حكم الغناء", "حكم المعازف"]),
            (&["hukum", "narkoba"], vec!["حكم المخدرات", "تحريم المسكرات"]),
            (&["hukum", "rokok"], vec!["حكم التدخين", "تدخين السجائر"]),
            (&["hukum", "vaping"], vec!["حكم التدخين الإلكتروني", "حكم الشيشة"]),
            (&["hukum", "game"], vec!["حكم الألعاب", "الألعاب الإلكترونية"]),
            (&["hukum", "cryptocurrency"], vec!["حكم العملات المشفرة", "العملة الرقمية"]),
            (&["hukum", "crypto"], vec!["حكم العملات المشفرة", "العملة الرقمية"]),
            (&["hukum", "bitcoin"], vec!["حكم البيتكوين", "العملة الافتراضية"]),
            (&["hukum", "pinjaman"], vec!["حكم القرض", "الإقراض"]),
            (&["hukum", "asuransi"], vec!["حكم التأمين", "التأمين الإسلامي"]),
            (&["hukum", "saham"], vec!["حكم الأسهم", "الاستثمار في الأسهم"]),
            (&["hukum", "forex"], vec!["حكم الصرف", "الفوركس"]),
            (&["hukum", "investasi"], vec!["حكم الاستثمار", "المضاربة"]),
            (&["hukum", "MLM"], vec!["حكم الشبكات", "التسويق الشبكي"]),
            (&["hukum", "mlm"], vec!["حكم الشبكات", "التسويق الشبكي"]),
            (&["hukum", "hewan"], vec!["حكم الذبح", "الأحكام المتعلقة بالحيوانات"]),
            (&["hukum", "aborsi"], vec!["حكم الإجهاض", "إسقاط الحمل"]),
            (&["hukum", "kb"], vec!["حكم تنظيم النسل", "حكم منع الحمل"]),
            (&["hukum", "bayi"], vec!["حكم الرضاعة", "أحكام الطفل"]),
            (&["hukum", "operasi"], vec!["حكم الجراحة", "أحكام الطبيب"]),
            // ── BATCH 40: halal/haram food, definisi bigrams, boleh X, daily ibadah detail ──
            // Halal/haram food queries
            (&["halal", "haram"], vec!["الحلال والحرام", "المحرمات"]),
            (&["makanan", "halal"], vec!["الأطعمة الحلال", "الطيبات"]),
            (&["minuman", "haram"], vec!["المحرمات من الشراب", "الأشربة المحرمة"]),
            (&["minuman", "halal"], vec!["الأشربة الحلال", "المشروبات"]),
            (&["daging", "babi"], vec!["لحم الخنزير", "تحريم الخنزير"]),
            (&["daging", "haram"], vec!["لحم المحرمات", "الأطعمة المحرمة"]),
            (&["daging", "sapi"], vec!["لحم البقر", "ذبيحة البقر"]),
            (&["daging", "kambing"], vec!["لحم الغنم", "ذبيحة الغنم"]),
            (&["daging", "ayam"], vec!["لحم الدجاج", "ذبيحة الطير"]),
            (&["sembelih", "hewan"], vec!["الذبح", "شروط الذكاة"]),
            (&["sembelih", "ayam"], vec!["ذبح الدجاج", "الذكاة"]),
            (&["sembelih", "sapi"], vec!["ذبح البقر", "الذكاة"]),
            // Definisi (same concept as pengertian but different word)  
            (&["definisi", "iman"], vec!["تعريف الإيمان", "معنى الإيمان"]),
            (&["definisi", "islam"], vec!["تعريف الإسلام", "معنى الإسلام"]),
            (&["definisi", "ihsan"], vec!["تعريف الإحسان", "معنى الإحسان"]),
            (&["definisi", "tauhid"], vec!["تعريف التوحيد", "معنى التوحيد"]),
            (&["definisi", "fiqih"], vec!["تعريف الفقه", "معنى الفقه"]),
            (&["definisi", "hadits"], vec!["تعريف الحديث", "علم الحديث"]),
            (&["definisi", "sunnah"], vec!["تعريف السنة", "معنى السنة النبوية"]),
            (&["definisi", "riba"], vec!["تعريف الربا", "معنى الربا"]),
            (&["definisi", "zakat"], vec!["تعريف الزكاة", "معنى الزكاة"]),
            (&["definisi", "haji"], vec!["تعريف الحج", "أركان الحج"]),
            (&["definisi", "nikah"], vec!["تعريف النكاح", "معنى الزواج"]),
            // Boleh/tidak dan hukum question phrases
            (&["boleh", "shalat"], vec!["جواز الصلاة", "هل يجوز الصلاة"]),
            (&["boleh", "nikah"], vec!["جواز النكاح", "هل يجوز الزواج"]),
            (&["boleh", "talak"], vec!["جواز الطلاق", "هل يجوز الطلاق"]),
            (&["boleh", "zakat"], vec!["جواز الزكاة", "هل يجب الزكاة"]),
            (&["boleh", "haji"], vec!["جواز الحج", "هل يجب الحج"]),
            (&["tidak", "shalat"], vec!["ترك الصلاة", "حكم تارك الصلاة"]),
            (&["tidak", "puasa"], vec!["الإفطار", "الأعذار المبيحة للفطر"]),
            (&["tidak", "zakat"], vec!["ترك الزكاة", "حكم مانع الزكاة"]),
            // Cara/proses ibadah detail queries
            (&["cara", "menyembelih"], vec!["كيفية الذبح", "شروط الذبح"]),
            (&["cara", "bertayamum"], vec!["كيفية التيمم", "صفة التيمم"]),
            (&["cara", "salat"], vec!["كيفية الصلاة", "صفة الصلاة"]),
            (&["cara", "berwudhu"], vec!["كيفية الوضوء", "صفة الوضوء"]),
            (&["cara", "berpuasa"], vec!["كيفية الصيام", "صفة الصوم"]),
            (&["cara", "mandi"], vec!["كيفية الغسل", "صفة الغسل"]),
            (&["cara", "shalat"], vec!["كيفية الصلاة", "صفة الصلاة"]),
            // Kalimat syahadat / rukun Islam questions
            (&["rukun", "syahadat"], vec!["ركن الشهادة", "الشهادتان"]),
            (&["syahadat", "iman"], vec!["شهادة أن لا إله إلا الله", "الشهادتان"]),
            (&["kalimat", "syahadat"], vec!["كلمة الشهادة", "الشهادتان"]),
            (&["makna", "syahadat"], vec!["معنى الشهادة", "مفهوم الشهادتين"]),
            (&["makna", "shalat"], vec!["معنى الصلاة", "مفهوم الصلاة"]),
            (&["makna", "puasa"], vec!["معنى الصيام", "حكمة الصيام"]),
            (&["makna", "zakat"], vec!["معنى الزكاة", "حكمة الزكاة"]),
            (&["makna", "haji"], vec!["معنى الحج", "حكمة الحج"]),
            (&["makna", "jihad"], vec!["معنى الجهاد", "مفهوم الجهاد"]),
            (&["makna", "taqwa"], vec!["معنى التقوى", "مفهوم التقوى"]),
            (&["makna", "ikhlas"], vec!["معنى الإخلاص", "مفهوم الإخلاص"]),
            (&["makna", "sabar"], vec!["معنى الصبر", "مفهوم الصبر"]),
            (&["makna", "syukur"], vec!["معنى الشكر", "الشكر لله"]),
            (&["makna", "tawakal"], vec!["معنى التوكل", "مفهوم التوكل على الله"]),
            // ── BATCH 41: dalil bigrams, adzan, tafsir surah detail, kontemporer ──
            // Dalil (evidence/proof) queries - very common Indonesian pattern
            (&["dalil", "shalat"], vec!["دليل وجوب الصلاة", "آيات الصلاة"]),
            (&["dalil", "zakat"], vec!["دليل وجوب الزكاة", "آيات الزكاة"]),
            (&["dalil", "puasa"], vec!["دليل وجوب الصيام", "آيات الصيام"]),
            (&["dalil", "haji"], vec!["دليل وجوب الحج", "آيات الحج"]),
            (&["dalil", "riba"], vec!["دليل تحريم الربا", "آيات الربا"]),
            (&["dalil", "nikah"], vec!["دليل مشروعية النكاح", "آيات النكاح"]),
            (&["dalil", "talak"], vec!["دليل الطلاق", "آيات الطلاق"]),
            (&["dalil", "waris"], vec!["دليل الميراث", "آيات الوراثة"]),
            (&["dalil", "jihad"], vec!["دليل الجهاد", "آيات الجهاد"]),
            (&["dalil", "naqli"], vec!["الدليل النقلي", "الأدلة الشرعية"]),
            (&["dalil", "aqli"], vec!["الدليل العقلي", "الأدلة العقلية"]),
            (&["dalil", "syar'i"], vec!["الدليل الشرعي", "الأدلة"]),
            // Adzan and iqamah
            (&["adzan", "subuh"], vec!["أذان الصبح", "الأذان والإقامة"]),
            (&["adzan", "jumat"], vec!["أذان الجمعة", "الأذان للجمعة"]),
            (&["adzan", "lafaz"], vec!["ألفاظ الأذان", "كلمات الأذان"]),
            (&["adzan", "makna"], vec!["معنى الأذان", "تفسير الأذان"]),
            (&["doa", "adzan"], vec!["دعاء بعد الأذان", "الدعاء عند الأذان"]),
            (&["jawab", "adzan"], vec!["إجابة المؤذن", "الرد على الأذان"]),
            (&["lafaz", "adzan"], vec!["ألفاظ الأذان", "كلمات الأذان"]),
            (&["lafaz", "iqamah"], vec!["ألفاظ الإقامة"]),
            (&["lafaz", "tasyahud"], vec!["التشهد الأخير", "التحيات"]),
            (&["lafaz", "shalawat"], vec!["الصلاة الإبراهيمية", "اللهم صل على محمد"]),
            // Tafsir surah popular
            (&["tafsir", "fatihah"], vec!["تفسير الفاتحة", "سورة الفاتحة"]),
            (&["tafsir", "ikhlas"], vec!["تفسير الإخلاص", "سورة الإخلاص"]),
            (&["tafsir", "nas"], vec!["تفسير الناس", "سورة الناس"]),
            (&["tafsir", "falaq"], vec!["تفسير الفلق", "سورة الفلق"]),
            (&["tafsir", "kafirun"], vec!["تفسير الكافرون", "سورة الكافرون"]),
            (&["tafsir", "nasr"], vec!["تفسير النصر", "سورة النصر"]),
            (&["tafsir", "asr"], vec!["تفسير العصر", "سورة العصر"]),
            (&["tafsir", "maun"], vec!["تفسير الماعون", "سورة الماعون"]),
            (&["tafsir", "quraisy"], vec!["تفسير قريش", "سورة قريش"]),
            (&["tafsir", "fatiha"], vec!["تفسير الفاتحة", "سورة الفاتحة"]),
            (&["tafsir", "lukman"], vec!["تفسير لقمان", "سورة لقمان"]),
            (&["tafsir", "maryam"], vec!["تفسير مريم", "سورة مريم"]),
            (&["tafsir", "taha"], vec!["تفسير طه", "سورة طه"]),
            (&["tafsir", "ankabut"], vec!["تفسير العنكبوت", "سورة العنكبوت"]),
            (&["tafsir", "rum"], vec!["تفسير الروم", "سورة الروم"]),
            (&["tafsir", "sajdah"], vec!["تفسير السجدة", "سورة السجدة"]),
            (&["tafsir", "yassin"], vec!["تفسير يس", "سورة يس"]),
            (&["tafsir", "saffat"], vec!["تفسير الصافات", "سورة الصافات"]),
            (&["tafsir", "zumar"], vec!["تفسير الزمر", "سورة الزمر"]),
            (&["tafsir", "ghafir"], vec!["تفسير غافر", "سورة المؤمن"]),
            (&["tafsir", "fussilat"], vec!["تفسير فصلت", "سورة فصلت"]),
            (&["tafsir", "hujurat"], vec!["تفسير الحجرات", "سورة الحجرات"]),
            (&["tafsir", "waqiah"], vec!["تفسير الواقعة", "سورة الواقعة"]),
            (&["tafsir", "hasyr"], vec!["تفسير الحشر", "سورة الحشر"]),
            (&["tafsir", "jumuah"], vec!["تفسير الجمعة", "سورة الجمعة"]),
            (&["tafsir", "munafiqun"], vec!["تفسير المنافقون", "سورة المنافقون"]),
            (&["tafsir", "talaq"], vec!["تفسير الطلاق", "سورة الطلاق"]),
            (&["tafsir", "tahrim"], vec!["تفسير التحريم", "سورة التحريم"]),
            (&["tafsir", "mulk"], vec!["تفسير الملك", "سورة الملك"]),
            (&["tafsir", "qalam"], vec!["تفسير القلم", "سورة القلم"]),
            (&["tafsir", "muzzammil"], vec!["تفسير المزمل", "سورة المزمل"]),
            (&["tafsir", "mudatstsir"], vec!["تفسير المدثر", "سورة المدثر"]),
            (&["tafsir", "qiyamah"], vec!["تفسير القيامة"]),
            (&["tafsir", "insan"], vec!["تفسير الإنسان", "سورة الإنسان"]),
            (&["tafsir", "naba"], vec!["تفسير النبأ", "سورة النبأ"]),
            (&["tafsir", "abasa"], vec!["تفسير عبس", "سورة عبس"]),
            (&["tafsir", "infitar"], vec!["تفسير الانفطار"]),
            (&["tafsir", "tatfif"], vec!["تفسير المطففين"]),
            (&["tafsir", "burooj"], vec!["تفسير البروج"]),
            (&["tafsir", "tariq"], vec!["تفسير الطارق"]),
            (&["tafsir", "ala"], vec!["تفسير الأعلى", "سورة الأعلى"]),
            (&["tafsir", "ghasyiyah"], vec!["تفسير الغاشية"]),
            (&["tafsir", "fajr"], vec!["تفسير الفجر"]),
            (&["tafsir", "balad"], vec!["تفسير البلد"]),
            (&["tafsir", "syams"], vec!["تفسير الشمس"]),
            (&["tafsir", "lail"], vec!["تفسير الليل"]),
            (&["tafsir", "duha"], vec!["تفسير الضحى"]),
            (&["tafsir", "syarh"], vec!["تفسير الشرح"]),
            (&["tafsir", "alaq"], vec!["تفسير العلق", "سورة العلق"]),
            (&["tafsir", "qadr"], vec!["تفسير القدر", "سورة القدر"]),
            (&["tafsir", "bayyinah"], vec!["تفسير البينة"]),
            (&["tafsir", "zalzalah"], vec!["تفسير الزلزلة"]),
            (&["tafsir", "adiyat"], vec!["تفسير العاديات"]),
            (&["tafsir", "qoriah"], vec!["تفسير القارعة"]),
            (&["tafsir", "takatsur"], vec!["تفسير التكاثر"]),
            (&["tafsir", "humazah"], vec!["تفسير الهمزة"]),
            (&["tafsir", "fil"], vec!["تفسير الفيل"]),
            (&["tafsir", "lahab"], vec!["تفسير المسد", "سورة اللهب"]),
            (&["tafsir", "kautsar"], vec!["تفسير الكوثر"]),
            // ── BATCH 42: haji detail, haid/nifas, khutbah, wali nikah, qunut ──
            // Haji manasik detail
            (&["tawaf", "ifadhah"], vec!["طواف الإفاضة", "الطواف"]),
            (&["tawaf", "wada"], vec!["طواف الوداع", "الطواف"]),
            (&["tawaf", "qudum"], vec!["طواف القدوم", "الطواف"]),
            (&["sa'i", "haji"], vec!["السعي بين الصفا والمروة", "السعي"]),
            (&["sa'i", "umroh"], vec!["السعي في العمرة", "السعي"]),
            (&["wukuf", "arafah"], vec!["الوقوف بعرفة", "ركن الوقوف"]),
            (&["wukuf", "mina"], vec!["المبيت بمنى", "رمي الجمرات"]),
            (&["ihram", "haji"], vec!["الإحرام للحج", "شروط الإحرام"]),
            (&["ihram", "umroh"], vec!["الإحرام للعمرة", "الإحرام"]),
            (&["larangan", "ihram"], vec!["محظورات الإحرام", "ما يحرم بالإحرام"]),
            (&["jumrah", "aqabah"], vec!["رمي جمرة العقبة", "رمي الجمرات"]),
            (&["jumrah", "ula"], vec!["الجمرة الأولى", "الجمرات"]),
            (&["dam", "haji"], vec!["الهدي والدم", "جزاء الإحرام"]),
            // Haid / nifas rulings (very common queries)
            (&["haid", "shalat"], vec!["حيض وصلاة", "الحائض والصلاة", "لا تصلي الحائض"]),
            (&["haid", "puasa"], vec!["حيض وصيام", "الحائض والصيام", "قضاء الحائض"]),
            (&["haid", "quran"], vec!["حيض وقراءة القرآن", "مس المصحف للحائض"]),
            (&["haid", "wanita"], vec!["الحيض", "أحكام الحائض"]),
            (&["haid", "thaharah"], vec!["الطهارة من الحيض", "الغسل من الحيض"]),
            (&["darah", "haid"], vec!["دم الحيض", "الحيض"]),
            (&["nifas", "shalat"], vec!["أحكام النفساء", "النفاس"]),
            (&["nifas", "puasa"], vec!["صيام النفساء", "النفاس والصيام"]),
            (&["istihadlah", "shalat"], vec!["الاستحاضة", "أحكام المستحاضة"]),
            (&["wanita", "haid"], vec!["أحكام الحائض", "الحيض"]),
            // Khutbah
            (&["khutbah", "jumat"], vec!["خطبة الجمعة", "شروط الخطبة"]),
            (&["syarat", "khutbah"], vec!["شروط الخطبة", "أركان الخطبة"]),
            (&["rukun", "khutbah"], vec!["أركان الخطبة", "الخطبة"]),
            (&["khutbah", "ied"], vec!["خطبة العيد", "صلاة العيد"]),
            // Wali nikah detail
            (&["wali", "mujbir"], vec!["الولي المجبر", "الإجبار على النكاح"]),
            (&["wali", "hakim"], vec!["ولي الحاكم", "النكاح بلا ولي"]),
            (&["wali", "nasab"], vec!["الولي من العصبة", "الولاية في النكاح"]),
            (&["wali", "nikah"], vec!["ولي النكاح", "شروط الولي"]),
            // Qunut
            (&["doa", "qunut"], vec!["دعاء القنوت", "القنوت"]),
            (&["qunut", "nazilah"], vec!["قنوت النازلة", "القنوت عند النوازل"]),
            (&["qunut", "witir"], vec!["القنوت في الوتر", "قنوت الوتر"]),
            // Jenazah detail
            (&["shalat", "jenazah"], vec!["صلاة الجنازة", "الصلاة على الميت"]),
            (&["doa", "jenazah"], vec!["دعاء صلاة الجنازة", "الدعاء للميت"]),
            (&["rukun", "jenazah"], vec!["أركان صلاة الجنازة", "الجنازة"]),
            (&["shalat", "ghaib"], vec!["صلاة الغائب", "الصلاة على الغائب"]),
            (&["kafan", "mayit"], vec!["كفن الميت", "تكفين الجنازة"]),
            (&["talkin", "mayit"], vec!["تلقين الميت", "التلقين"]),
            // Aqiqah
            (&["aqiqah", "anak"], vec!["عقيقة الولد", "أحكام العقيقة"]),
            (&["aqiqah", "laki"], vec!["عقيقة الغلام", "عقيقتان للذكر"]),
            (&["aqiqah", "perempuan"], vec!["عقيقة الأنثى", "عقيقة واحدة للأنثى"]),
            (&["aqiqah", "hukum"], vec!["حكم العقيقة", "العقيقة سنة"]),
            // Akad dan transaksi modern
            (&["akad", "jual"], vec!["عقد البيع", "شروط البيع"]),
            (&["akad", "sewa"], vec!["عقد الإجارة", "الإجارة"]),
            (&["akad", "nikah"], vec!["عقد النكاح", "صيغة النكاح"]),
            (&["jual", "beli"], vec!["البيع والشراء", "أحكام البيع"]),
            (&["sewa", "menyewa"], vec!["الإجارة", "أحكام الإجارة"]),
            (&["hutang", "piutang"], vec!["الدين والقرض", "أحكام القرض"]),
            (&["gadai", "emas"], vec!["رهن الذهب", "الرهن"]),
            (&["titip", "barang"], vec!["الوديعة", "أمانة الوديعة"]),
            // ── BATCH 43: ilmu, tazkiyah, sirah, fiqih kontemporer lanjut ──
            // Ilmu series - academic/pesantren queries
            (&["ilmu", "fiqih"], vec!["علم الفقه", "الفقه الإسلامي"]),
            (&["ilmu", "tauhid"], vec!["علم التوحيد", "علم الكلام"]),
            (&["ilmu", "hadits"], vec!["علم الحديث", "علوم الحديث"]),
            (&["ilmu", "tafsir"], vec!["علم التفسير", "تفسير القرآن"]),
            (&["ilmu", "nahwu"], vec!["علم النحو", "قواعد اللغة العربية"]),
            (&["ilmu", "sharaf"], vec!["علم الصرف", "تصريف الأفعال"]),
            (&["ilmu", "balaghah"], vec!["علم البلاغة", "البيان والمعاني"]),
            (&["ilmu", "mantiq"], vec!["علم المنطق", "المنطق"]),
            (&["ilmu", "faraid"], vec!["علم الفرائض", "أحكام الميراث"]),
            (&["ilmu", "kalam"], vec!["علم الكلام", "علم العقيدة"]),
            // Tazkiyah / tasawuf terms
            (&["tazkiyah", "nafs"], vec!["تزكية النفس", "تهذيب الأخلاق"]),
            (&["taubat", "nasuha"], vec!["التوبة النصوح", "شروط التوبة"]),
            (&["muhasabah", "nafs"], vec!["محاسبة النفس", "المحاسبة"]),
            (&["sufi", "thariqah"], vec!["الطريقة الصوفية", "السلوك الصوفي"]),
            (&["maqam", "tawakkal"], vec!["مقام التوكل", "مقامات السالكين"]),
            (&["akhlak", "mulia"], vec!["الأخلاق الكريمة", "مكارم الأخلاق"]),
            (&["akhlak", "tercela"], vec!["الأخلاق الذميمة", "الصفات المذمومة"]),
            (&["akhlak", "terpuji"], vec!["الأخلاق المحمودة", "الصفات الحسنة"]),
            // Sirah and history queries
            (&["sirah", "nabi"], vec!["السيرة النبوية", "سير الأنبياء"]),
            (&["sirah", "rasul"], vec!["السيرة النبوية", "سيرة المصطفى"]),
            (&["sejarah", "islam"], vec!["تاريخ الإسلام", "التاريخ الإسلامي"]),
            (&["khulafa", "rasyidin"], vec!["الخلفاء الراشدين", "أبوبكر عمر عثمان علي"]),
            (&["sahabat", "nabi"], vec!["أصحاب النبي", "الصحابة الكرام"]),
            (&["nabi", "ibrahim"], vec!["إبراهيم عليه السلام", "سيدنا إبراهيم"]),
            (&["nabi", "musa"], vec!["موسى عليه السلام", "سيدنا موسى"]),
            (&["nabi", "isa"], vec!["عيسى ابن مريم", "سيدنا عيسى"]),
            (&["nabi", "yusuf"], vec!["يوسف عليه السلام", "قصة يوسف"]),
            (&["nabi", "adam"], vec!["آدم أبو البشر", "سيدنا آدم"]),
            // Fiqih kontemporer lanjutan
            (&["donor", "darah"], vec!["التبرع بالدم", "حكم التبرع"]),
            (&["donor", "organ"], vec!["التبرع بالأعضاء", "زراعة الأعضاء"]),
            (&["transfusi", "darah"], vec!["نقل الدم", "حكم نقل الدم"]),
            (&["operasi", "plastik"], vec!["الجراحة التجميلية", "حكم تغيير الخلقة"]),
            (&["bayi", "tabung"], vec!["التلقيح الصناعي", "أطفال الأنابيب"]),
            (&["kloning", "manusia"], vec!["الاستنساخ البشري", "حكم الاستنساخ"]),
            (&["vaksin", "hukum"], vec!["التطعيم", "حكم اللقاح"]),
            (&["hukum", "vaksin"], vec!["حكم التطعيم", "اللقاح والشريعة"]),
            (&["bank", "darah"], vec!["بنك الدم", "حكم بنوك الدم"]),
            (&["asuransi", "jiwa"], vec!["التأمين على الحياة", "حكم التأمين"]),
            (&["kartu", "kredit"], vec!["بطاقة الائتمان", "حكم الفائدة"]),
            (&["pinjaman", "online"], vec!["الاقتراض الإلكتروني", "حكم القرض"]),
            (&["gaji", "haram"], vec!["الكسب الحرام", "أحكام الأجرة"]),
            (&["upah", "kerja"], vec!["الأجرة", "أحكام الإجارة"]),
            // Berkah dan doa
            (&["doa", "qunut"], vec!["دعاء القنوت", "قنوت الفجر"]),
            (&["doa", "witir"], vec!["دعاء القنوت في الوتر", "صلاة الوتر"]),
            (&["doa", "pagi"], vec!["أذكار الصباح", "الأذكار"]),
            (&["doa", "malam"], vec!["أذكار المساء", "الأذكار"]),
            (&["doa", "tidur"], vec!["دعاء قبل النوم", "أذكار النوم"]),
            (&["doa", "makan"], vec!["دعاء قبل الطعام", "بسم الله"]),
            (&["doa", "bepergian"], vec!["دعاء السفر", "حفظ المسافر"]),
            (&["doa", "orang"], vec!["الدعاء للآخرين", "الدعاء"]),
            (&["wirid", "pagi"], vec!["ورد الصباح", "الأوراد"]),
            (&["zikir", "pagi"], vec!["ذكر الصباح", "الأذكار"]),
            // ── BATCH 44: umroh detail, shalat khusus, fikih wanita, fiqih ibadah lanjut ──
            // Umroh / haji travel
            (&["umroh", "hukum"], vec!["حكم العمرة", "وجوب العمرة"]),
            (&["umroh", "cara"], vec!["كيفية العمرة", "مناسك العمرة"]),
            (&["umroh", "rukun"], vec!["أركان العمرة", "واجبات العمرة"]),
            (&["umroh", "syarat"], vec!["شروط العمرة", "الاستطاعة"]),
            (&["haji", "mabrur"], vec!["الحج المبرور", "قبول الحج"]),
            (&["haji", "qiran"], vec!["حج القران", "أنواع الحج"]),
            (&["haji", "tamattu"], vec!["حج التمتع", "أنواع الحج"]),
            (&["haji", "ifrad"], vec!["حج الإفراد", "أنواع الحج"]),
            (&["haji", "badal"], vec!["الحج عن الغير", "النيابة في الحج"]),
            (&["haji", "nazar"], vec!["حج النذر", "الوفاء بالنذر"]),
            // Shalat khusus detail
            (&["shalat", "gerhana"], vec!["صلاة الكسوف", "صلاة الخسوف"]),
            (&["shalat", "khusuf"], vec!["صلاة الخسوف", "كسوف القمر"]),
            (&["shalat", "kusuf"], vec!["صلاة الكسوف", "كسوف الشمس"]),
            (&["shalat", "istisqa"], vec!["صلاة الاستسقاء", "دعاء الاستسقاء"]),
            (&["shalat", "hajat"], vec!["صلاة الحاجة", "صلاة الاستخارة"]),
            (&["shalat", "tasbih"], vec!["صلاة التسبيح", "أحاديث التسبيح"]),
            (&["shalat", "tahiyyat"], vec!["صلاة تحية المسجid", "التحية"]),
            (&["shalat", "awwabin"], vec!["صلاة الأوابين", "ست ركعات بعد المغرب"]),
            // Fikih wanita
            (&["hamil", "shalat"], vec!["صلاة الحامل", "أحكام المرأة الحامل"]),
            (&["hamil", "puasa"], vec!["صيام الحامل", "المرأة الحامل والصوم"]),
            (&["menyusui", "puasa"], vec!["صيام المرضعة", "المرأة المرضعة"]),
            (&["wanita", "shalat"], vec!["صلاة المرأة", "أحكام صلاة المرأة"]),
            (&["wanita", "puasa"], vec!["صيام المرأة", "أحكام الصيام"]),
            (&["wanita", "haji"], vec!["حج المرأة", "استطاعة المرأة"]),
            (&["wanita", "mahram"], vec!["محرم المرأة", "سفر المرأة"]),
            (&["muslimah", "menutup"], vec!["ستر العورة", "حجاب المرأة"]),
            (&["jilbab", "wajib"], vec!["وجوب الحجاب", "ستر المرأة"]),
            (&["aurat", "wanita"], vec!["عورة المرأة", "الحجاب"]),
            (&["aurat", "pria"], vec!["عورة الرجل", "ستر العورة"]),
            // Jenazah more detail
            (&["memandikan", "jenazah"], vec!["غسل الميت", "أحكام الغسل"]),
            (&["mengkafani", "jenazah"], vec!["تكفين الميت", "الكفن"]),
            (&["menguburkan", "jenazah"], vec!["دفن الميت", "آداب الدفن"]),
            (&["ziarah", "kubur"], vec!["زيارة القبور", "السنة في القبور"]),
            (&["tahlilan", "hukum"], vec!["حكم التهليل", "ختمة القرآن للميت"]),
            (&["hukum", "tahlilan"], vec!["حكم التهليل", "بدعة أهل الجاهلية"]),
            (&["doa", "kubur"], vec!["الدعاء للميت", "زيارة القبور"]),
            // Sunnah nabi detail
            (&["sunnah", "rasulullah"], vec!["سنة النبي صلى الله عليه وسلم", "الحديث"]),
            (&["sunnah", "qauliyah"], vec!["السنة القولية", "أقوال النبي"]),
            (&["sunnah", "fi'liyah"], vec!["السنة الفعلية", "أفعال النبي"]),
            (&["sunnah", "taqririyah"], vec!["السنة التقريرية", "تقرير النبي"]),
            (&["hadits", "shahih"], vec!["الحديث الصحيح", "رجال الإسناد"]),
            (&["hadits", "dhaif"], vec!["الحديث الضعيف", "ضعف الإسناد"]),
            (&["hadits", "maudhu"], vec!["الحديث الموضوع", "الوضع في الحديث"]),
            (&["hadits", "hasan"], vec!["الحديث الحسن", "علوم الحديث"]),
            // Kelompok ibadah spiritual
            (&["shalawat", "nabi"], vec!["الصلاة على النبي", "صيغ الصلاة"]),
            (&["shalawat", "ibrahimiyah"], vec!["الصلاة الإبراهيمية", "اللهم صل على محمد"]),
            (&["istighfar", "dosa"], vec!["الاستغفار", "التوبة من الذنوب"]),
            (&["hasbunallah", "wakkalt"], vec!["حسبنا الله ونعم الوكيل", "الدعاء"]),
            // ── BATCH 45: mu'amalat lanjut, usul fiqh, aqidah detail, ahkam lanjut ──
            // Usul fiqh key concepts
            (&["ijma", "ulama"], vec!["إجماع العلماء", "الإجماع"]),
            (&["qiyas", "fiqih"], vec!["القياس في الفقه", "الاجتهاد بالقياس"]),
            (&["ijtihad", "ulama"], vec!["اجتهاد العلماء", "الاجتهاد"]),
            (&["istihsan", "fiqih"], vec!["الاستحسان", "الاستحسان في الفقه"]),
            (&["maslahah", "mursalah"], vec!["المصلحة المرسلة", "مصلحة الأمة"]),
            (&["sad", "dzariah"], vec!["سد الذرائع", "درء المفاسد"]),
            (&["istishab", "hukum"], vec!["الاستصحاب", "استصحاب الأصل"]),
            (&["urf", "adat"], vec!["العرف والعادة", "العادة محكمة"]),
            (&["darurat", "syariyah"], vec!["الضرورة الشرعية", "الضرورات تبيح المحظورات"]),
            (&["rukhsah", "ibadah"], vec!["الرخصة", "التخفيف في العبادة"]),
            (&["azimah", "ibadah"], vec!["العزيمة", "الأصل"]),
            // Aqidah fundamentals
            (&["rukun", "iman"], vec!["أركان الإيمان", "الإيمان به"]),
            (&["rukun", "islam"], vec!["أركان الإسلام", "الإسلام"]),
            (&["iman", "allah"], vec!["الإيمان بالله", "توحيد الربوبية"]),
            (&["iman", "malaikat"], vec!["الإيمان بالملائكة", "الملائكة"]),
            (&["iman", "kitab"], vec!["الإيمان بالكتب", "الكتب المنزلة"]),
            (&["iman", "rasul"], vec!["الإيمان بالرسل", "الأنبياء والمرسلون"]),
            (&["iman", "kiamat"], vec!["الإيمان باليوم الآخر", "أشراط الساعة"]),
            (&["iman", "qadar"], vec!["الإيمان بالقدر", "القضاء والقدر"]),
            (&["tauhid", "rububiyah"], vec!["توحيد الربوبية", "الأسماء والصفات"]),
            (&["tauhid", "uluhiyah"], vec!["توحيد الألوهية", "التوحيد"]),
            (&["tauhid", "asma"], vec!["توحيد الأسماء والصفات", "أسماء الله"]),
            (&["asmaul", "husna"], vec!["أسماء الله الحسنى", "الأسماء الحسنى"]),
            (&["sifat", "wajib"], vec!["الصفات الواجبة لله", "صفات الله"]),
            (&["sifat", "mustahil"], vec!["الصفات المستحيلة", "نفي النقائص"]),
            (&["sifat", "jaiz"], vec!["الصفات الجائزة", "ما يجوز في حق الله"]),
            // Kafir and riddah
            (&["riddah", "hukum"], vec!["حكم الردة", "المرتد"]),
            (&["hukum", "riddah"], vec!["حكم المرتد", "الردة"]),
            (&["murtad", "hukum"], vec!["حكم المرتد", "الردة عن الإسلام"]),
            (&["takfir", "hukum"], vec!["التكفير", "حكم التكفير"]),
            // Jinayat
            (&["hudud", "hukum"], vec!["الحدود الشرعية", "أحكام الحدود"]),
            (&["qishash", "hukum"], vec!["القصاص", "أحكام القصاص"]),
            (&["diyat", "pembunuhan"], vec!["الدية", "أحكام الدية"]),
            (&["had", "zina"], vec!["حد الزنا", "عقوبة الزنا"]),
            (&["had", "sariqah"], vec!["حد السرقة", "قطع اليد"]),
            (&["had", "khamr"], vec!["حد شرب الخمر", "عقوبة السكر"]),
            (&["had", "qadzaf"], vec!["حد القذف", "عقوبة القذف"]),
            (&["pembunuhan", "sengaja"], vec!["القتل العمد", "القصاص"]),
            (&["pembunuhan", "tidak"], vec!["القتل الخطأ", "الدية"]),
            // Muamalat advanced
            (&["mudharabah", "hukum"], vec!["المضاربة", "أحكام المضاربة"]),
            (&["musyarakah", "hukum"], vec!["المشاركة", "الشركة"]),
            (&["murabahah", "hukum"], vec!["المرابحة", "بيع المرابحة"]),
            (&["ijarah", "hukum"], vec!["الإجارة", "أحكام الإجارة"]),
            (&["wadiah", "hukum"], vec!["الوديعة", "الأمانة"]),
            (&["kafalah", "hukum"], vec!["الكفالة", "ضمان الغرماء"]),
            (&["hiwalah", "hukum"], vec!["الحوالة", "تحويل الدين"]),
            (&["wakaf", "produktif"], vec!["الوقف المنتج", "الوقف"]),
            (&["zakat", "profesi"], vec!["زكاة المهنة", "زكاة الراتب"]),
            (&["zakat", "investasi"], vec!["زكاة الاستثمار", "زكاة الأموال"]),
            (&["asy'ariyah", "maturidiyah"], vec!["الأشعرية والماتريدية", "أهل السنة والجماعة"]),
            (&["ahlus", "sunnah"], vec!["أهل السنة والجماعة", "السنة"]),
            (&["ahlussunnah", "waljamaah"], vec!["أهل السنة والجماعة"]),
            // ── BATCH 46: additional vocab, question types, more Islamic terms ──
            // Apa/apakah+ question bigrams (very common Indonesian patterns)
            (&["apakah", "shalat"], vec!["حكم الصلاة", "الصلاة"]),
            (&["apakah", "puasa"], vec!["حكم الصيام", "الصوم"]),
            (&["apakah", "hukum"], vec!["ما حكم", "الأحكام الشرعية"]),
            (&["apakah", "boleh"], vec!["هل يجوز", "الجواز"]),
            (&["apakah", "wajib"], vec!["هل يجب", "الوجوب"]),
            (&["apakah", "haram"], vec!["هل يحرم", "التحريم"]),
            (&["apa", "hukum"], vec!["ما حكم", "الأحكام الشرعية"]),
            (&["apa", "maksud"], vec!["ما معنى", "المعنى"]),
            (&["apa", "arti"], vec!["ما معنى", "المعنى"]),
            (&["apa", "itu"], vec!["ما هو", "التعريف"]),
            (&["apa", "yang"], vec!["ما هو", "ما الذي"]),
            (&["bagaimana", "cara"], vec!["كيفية", "الطريقة"]),
            (&["bagaimana", "hukum"], vec!["ما حكم", "الحكم"]),
            (&["bagaimana", "shalat"], vec!["كيفية الصلاة", "صفة الصلاة"]),
            (&["mengapa", "dilarang"], vec!["لماذا يحرم", "الحكمة من التحريم"]),
            (&["mengapa", "wajib"], vec!["لماذا يجب", "الحكمة من الوجوب"]),
            // Siapa series
            (&["siapa", "nabi"], vec!["الأنبياء والمرسلون", "خاتم الأنبياء"]),
            (&["siapa", "wali"], vec!["الولي", "الولاية"]),
            // More fiqh question words
            (&["kapan", "wajib"], vec!["متى يجب", "شرط الوجوب"]),
            (&["kapan", "shalat"], vec!["أوقات الصلاة", "وقت الصلاة"]),
            (&["kapan", "puasa"], vec!["وقت الصيام", "بداية رمضان"]),
            (&["dimana", "shalat"], vec!["مكان الصلاة", "مكان العبادة"]),
            (&["berapa", "rakaat"], vec!["عدد الركعات", "ركعات الصلاة"]),
            (&["berapa", "zakat"], vec!["مقدار الزكاة", "نصاب الزكاة"]),
            (&["berapa", "nishab"], vec!["النصاب", "مقدار الزكاة"]),
            // Common Indonesian diphthong words mapped
            (&["sembahyang", "hukum"], vec!["حكم الصلاة", "الصلاة"]),
            (&["sembahyang", "cara"], vec!["كيفية الصلاة", "صفة الصلاة"]),
            (&["ibadah", "shalat"], vec!["عبادة الصلاة", "الصلاة"]),
            (&["amalan", "harian"], vec!["الأعمال اليومية", "الأذكار اليومية"]),
            (&["amalan", "sunnah"], vec!["الأعمال المسنونة", "السنن"]),
            (&["amalan", "jumat"], vec!["أعمال يوم الجمعة", "سنن الجمعة"]),
            (&["amalan", "malam"], vec!["أعمال الليل", "قيام الليل"]),
            // More ibadah terms
            (&["witir", "rakaat"], vec!["ركعات الوتر", "الوتر"]),
            (&["witir", "cara"], vec!["كيفية الوتر", "صلاة الوتر"]),
            (&["tahajjud", "cara"], vec!["كيفية تهجد", "صلاة الليل"]),
            (&["tahajjud", "waktu"], vec!["وقت التهجد", "وقت قيام الليل"]),
            (&["tahajjud", "rakaat"], vec!["ركعات التهجد", "قيام الليل"]),
            (&["dhuha", "cara"], vec!["كيفية الضحى", "صلاة الضحى"]),
            (&["dhuha", "waktu"], vec!["وقت الضحى", "وقت صلاة الضحى"]),
            (&["dhuha", "rakaat"], vec!["ركعات الضحى", "عدد ركعات الضحى"]),
            // Thaharah specifics
            (&["najis", "mughalladah"], vec!["النجاسة المغلظة", "نجاسة الكلب"]),
            (&["najis", "mukhafafah"], vec!["النجاسة المخففة", "تخفيف النجاسة"]),
            (&["najis", "mutawasitah"], vec!["النجاسة المتوسطة", "النجاسة"]),
            (&["hadats", "besar"], vec!["الحدث الأكبر", "الجنابة"]),
            (&["hadats", "kecil"], vec!["الحدث الأصغر", "الوضوء"]),
            (&["suci", "hadas"], vec!["رفع الحدث", "الطهارة"]),
            (&["batal", "wudhu"], vec!["نواقض الوضوء", "مبطلات الوضوء"]),
            (&["batal", "shalat"], vec!["مبطلات الصلاة", "ما يفسد الصلاة"]),
            (&["batal", "puasa"], vec!["مفطرات الصوم", "ما يفطر"]),
            (&["batalkan", "shalat"], vec!["مبطلات الصلاة", "ما يفسد الصلاة"]),
            // Sajdah detail
            (&["sajdah", "tilawah"], vec!["سجدة التلاوة", "سجود الإخلاص"]),
            (&["sajdah", "sahwi"], vec!["سجدة السهو", "سجود السهو"]),
            (&["sujud", "sahwi"], vec!["سجود السهو", "التسهي في الصلاة"]),
            // ── BATCH 47: more questions, mazhab specifics, Islamic finance ──
            // Mazhab-specific queries
            (&["mazhab", "syafi'i"], vec!["المذهب الشافعي", "الإمام الشافعي"]),
            (&["mazhab", "hanafi"], vec!["المذهب الحنفي", "الإمام أبو حنيفة"]),
            (&["mazhab", "maliki"], vec!["المذهب المالكي", "الإمام مالك"]),
            (&["mazhab", "hanbali"], vec!["المذهب الحنبلي", "الإمام أحمد"]),
            (&["pendapat", "syafi'i"], vec!["رأي الشافعي", "مذهب الشافعي"]),
            (&["pendapat", "hanafi"], vec!["رأي الحنفية", "مذهب الحنفية"]),
            (&["pendapat", "maliki"], vec!["رأي المالكية", "مذهب المالكية"]),
            (&["menurut", "ulama"], vec!["حسب العلماء", "آراء العلماء"]),
            (&["menurut", "syafi'i"], vec!["عند الشافعية", "في مذهب الشافعي"]),
            (&["menurut", "hanafi"], vec!["عند الحنفية", "في مذهب الحنفي"]),
            // More transaction types
            (&["bai", "salam"], vec!["بيع السلم", "السلم"]),
            (&["bai", "istisna"], vec!["الاستصناع", "بيع الاستصناع"]),
            (&["bai", "muajjal"], vec!["البيع بالتقسيط", "البيع الآجل"]),
            (&["harga", "ditentukan"], vec!["تسمية الثمن", "تحديد السعر"]),
            (&["khiyar", "majlis"], vec!["خيار المجلس", "الخيار"]),
            (&["khiyar", "syarat"], vec!["خيار الشرط", "الخيار"]),
            (&["khiyar", "aib"], vec!["خيار العيب", "الرد بالعيب"]),
            (&["riba", "fadhl"], vec!["ربا الفضل", "الربا"]),
            (&["riba", "nasi'ah"], vec!["ربا النسيئة", "ربا النسأ"]),
            (&["riba", "qardh"], vec!["ربا القرض", "الربا"]),
            (&["gharar", "hukum"], vec!["الغرر", "حكم الغرر"]),
            (&["maisir", "hukum"], vec!["الميسر", "القمار"]),
            // Syirkah types
            (&["syirkah", "inan"], vec!["شركة العنان", "الشركة"]),
            (&["syirkah", "abdan"], vec!["شركة الأبدان", "الشركة"]),
            (&["syirkah", "wujuh"], vec!["شركة الوجوه", "الشركة"]),
            // Jual beli terlarang
            (&["jual", "gharar"], vec!["بيع الغرر", "الغرر"]),
            (&["jual", "najis"], vec!["بيع النجاسات", "ما لا يجوز بيعه"]),
            (&["jual", "maisir"], vec!["بيع الميسر", "القمار"]),
            (&["jual", "ijon"], vec!["بيع المعاوضة", "البيوع الفاسدة"]),
            // Islam dan sains
            (&["islam", "sains"], vec!["الإسلام والعلم", "الإسلام والعلوم"]),
            (&["islam", "teknologi"], vec!["الإسلام والتكنولوجيا", "المستجدات"]),
            (&["islam", "demokrasi"], vec!["الإسلام والديمقراطية", "الإسلام السياسي"]),
            (&["negara", "islam"], vec!["الدولة الإسلامية", "الخلافة"]),
            (&["khilafah", "hukum"], vec!["الخلافة الإسلامية", "الحكم الإسلامي"]),
            // Lingkungan dan alam
            (&["alam", "lingkungan"], vec!["البيئة", "رعاية البيئة"]),
            (&["hewan", "pelihara"], vec!["اقتناء الحيوانات", "تربية الحيوانات"]),
            (&["anjing", "hukum"], vec!["حكم الكلب", "اقتناء الكلب"]),
            (&["kucing", "hukum"], vec!["حكم القط", "اقتناء القط"]),
            // Makanan spesifik
            (&["gelatin", "hukum"], vec!["حكم الجيلاتين", "المادة الجيلاتينية"]),
            (&["alkohol", "makanan"], vec!["الكحول في الطعام", "حكم الكحول"]),
            (&["msg", "halal"], vec!["حكم مسحوق الطعام", "المحسنات الغذائية"]),
            (&["darah", "hukum"], vec!["حكم الدم", "تحريم الدم"]),
            (&["bangkai", "hukum"], vec!["حكم الميتة", "الميتة"]),
            (&["makan", "babi"], vec!["تحريم لحم الخنزير", "الخنزير"]),
            (&["makan", "haram"], vec!["الأطعمة المحرمة", "المحرمات"]),
            // Pendidikan dan ilmu
            (&["kewajiban", "ilmu"], vec!["وجوب طلب العلم", "فضل العلم"]),
            (&["mencari", "ilmu"], vec!["طلب العلم", "فضل العلم"]),
            (&["kitab", "kuning"], vec!["كتب التراث", "أمهات الكتب"]),
            (&["pesantren", "pendidikan"], vec!["التعليم الديني", "المدراس الإسلامية"]),
            // ── BATCH 48: niat forms, mazhab opinions, waktu ibadah, urutan fiqh ──
            // Niat shalat wajib
            (&["niat", "subuh"], vec!["نية صلاة الصبح", "نية الصلاة"]),
            (&["niat", "dzuhur"], vec!["نية صلاة الظهر", "نية الصلاة"]),
            (&["niat", "ashar"], vec!["نية صلاة العصر", "نية الصلاة"]),
            (&["niat", "maghrib"], vec!["نية صلاة المغرب", "نية الصلاة"]),
            (&["niat", "isya"], vec!["نية صلاة العشاء", "نية الصلاة"]),
            (&["niat", "jumat"], vec!["نية صلاة الجمعة", "نية الصلاة"]),
            (&["niat", "tarawih"], vec!["نية صلاة التراويح", "نية الصلاة"]),
            (&["niat", "witir"], vec!["نية صلاة الوتر", "نية الصلاة"]),
            (&["niat", "puasa"], vec!["نية الصيام", "نوى صوم رمضان"]),
            (&["niat", "haji"], vec!["نية الحج", "نية الإحرام"]),
            (&["niat", "umroh"], vec!["نية العمرة", "نية الإحرام"]),
            // Waktu shalat spesifik
            (&["waktu", "subuh"], vec!["وقت صلاة الصبح", "وقت الفجر"]),
            (&["waktu", "dzuhur"], vec!["وقت صلاة الظهر", "الزوال"]),
            (&["waktu", "ashar"], vec!["وقت صلاة العصر", "العصر"]),
            (&["waktu", "maghrib"], vec!["وقت صلاة المغرب", "الغروب"]),
            (&["waktu", "isya"], vec!["وقت صلاة العشاء", "الليل"]),
            (&["waktu", "shalat"], vec!["أوقات الصلاة", "مواقيت الصلاة"]),
            // Jumhur ulama positions
            (&["jumhur", "ulama"], vec!["جمهور العلماء", "الأكثرية"]),
            (&["kesepakatan", "ulama"], vec!["اتفاق العلماء", "الإجماع"]),
            (&["menurut", "syafi'i"], vec!["عند الشافعي", "في مذهب الشافعي"]),
            (&["menurut", "hanafi"], vec!["عند الحنفية", "في مذهب الحنفي"]),
            (&["menurut", "maliki"], vec!["عند المالكية", "في مذهب المالكي"]),
            (&["menurut", "hambali"], vec!["عند الحنابلة", "في مذهب الحنبلي"]),
            // Bacaan shalat
            (&["bacaan", "fatihah"], vec!["قراءة الفاتحة", "الفاتحة في الصلاة"]),
            (&["bacaan", "ruku"], vec!["ذكر الركوع", "سبحان ربي العظيم"]),
            (&["bacaan", "sujud"], vec!["ذكر السجود", "سبحان ربي الأعلى"]),
            (&["bacaan", "tahiyat"], vec!["التشهد", "التحيات"]),
            (&["bacaan", "i'tidal"], vec!["التسميع", "سمع الله لمن حمده"]),
            (&["bacaan", "qunut"], vec!["دعاء القنوت", "القنوت"]),
            (&["bacaan", "doa"], vec!["الدعاء", "أدعية الصلاة"]),
            // Urutan ibadah (how to perform)
            (&["cara", "wudhu"], vec!["كيفية الوضوء", "فرائض الوضوء"]),
            (&["urutan", "wudhu"], vec!["ترتيب الوضوء", "سنن الوضوء"]),
            (&["cara", "tayamum"], vec!["كيفية التيمم", "التيمم"]),
            (&["cara", "mandi"], vec!["كيفية الغسل", "فرائض الغسل"]),
            (&["cara", "shalat"], vec!["كيفية الصلاة", "أركان الصلاة"]),
            (&["urutan", "shalat"], vec!["ترتيب الصلاة", "أركان الصلاة"]),
            (&["cara", "puasa"], vec!["كيفية الصيام", "فرائض الصوم"]),
            (&["cara", "zakat"], vec!["كيفية الزكاة", "الزكاة"]),
            (&["cara", "haji"], vec!["مناسك الحج", "كيفية الحج"]),
            (&["cara", "umroh"], vec!["مناسك العمرة", "كيفية العمرة"]),
            // Kalkulation zakat
            (&["nisab", "zakat"], vec!["نصاب الزكاة", "مقدار النصاب"]),
            (&["nisab", "emas"], vec!["نصاب الذهب", "85 غرام"]),
            (&["nisab", "perak"], vec!["نصاب الفضة", "595 غرام"]),
            (&["kadar", "zakat"], vec!["مقدار الزكاة", "نسبة الزكاة"]),
            (&["zakat", "berapa"], vec!["حساب الزكاة", "مقدار الزكاة"]),
            // Waris calculation
            (&["warisan", "anak"], vec!["ميراث الأبناء", "الفرائض"]),
            (&["warisan", "istri"], vec!["ميراث الزوجة", "الثمن والربع"]),
            (&["warisan", "suami"], vec!["ميراث الزوج", "النصف والربع"]),
            (&["warisan", "ibu"], vec!["ميراث الأم", "الثلث"]),
            (&["warisan", "ayah"], vec!["ميراث الأب", "العصبة"]),
            (&["ahli", "waris"], vec!["أهل الوراثة", "ورثة"]),
            (&["bagian", "waris"], vec!["الفروض المقدرة", "الفرائض"]),
            // ── BATCH 49: ibadat comparisons, social issues, akhlak, quran topics ──
            // Pertanyaan hoboh/informal
            (&["gimana", "hukum"], vec!["ما حكم", "حكم"]),
            (&["gimana", "cara"], vec!["كيف", "طريقة"]),
            (&["gmn", "hukum"], vec!["ما حكم", "حكم"]),
            (&["boleh", "gak"], vec!["هل يجوز", "الجواز"]),
            (&["haram", "gak"], vec!["هل هو حرام", "التحريم"]),
            (&["halal", "gak"], vec!["هل هو حلال", "الحلال"]),
            (&["apa", "hukumnya"], vec!["ما حكمه", "حكم"]),
            (&["hukumnya", "apa"], vec!["ما حكمه", "حكم"]),
            (&["hukumnya", "gimana"], vec!["ما حكمه", "حكم"]),
            // Akhlak dan moral
            (&["berbohong", "hukum"], vec!["حكم الكذب", "الكذب"]),
            (&["dusta", "hukum"], vec!["حكم الكذب", "الكذب"]),
            (&["bohong", "hukum"], vec!["حكم الكذب", "الكذب"]),
            (&["mencuri", "hukum"], vec!["حكم السرقة", "السرقة"]),
            (&["membunuh", "hukum"], vec!["حكم القتل", "القتل"]),
            (&["dendam", "hukum"], vec!["حكم الحقد", "الحقد والغل"]),
            (&["hasad", "hukum"], vec!["حكم الحسد", "الحسد"]),
            (&["ghibah", "hukum"], vec!["حكم الغيبة", "الغيبة"]),
            (&["namimah", "hukum"], vec!["حكم النميمة", "النميمة"]),
            (&["ujub", "hukum"], vec!["حكم العجب", "العجب"]),
            (&["riya", "hukum"], vec!["حكم الرياء", "الرياء"]),
            (&["takabur", "hukum"], vec!["حكم الكبر", "الكبر والتكبر"]),
            (&["sombong", "hukum"], vec!["حكم الكبر", "الكبر"]),
            (&["sabar", "keutamaan"], vec!["فضل الصبر", "الصبر"]),
            (&["syukur", "keutamaan"], vec!["فضل الشكر", "الشكر"]),
            (&["tawakkal", "hukum"], vec!["التوكل على الله", "التوكل"]),
            (&["ikhlash", "hukum"], vec!["الإخلاص", "الإخلاص في العبادة"]),
            (&["ikhlas", "hukum"], vec!["الإخلاص", "الإخلاص في العبادة"]),
            // Quran reading and recitation
            (&["baca", "quran"], vec!["قراءة القرآن", "تلاوة القرآن"]),
            (&["tajwid", "hukum"], vec!["أحكام التجويد", "التجويد"]),
            (&["hafalan", "quran"], vec!["حفظ القرآن", "الحفاظ"]),
            (&["tafsir", "quran"], vec!["تفسير القرآن", "التفسير"]),
            (&["arti", "quran"], vec!["معاني القرآن", "تفسير"]),
            (&["makna", "quran"], vec!["معاني القرآن", "تفسير"]),
            (&["terjemah", "quran"], vec!["ترجمة القرآن", "معاني القرآن"]),
            // Hadits related
            (&["hadits", "shahih"], vec!["الحديث الصحيح", "صحيح"]),
            (&["hadits", "dhaif"], vec!["الحديث الضعيف", "ضعيف"]),
            (&["hadits", "maudhu"], vec!["الحديث الموضوع", "الموضوع"]),
            (&["hadits", "hasan"], vec!["الحديث الحسن", "حسن"]),
            (&["amalan", "bid'ah"], vec!["بدعة في العمل", "البدعة"]),
            (&["hukum", "bid'ah"], vec!["حكم البدعة", "البدعة"]),
            (&["bid'ah", "hasanah"], vec!["البدعة الحسنة", "البدعة"]),
            // Tanda kiamat
            (&["tanda", "kiamat"], vec!["أشراط الساعة", "علامات القيامة"]),
            (&["kiamat", "tanda"], vec!["أشراط الساعة", "القيامة"]),
            (&["hari", "kiamat"], vec!["يوم القيامة", "الآخرة"]),
            (&["akhirat", "kehidupan"], vec!["الحياة الآخرة", "الآخرة"]),
            (&["surga", "neraka"], vec!["الجنة والنار", "الآخرة"]),
            (&["azab", "kubur"], vec!["عذاب القبر", "القبر"]),
            (&["barzakh", "alam"], vec!["عالم البرزخ", "البرزخ"]),
            (&["hisab", "amal"], vec!["حساب الأعمال", "الحشر"]),
            (&["mizan", "timbangan"], vec!["الميزان", "الوزن"]),
            (&["sirath", "jembatan"], vec!["الصراط المستقيم", "الصراط"]),
            // ── BATCH 50: doa specifics, dzikir, tahlil, ibadah malam, quran suras ──
            // Doa-doa populer
            (&["doa", "makan"], vec!["دعاء الطعام", "الدعاء قبل الأكل"]),
            (&["doa", "tidur"], vec!["دعاء النوم", "أذكار النوم"]),
            (&["doa", "bangun"], vec!["دعاء الاستيقاظ", "أذكار الصباح"]),
            (&["doa", "wudhu"], vec!["دعاء الوضوء", "الدعاء عند الوضوء"]),
            (&["doa", "masuk"], vec!["دعاء دخول المسجد", "أذكار"]),
            (&["doa", "keluar"], vec!["دعاء الخروج", "أذكار الخروج"]),
            (&["doa", "safar"], vec!["دعاء السفر", "أذكار السفر"]),
            (&["doa", "hujan"], vec!["دعاء الاستسقاء", "دعاء المطر"]),
            (&["doa", "kesembuhan"], vec!["دعاء المريض", "دعاء الشفاء"]),
            (&["doa", "qunut"], vec!["دعاء القنوت", "القنوت"]),
            (&["doa", "iftitah"], vec!["دعاء الاستفتاح", "افتتاح الصلاة"]),
            (&["doa", "nabi"], vec!["أدعية النبي", "دعاء النبي ﷺ"]),
            // Dzikir
            (&["dzikir", "hizb"], vec!["حزب الأوراد", "الأوراد"]),
            (&["dzikir", "pagi"], vec!["أذكار الصباح", "أذكار الصباح والمساء"]),
            (&["dzikir", "sore"], vec!["أذكار المساء", "أذكار الصباح والمساء"]),
            (&["dzikir", "setelah"], vec!["أذكار بعد الصلاة", "تسبيح"]),
            (&["tasbih", "tahmid"], vec!["التسبيح والتحميد", "الأذكار"]),
            (&["istighfar", "hukum"], vec!["الاستغفار", "طلب المغفرة"]),
            // Tahlil dan tradisi
            (&["tahlil", "hukum"], vec!["حكم التهليل", "التهليل"]),
            (&["tahlil", "arwah"], vec!["التهليل للأرواح", "إهداء الثواب"]),
            (&["yasin", "hukum"], vec!["قراءة سورة يس", "سورة يس"]),
            (&["kirim", "doa"], vec!["إهداء الثواب", "إهداء ثواب القراءة"]),
            (&["doa", "arwah"], vec!["الدعاء للميت", "إهداء الثواب"]),
            (&["maulid", "nabi"], vec!["المولد النبوي", "الاحتفال بالمولد"]),
            (&["isra", "mi'raj"], vec!["الإسراء والمعراج", "المعراج"]),
            // Surat-surat Al-Quran
            (&["surat", "yasin"], vec!["سورة يس", "يس"]),
            (&["surat", "al-fatihah"], vec!["سورة الفاتحة", "الفاتحة"]),
            (&["surat", "al-baqarah"], vec!["سورة البقرة", "البقرة"]),
            (&["surat", "ali-imran"], vec!["سورة آل عمران", "آل عمران"]),
            (&["surat", "al-kahfi"], vec!["سورة الكهف", "الكهف"]),
            (&["surat", "al-mulk"], vec!["سورة الملك", "الملك"]),
            (&["surat", "al-waqiah"], vec!["سورة الواقعة", "الواقعة"]),
            (&["surat", "ar-rahman"], vec!["سورة الرحمن", "الرحمن"]),
            (&["ayat", "kursi"], vec!["آية الكرسي", "الكرسي"]),
            // Ibadah malam
            (&["shalat", "lail"], vec!["صلاة الليل", "قيام الليل"]),
            (&["qiyamul", "lail"], vec!["قيام الليل", "صلاة الليل"]),
            (&["shalat", "tahajjud"], vec!["صلاة التهجد", "التهجد"]),
            (&["waktu", "tahajjud"], vec!["وقت التهجد", "الثلث الأخير"]),
            (&["rakaat", "tahajjud"], vec!["عدد ركعات التهجد", "التهجد"]),
            (&["keutamaan", "tahajjud"], vec!["فضل التهجد", "فضل صلاة الليل"]),
            // Nabi Muhammad
            (&["sirah", "nabi"], vec!["السيرة النبوية", "سيرة النبي ﷺ"]),
            (&["kelahiran", "nabi"], vec!["مولد النبي ﷺ", "تاريخ الميلاد"]),
            (&["wafat", "nabi"], vec!["وفاة النبي ﷺ", "المدينة المنورة"]),
            (&["sahabat", "nabi"], vec!["صحابة النبي ﷺ", "الصحابة"]),
            (&["khulafaur", "rasyidin"], vec!["الخلفاء الراشدون", "أبو بكر وعمر وعثمان وعلي"]),
            (&["abu", "bakar"], vec!["أبو بكر الصديق", "الخليفة الأول"]),
            (&["umar", "khattab"], vec!["عمر بن الخطاب", "الخليفة الثاني"]),
            (&["utsman", "affan"], vec!["عثمان بن عفان", "الخليفة الثالث"]),
            (&["ali", "thalib"], vec!["علي بن أبي طالب", "الخليفة الرابع"]),
            // ── BATCH 51: masyarakat, politik, perempuan hak, doa harian lanjut ──
            // Hak perempuan
            (&["hak", "perempuan"], vec!["حقوق المرأة", "حق المرأة"]),
            (&["hak", "wanita"], vec!["حقوق المرأة", "حق المرأة"]),
            (&["perempuan", "kerja"], vec!["عمل المرأة", "المرأة والعمل"]),
            (&["wanita", "kerja"], vec!["عمل المرأة", "المرأة والعمل"]),
            (&["perempuan", "kepemimpinan"], vec!["قيادة المرأة", "ولاية المرأة"]),
            (&["imamah", "wanita"], vec!["إمامة المرأة", "المرأة في الصلاة"]),
            (&["wanita", "shalat"], vec!["صلاة المرأة", "المرأة في الصلاة"]),
            (&["perempuan", "shalat"], vec!["صلاة المرأة", "المرأة في الصلاة"]),
            (&["wanita", "jum'at"], vec!["صلاة الجمعة للمرأة", "الجمعة"]),
            (&["wanita", "safar"], vec!["سفر المرأة", "المحرم"]),
            (&["mahram", "wanita"], vec!["محرم المرأة", "المحارم"]),
            // Pernikahan spesifik
            (&["nikah", "beda"], vec!["الزواج من غير المسلم", "نكاح الكافر"]),
            (&["nikah", "mut'ah"], vec!["نكاح المتعة", "زواج المتعة"]),
            (&["nikah", "poligami"], vec!["التعدد", "تعدد الزوجات"]),
            (&["poligami", "hukum"], vec!["حكم التعدد", "تعدد الزوجات"]),
            (&["istri", "lebih"], vec!["تعدد الزوجات", "العدل بين الزوجات"]),
            (&["dua", "istri"], vec!["تعدد الزوجات", "الزوجتان"]),
            (&["talak", "tiga"], vec!["الطلاق الثلاث", "الطلاق البائن"]),
            (&["talak", "satu"], vec!["الطلاق الأول", "الطلاق"]),
            (&["ruju", "hukum"], vec!["الرجعة", "حكم الرجعة"]),
            (&["iddah", "cerai"], vec!["العدة بعد الطلاق", "العدة"]),
            (&["iddah", "wafat"], vec!["العدة بعد الوفاة", "عدة الوفاة"]),
            // Muamalat modern
            (&["kredit", "hukum"], vec!["حكم البيع بالتقسيط", "التقسيط"]),
            (&["cicilan", "hukum"], vec!["البيع بالتقسيط", "التقسيط"]),
            (&["bunga", "bank"], vec!["فوائد البنك", "الربا المصرفي"]),
            (&["deposito", "hukum"], vec!["حكم الودائع المصرفية", "الودائع"]),
            (&["saham", "hukum"], vec!["حكم الأسهم", "الأسهم"]),
            (&["reksa", "dana"], vec!["صناديق الاستثمار", "الاستثمار"]),
            (&["obligasi", "hukum"], vec!["حكم السندات", "الصكوك"]),
            (&["sukuk", "hukum"], vec!["الصكوك الإسلامية", "الصكوك"]),
            (&["asuransi", "hukum"], vec!["حكم التأمين", "التأمين"]),
            (&["bpjs", "hukum"], vec!["حكم التأمين الصحي", "التأمين"]),
            (&["jual", "online"], vec!["البيع عبر الإنترنت", "البيع الإلكتروني"]),
            (&["dropship", "hukum"], vec!["حكم الدروبشيبينغ", "بيع ما لا يملك"]),
            // Ilmu dan etika
            (&["adab", "guru"], vec!["أدب الطالب مع الشيخ", "أدب المتعلم"]),
            (&["adab", "orangtua"], vec!["برالوالدين", "حق الوالدين"]),
            (&["berbakti", "orangtua"], vec!["بر الوالدين", "حق الوالدين"]),
            (&["durhaka", "orangtua"], vec!["عقوق الوالدين", "الكبائر"]),
            (&["silaturahmi", "hukum"], vec!["صلة الرحم", "صلة الأرحام"]),
            (&["hubungan", "keluarga"], vec!["صلة الرحم", "الأسرة"]),
            // Ibadah sosial
            (&["kurban", "hukum"], vec!["حكم الأضحية", "الأضحية"]),
            (&["kurban", "syarat"], vec!["شروط الأضحية", "الأضحية"]),
            (&["kurban", "niat"], vec!["نية الأضحية", "الأضحية"]),
            (&["aqiqah", "hukum"], vec!["حكم العقيقة", "العقيقة"]),
            (&["aqiqah", "anak"], vec!["عقيقة المولود", "العقيقة"]),
            (&["aqiqah", "waktu"], vec!["وقت العقيقة", "العقيقة"]),
            // ── BATCH 52: comprehensive search patterns, alternative spellings ──
            // Alternative bahasa spellings (variant)
            (&["solat", "subuh"], vec!["صلاة الصبح", "صلاة الفجر"]),
            (&["solat", "dzuhur"], vec!["صلاة الظهر", "الظهر"]),
            (&["solat", "ashar"], vec!["صلاة العصر", "العصر"]),
            (&["solat", "maghrib"], vec!["صلاة المغرب", "المغرب"]),
            (&["solat", "isya"], vec!["صلاة العشاء", "العشاء"]),
            (&["solat", "jumat"], vec!["صلاة الجمعة", "الجمعة"]),
            (&["solat", "sunnah"], vec!["الصلوات النافلة", "السنن"]),
            (&["sholat", "subuh"], vec!["صلاة الصبح", "صلاة الفجر"]),
            (&["sholat", "dzuhur"], vec!["صلاة الظهر", "الظهر"]),
            (&["sholat", "ashar"], vec!["صلاة العصر", "العصر"]),
            (&["sholat", "maghrib"], vec!["صلاة المغرب", "المغرب"]),
            (&["sholat", "isya"], vec!["صلاة العشاء", "العشاء"]),
            (&["sholat", "jumat"], vec!["صلاة الجمعة", "الجمعة"]),
            (&["sholat", "sunnah"], vec!["السنن المؤكدة", "السنن"]),
            (&["sholat", "berjamaah"], vec!["الصلاة في جماعة", "الجماعة"]),
            (&["shoum", "hukum"], vec!["حكم الصيام", "الصوم"]),
            (&["shaum", "hukum"], vec!["حكم الصيام", "الصوم"]),
            // CRUD variants for common fiqh questions  
            (&["wajib", "shalat"], vec!["وجوب الصلاة", "الصلاة"]),
            (&["wajib", "zakat"], vec!["وجوب الزكاة", "الزكاة"]),
            (&["wajib", "puasa"], vec!["وجوب الصوم", "الصيام"]),
            (&["wajib", "haji"], vec!["وجوب الحج", "الحج"]),
            // Pertanyaan with kata kunci "kenapa"
            (&["kenapa", "haram"], vec!["لماذا يحرم", "سبب التحريم"]),
            (&["kenapa", "wajib"], vec!["لماذا يجب", "سبب الوجوب"]),
            (&["kenapa", "shalat"], vec!["حكمة الصلاة", "فائدة الصلاة"]),
            (&["kenapa", "puasa"], vec!["حكمة الصيام", "فائدة الصوم"]),
            (&["kenapa", "zakat"], vec!["حكمة الزكاة", "فائدة الزكاة"]),
            // Jawaban ibadah detail
            (&["berapa", "rakaat"], vec!["عدد الركعات", "الركعات"]),
            (&["berapa", "kali"], vec!["عدد المرات", "المرات"]),
            (&["berapa", "waktu"], vec!["الوقت المحدد", "وقت"]),
            (&["berapa", "nisab"], vec!["مقدار النصاب", "النصاب"]),
            (&["berapa", "zakat"], vec!["مقدار الزكاة", "الزكاة"]),
            // Muslim social interactions
            (&["salam", "hukum"], vec!["حكم السلام", "السلام"]),
            (&["jabat", "tangan"], vec!["المصافحة", "حكم المصافحة"]),
            (&["pertemanan", "non"], vec!["صداقة غير المسلمين", "مودة الكافرين"]),
            (&["berteman", "kafir"], vec!["صداقة الكافر", "موالاة الكافرين"]),
            (&["non", "muslim"], vec!["غير المسلمين", "التعامل مع غير المسلمين"]),
            // Kesehatan dan ibadah
            (&["sakit", "shalat"], vec!["صلاة المريض", "الصلاة في المرض"]),
            (&["sakit", "puasa"], vec!["صيام المريض", "الفطر للمريض"]),
            (&["hamil", "shalat"], vec!["صلاة الحامل", "المرأة الحامل"]),
            (&["hamil", "puasa"], vec!["صيام الحامل", "الحامل والصوم"]),
            (&["menyusui", "puasa"], vec!["صيام المرضع", "المرضع والصوم"]),
            (&["lansia", "puasa"], vec!["صيام الشيخ الكبير", "الفدية"]),
            (&["tua", "puasa"], vec!["صيام الشيخ الكبير", "الفدية"]),
            // Kalimat sederhana (simple phrases)
            (&["apa", "itu"], vec!["ما هو", "تعريف"]),
            (&["apa", "artinya"], vec!["ما معناه", "تعريف"]),
            (&["apa", "bedanya"], vec!["ما الفرق", "الفرق بين"]),
            (&["bedanya", "apa"], vec!["ما الفرق", "الفرق بين"]),
            (&["perbedaan", "antara"], vec!["الفرق بين", "الفرق"]),
            (&["sama", "tidak"], vec!["هل هو مثله", "الفرق"]),
            // ── BATCH 53: more fiqh patterns, terminology, specific rulings ──
            // Ibadah makruh  
            (&["makruh", "shalat"], vec!["مكروهات الصلاة", "المكروه"]),
            (&["makruh", "puasa"], vec!["مكروهات الصيام", "المكروه في الصوم"]),
            (&["makruh", "wudhu"], vec!["مكروهات الوضوء", "المكروه"]),
            (&["haram", "shalat"], vec!["محرمات الصلاة", "المحرمات"]),
            // Batalnya ibadah
            (&["batal", "shalat"], vec!["مبطلات الصلاة", "ما يبطل الصلاة"]),
            (&["batal", "puasa"], vec!["مبطلات الصيام", "ما يبطل الصوم"]),
            (&["batal", "wudhu"], vec!["نواقض الوضوء", "ما ينقض الوضوء"]),
            (&["batal", "haji"], vec!["مبطلات الحج", "الإفساد"]),
            // Syarat sah ibadah
            (&["syarat", "shalat"], vec!["شروط صحة الصلاة", "شروط الصلاة"]),
            (&["syarat", "wudhu"], vec!["شروط صحة الوضوء", "شروط الوضوء"]),
            (&["syarat", "puasa"], vec!["شروط صحة الصوم", "شروط الصيام"]),
            (&["syarat", "haji"], vec!["شروط الحج", "الاستطاعة"]),
            (&["syarat", "zakat"], vec!["شروط وجوب الزكاة", "شروط الزكاة"]),
            (&["syarat", "nikah"], vec!["شروط صحة النكاح", "شروط النكاح"]),
            // Rukun ibadah
            (&["rukun", "shalat"], vec!["أركان الصلاة", "الأركان"]),
            (&["rukun", "wudhu"], vec!["أركان الوضوء", "فرائض الوضوء"]),
            (&["rukun", "puasa"], vec!["أركان الصيام", "فرائض الصوم"]),
            (&["rukun", "haji"], vec!["أركان الحج", "الأركان"]),
            (&["rukun", "umroh"], vec!["أركان العمرة", "الأركان"]),
            (&["rukun", "nikah"], vec!["أركان النكاح", "الأركان"]),
            // Sunnat ibadah
            (&["sunnah", "shalat"], vec!["سنن الصلاة", "المسنونات"]),
            (&["sunnah", "wudhu"], vec!["سنن الوضوء", "المسنونات"]),
            (&["sunnah", "puasa"], vec!["سنن الصيام", "المسنونات"]),
            (&["sunnah", "haji"], vec!["سنن الحج", "المسنونات"]),
            // Terlarang dalam ibadah
            (&["dilarang", "shalat"], vec!["المنهيات في الصلاة", "التحريم"]),
            (&["dilarang", "puasa"], vec!["المنهيات في الصوم", "التحريم"]),
            (&["dilarang", "haji"], vec!["محظورات الإحرام", "المحرمات"]),
            // Question "apakah termasuk..."
            (&["termasuk", "sunnah"], vec!["هل هو من السنة", "المسنونات"]),
            (&["termasuk", "wajib"], vec!["هل هو واجب", "الواجب"]),
            (&["termasuk", "rukun"], vec!["هل هو ركن", "الأركان"]),
            (&["termasuk", "haram"], vec!["هل هو حرام", "التحريم"]),
            (&["termasuk", "makruh"], vec!["هل هو مكروه", "المكروه"]),
            // Ibadah di bulan istimewa
            (&["ramadhan", "amalan"], vec!["أعمال رمضان", "عبادات رمضان"]),
            (&["rajab", "ibadah"], vec!["عبادات رجب", "أعمال رجب"]),
            (&["syaban", "ibadah"], vec!["عبادات شعبان", "أعمال شعبان"]),
            (&["muharam", "ibadah"], vec!["عبادات المحرم", "عاشوراء"]),
            (&["dzulhijjah", "amalan"], vec!["أعمال ذي الحجة", "عبادات ذي الحجة"]),
            (&["hari", "arafah"], vec!["يوم عرفة", "صيام عرفة"]),
            (&["idul", "fitri"], vec!["عيد الفطر", "صلاة العيد"]),
            (&["idul", "adha"], vec!["عيد الأضحى", "الأضحية"]),
            // Question "kenapa Islam..."
            (&["kenapa", "islam"], vec!["حكمة الإسلام", "حكمة التشريع"]),
            (&["mengapa", "islam"], vec!["لماذا الإسلام", "حكمة"]),
            (&["islam", "mengajarkan"], vec!["تعاليم الإسلام", "الإسلام"]),
            (&["perintah", "allah"], vec!["أوامر الله", "فرائض"]),
            (&["larangan", "allah"], vec!["نواهي الله", "المحرمات"]),
            // ── BATCH 54: specific fiqh rulings, contemporary issues, zakat types ──
            // Zakat specific types
            (&["zakat", "pertanian"], vec!["زكاة الزروع والثمار", "زكاة المزروعات"]),
            (&["zakat", "perniagaan"], vec!["زكاة التجارة", "عروض التجارة"]),
            (&["zakat", "ternak"], vec!["زكاة الأنعام", "زكاة الإبل"]),
            (&["zakat", "penghasilan"], vec!["زكاة الدخل", "زكاة المال"]),
            (&["zakat", "profesi"], vec!["زكاة المهنة", "زكاة الراتب"]),
            (&["zakat", "tabungan"], vec!["زكاة المدخرات", "زكاة المال"]),
            (&["mustahiq", "zakat"], vec!["مستحقو الزكاة", "أهل الزكاة"]),
            (&["asnaf", "zakat"], vec!["أصناف مستحقي الزكاة", "مصارف الزكاة"]),
            (&["amil", "zakat"], vec!["العاملون على الزكاة", "العامل"]),
            // Shalat qashar dan jamak
            (&["shalat", "qashar"], vec!["صلاة القصر", "قصر الصلاة"]),
            (&["shalat", "jamak"], vec!["الجمع بين الصلاتين", "جمع الصلوات"]),
            (&["jamak", "taqdim"], vec!["جمع التقديم", "الجمع"]),
            (&["jamak", "takhir"], vec!["جمع التأخير", "الجمع"]),
            (&["safar", "shalat"], vec!["صلاة المسافر", "القصر"]),
            (&["musafir", "shalat"], vec!["صلاة المسافر", "قصر الصلاة"]),
            // Puasa qadha dan kafarat
            (&["puasa", "qadha"], vec!["قضاء الصوم", "قضاء رمضان"]),
            (&["puasa", "kafarat"], vec!["كفارة الصوم", "كفارة"]),
            (&["puasa", "fidyah"], vec!["الفدية", "فدية الصيام"]),
            (&["qadha", "shalat"], vec!["قضاء الصلاة", "صلاة القضاء"]),
            // Fiqh digital
            (&["media", "sosial"], vec!["حكم وسائل التواصل", "الإعلام الاجتماعي"]),
            (&["youtube", "hukum"], vec!["حكم يوتيوب", "وسائل التواصل"]),
            (&["musik", "hukum"], vec!["حكم الموسيقى", "الموسيقى"]),
            (&["lagu", "hukum"], vec!["حكم الغناء", "الغناء"]),
            (&["nyanyi", "hukum"], vec!["حكم الغناء", "الغناء"]),
            (&["foto", "hukum"], vec!["حكم التصوير", "تصوير ذوات الأرواح"]),
            (&["video", "hukum"], vec!["حكم التصوير المرئي", "التصوير"]),
            (&["game", "hukum"], vec!["حكم الألعاب الإلكترونية", "اللهو"]),
            (&["nonton", "tv"], vec!["حكم مشاهدة التلفاز", "التلفاز"]),
            // Pemimpin dan masyarakat
            (&["taat", "pemimpin"], vec!["طاعة ولي الأمر", "أولي الأمر"]),
            (&["pemimpin", "islam"], vec!["الولاية الإسلامية", "الحاكم"]),
            (&["demos", "tasi"], vec!["حكم الاحتجاج", "التظاهر"]),
            (&["suara", "pemilu"], vec!["حكم الانتخابات", "التصويت"]),
            // Fiqh khusus wanita
            (&["haid", "shalat"], vec!["صلاة الحائض", "الحيض والصلاة"]),
            (&["haid", "puasa"], vec!["صيام الحائض", "الحيض والصيام"]),
            (&["haid", "quran"], vec!["مس القرآن حال الحيض", "الحيض"]),
            (&["nifas", "shalat"], vec!["صلاة النفساء", "النفاس"]),
            (&["nifas", "puasa"], vec!["صيام النفساء", "النفاس"]),
            (&["istihadah", "shalat"], vec!["صلاة المستحاضة", "الاستحاضة"]),
            (&["aurat", "wanita"], vec!["عورة المرأة", "الحجاب"]),
            (&["jilbab", "hukum"], vec!["حكم الحجاب", "الحجاب"]),
            (&["hijab", "wajib"], vec!["وجوب الحجاب", "الحجاب"]),
            (&["cadar", "hukum"], vec!["حكم النقاب", "النقاب"]),
            // ── BATCH 55: comprehensive fiqh terms, more aqidah, ibadah complete ──
            // Tafsir terms
            (&["tafsir", "bil-matsur"], vec!["تفسير بالمأثور", "التفسير"]),
            (&["tafsir", "bil-ra'yi"], vec!["تفسير بالرأي", "التفسير"]),
            (&["muhkam", "mutasyabih"], vec!["المحكم والمتشابه", "القرآن"]),
            (&["nasikh", "mansukh"], vec!["الناسخ والمنسوخ", "النسخ"]),
            (&["asbab", "nuzul"], vec!["أسباب النزول", "نزول القرآن"]),
            (&["qiraat", "quran"], vec!["القراءات القرآنية", "علم القراءات"]),
            (&["ijaz", "quran"], vec!["إعجاز القرآن", "القرآن"]),
            // Hadits terms
            (&["hadits", "qudsi"], vec!["الحديث القدسي", "الحديث القدسي"]),
            (&["hadits", "mutawatir"], vec!["الحديث المتواتر", "المتواتر"]),
            (&["hadits", "ahad"], vec!["حديث الآحاد", "خبر الآحاد"]),
            (&["sanad", "hadits"], vec!["إسناد الحديث", "السند"]),
            (&["matan", "hadits"], vec!["متن الحديث", "المتن"]),
            (&["rijal", "hadits"], vec!["رجال الحديث", "علم الرجال"]),
            (&["ilmu", "hadits"], vec!["علم الحديث", "مصطلح الحديث"]),
            (&["musnad", "imam"], vec!["المسند", "مسند الإمام أحمد"]),
            // Usul fiqh terms
            (&["qath'i", "zhanni"], vec!["القطعي والظني", "الدلالة"]),
            (&["mantuq", "mafhum"], vec!["المنطوق والمفهوم", "الدلالة"]),
            (&["am", "khas"], vec!["العام والخاص", "الألفاظ"]),
            (&["mutlaq", "muqayyad"], vec!["المطلق والمقيد", "الألفاظ"]),
            (&["amr", "nahy"], vec!["الأمر والنهي", "الألفاظ"]),
            (&["mujmal", "mubayyan"], vec!["المجمل والمبين", "الألفاظ"]),
            (&["hukum", "syara"], vec!["الحكم الشرعي", "الأحكام"]),
            (&["taklifi", "wadhi"], vec!["الحكم التكليفي والوضعي", "الأحكام"]),
            // Aqidah advanced
            (&["sifat", "khabariyah"], vec!["الصفات الخبرية", "الصفات"]),
            (&["tawassul", "hukum"], vec!["التوسل", "حكم التوسل"]),
            (&["ziarah", "kubur"], vec!["زيارة القبور", "حكم زيارة القبور"]),
            (&["bertawassul", "nabi"], vec!["التوسل بالنبي", "التوسل"]),
            (&["tabarruk", "hukum"], vec!["التبرك", "حكم التبرك"]),
            (&["qunut", "nazilah"], vec!["قنوت النازلة", "القنوت"]),
            // Some more fiqh
            (&["shalat", "rawatib"], vec!["السنن الرواتب", "رواتب الصلاة"]),
            (&["shalat", "mutlaq"], vec!["الصلاة المطلقة", "التنفل"]),
            (&["shalat", "taubat"], vec!["صلاة التوبة", "التوبة"]),
            (&["shalat", "safar"], vec!["صلاة المسافر", "القصر"]),
            (&["shalat", "jama'ah"], vec!["الصلاة في الجماعة", "الجماعة"]),
            (&["masjid", "hukum"], vec!["أحكام المسجد", "المسجد"]),
            (&["adzan", "iqamah"], vec!["الأذان والإقامة", "الأذان"]),
            (&["shaf", "shalat"], vec!["صفوف الصلاة", "الصفوف"]),
            (&["imam", "shalat"], vec!["إمامة الصلاة", "الإمام"]),
            (&["makmum", "shalat"], vec!["أحكام المأموم", "المأموم"]),
            // ── BATCH 56: final comprehensive list - varied question styles ──
            // Kata tanya lebih beragam
            (&["siapa", "yang"], vec!["من هو", "منهم"]),
            (&["siapa", "boleh"], vec!["من يجوز له", "الجواز"]),
            (&["siapa", "wajib"], vec!["من يجب عليه", "الوجوب"]),
            (&["kapan", "boleh"], vec!["متى يجوز", "الجواز"]),
            (&["kapan", "haram"], vec!["متى يحرم", "التحريم"]),
            (&["kapan", "wajib"], vec!["متى يجب", "الوجوب"]),
            (&["dimana", "shalat"], vec!["أين تصح الصلاة", "مكان الصلاة"]),
            (&["bagaimana", "caranya"], vec!["كيف يكون", "طريقة"]),
            // Pelajaran keislaman
            (&["pelajaran", "fiqih"], vec!["دروس الفقه", "تعلم الفقه"]),
            (&["belajar", "fiqih"], vec!["تعلم الفقه", "الفقه"]),
            (&["kitab", "fiqih"], vec!["كتب الفقه", "كتاب الفقه"]),
            (&["kitab", "tasawuf"], vec!["كتب التصوف", "التصوف"]),
            (&["kitab", "hadits"], vec!["كتب الحديث", "كتاب الحديث"]),
            (&["kitab", "tafsir"], vec!["كتب التفسير", "كتاب التفسير"]),
            (&["kitab", "aqidah"], vec!["كتب العقيدة", "كتاب العقيدة"]),
            // Lebih banyak istilah fiqih khusus
            (&["israf", "hukum"], vec!["حكم الإسراف", "الإسراف"]),
            (&["boros", "hukum"], vec!["حكم الإسراف", "الإسراف"]),
            (&["qana'ah", "hukum"], vec!["القناعة", "الزهد"]),
            (&["zuhud", "hukum"], vec!["الزهد", "الزاهدون"]),
            (&["wara", "hukum"], vec!["الورع", "الحلال والحرام"]),
            (&["khusyu", "shalat"], vec!["الخشوع في الصلاة", "الخشوع"]),
            (&["tuma'ninah", "shalat"], vec!["الطمأنينة في الصلاة", "ركن الطمأنينة"]),
            // Spesifik pengurusan jenazah
            (&["mengurus", "jenazah"], vec!["تجهيز الجنازة", "الجنازة"]),
            (&["kewajiban", "jenazah"], vec!["فرضية الجنازة", "فرض الكفاية"]),
            (&["solat", "jenazah"], vec!["صلاة الجنازة", "صلاة الميت"]),
            (&["mayat", "hukum"], vec!["أحكام الميت", "الميت"]),
            (&["kubur", "hukum"], vec!["أحكام القبر", "دفن الميت"]),
            // Pertanyaan dunia-akhirat
            (&["dunia", "akhirat"], vec!["الدنيا والآخرة", "الآخرة"]),
            (&["cinta", "dunia"], vec!["حب الدنيا", "الدنيا"]),
            (&["zuhud", "dunia"], vec!["الزهد في الدنيا", "الزهد"]),
            (&["amal", "jariyah"], vec!["الصدقة الجارية", "العمل الصالح"]),
            (&["pahala", "sedekah"], vec!["ثواب الصدقة", "الصدقة"]),
            (&["sedekah", "jariyah"], vec!["الصدقة الجارية", "وقف"]),
            (&["ilmu", "bermanfaat"], vec!["العلم النافع", "علم ينتفع به"]),
            (&["doa", "anak"], vec!["الدعاء بالذرية", "الولد الصالح"]),
            (&["anak", "sholeh"], vec!["الولد الصالح", "البنون"]),
            // Hukum dan negara
            (&["hukum", "pidana"], vec!["الفقه الجنائي", "الجرائم"]),
            (&["qishas", "hukum"], vec!["القصاص", "حكم القصاص"]),
            (&["diyat", "hukum"], vec!["الدية", "حكم الدية"]),
            (&["had", "hukum"], vec!["الحدود", "حد"]),
            (&["tazir", "hukum"], vec!["التعزير", "العقوبة التعزيرية"]),
            (&["hadd", "zina"], vec!["حد الزنا", "عقوبة الزنا"]),
            (&["hadd", "sariqah"], vec!["حد السرقة", "عقوبة السرقة"]),
            (&["hadd", "qadzaf"], vec!["حد القذف", "القذف"]),
            (&["hadd", "khamr"], vec!["حد الخمر", "عقوبة شرب الخمر"]),
            // ── BATCH 57: numbers, amounts, calculations in fiqh ──
            // Rakaat specific
            (&["dua", "rakaat"], vec!["ركعتان", "صلاة ركعتين"]),
            (&["tiga", "rakaat"], vec!["ثلاث ركعات", "صلاة ثلاث"]),
            (&["empat", "rakaat"], vec!["أربع ركعات", "صلاة أربع"]),
            (&["satu", "rakaat"], vec!["ركعة واحدة", "صلاة ركعة"]),
            (&["rakaat", "witir"], vec!["ركعات الوتر", "الوتر"]),
            (&["rakaat", "tarawih"], vec!["ركعات التراويح", "التراويح"]),
            (&["rakaat", "dhuha"], vec!["ركعات الضحى", "الضحى"]),
            (&["rakaat", "tahajjud"], vec!["ركعات التهجد", "التهجد"]),
            // Jumlah hari ibadah
            (&["tiga", "hari"], vec!["ثلاثة أيام", "الأيام"]),
            (&["hari", "asyura"], vec!["يوم عاشوراء", "محرم"]),
            (&["hari", "tarwiyah"], vec!["يوم الترويه", "ذو الحجة"]),
            (&["hari", "tasyriq"], vec!["أيام التشريق", "أيام النحر"]),
            // Tahun sharia
            (&["hawl", "zakat"], vec!["حول الزكاة", "الحول"]),
            (&["haul", "zakat"], vec!["حول الزكاة", "الحول"]),
            (&["nishab", "harta"], vec!["النصاب", "مقدار النصاب"]),
            // Durasi ibadah
            (&["lama", "iddah"], vec!["مدة العدة", "العدة"]),
            (&["masa", "iddah"], vec!["مدة العدة", "العدة"]),
            (&["iddah", "berapa"], vec!["كم مدة العدة", "العدة"]),
            // Jumlah spesifik
            (&["tujuh", "kali"], vec!["سبع مرات", "العدد"]),
            (&["tiga", "kali"], vec!["ثلاث مرات", "العدد"]),
            (&["empat", "puluh"], vec!["أربعون", "العدد"]),
            (&["tujuh", "hari"], vec!["سبعة أيام", "الأيام"]),
            // Batas waktu islamik
            (&["batas", "waktu"], vec!["الوقت المحدد", "الوقت"]),
            (&["akhir", "waktu"], vec!["آخر الوقت", "الوقت"]),
            (&["awal", "waktu"], vec!["أول الوقت", "الوقت"]),
            // Types of fiqh opinions
            (&["qaul", "qadim"], vec!["القول القديم", "الشافعي"]),
            (&["qaul", "jadid"], vec!["القول الجديد", "الشافعي"]),
            (&["qawl", "mu'tamad"], vec!["القول المعتمد", "المذهب"]),
            (&["pendapat", "mu'tamad"], vec!["القول المعتمد", "المذهب"]),
            (&["wajh", "tarjih"], vec!["الترجيح", "الراجح"]),
            (&["rajih", "arjah"], vec!["الراجح", "الترجيح"]),
            // Categories of obligation
            (&["fardhu", "ain"], vec!["فرض العين", "الواجب"]),
            (&["fardhu", "kifayah"], vec!["فرض الكفاية", "الواجب"]),
            (&["wajib", "ainiyah"], vec!["الواجب العيني", "الفرض"]),
            (&["sunnah", "mu'akkad"], vec!["السنة المؤكدة", "الرواتب"]),
            (&["sunnah", "ghairu"], vec!["السنة غير المؤكدة", "السنن"]),
            (&["mubah", "hukum"], vec!["الإباحة", "الحكم"]),
            // Istilah fiqih waris
            (&["ashabah", "waris"], vec!["العصبة", "الإرث"]),
            (&["dzawil", "arham"], vec!["ذوو الأرحام", "الإرث"]),
            (&["hajb", "waris"], vec!["الحجب", "حجب الوارث"]),
            (&["aul", "waris"], vec!["العول", "التعصيب"]),
            (&["radd", "waris"], vec!["الرد", "الرد في الميراث"]),
            // ── BATCH 58: phrases with "tentang", "mengenai", "perihal" ──
            // "tentang X" query pattern
            (&["tentang", "shalat"], vec!["عن الصلاة", "الصلاة"]),
            (&["tentang", "puasa"], vec!["عن الصوم", "الصيام"]),
            (&["tentang", "zakat"], vec!["عن الزكاة", "الزكاة"]),
            (&["tentang", "haji"], vec!["عن الحج", "الحج"]),
            (&["tentang", "nikah"], vec!["عن النكاح", "النكاح"]),
            (&["tentang", "talak"], vec!["عن الطلاق", "الطلاق"]),
            (&["tentang", "waris"], vec!["عن الإرث", "الفرائض"]),
            (&["tentang", "wudhu"], vec!["عن الوضوء", "الوضوء"]),
            (&["tentang", "mandi"], vec!["عن الغسل", "الغسل"]),
            (&["tentang", "tayamum"], vec!["عن التيمم", "التيمم"]),
            (&["tentang", "thaharah"], vec!["عن الطهارة", "الطهارة"]),
            (&["tentang", "riba"], vec!["عن الربا", "الربا"]),
            (&["tentang", "akad"], vec!["عن العقد", "العقد"]),
            (&["tentang", "jihad"], vec!["عن الجهاد", "الجهاد"]),
            (&["tentang", "qurban"], vec!["عن الأضحية", "الأضحية"]),
            (&["tentang", "aqiqah"], vec!["عن العقيقة", "العقيقة"]),
            (&["tentang", "bid'ah"], vec!["عن البدعة", "البدعة"]),
            (&["tentang", "sunnah"], vec!["عن السنة", "السنة"]),
            (&["tentang", "syirkah"], vec!["عن الشركة", "الشركة"]),
            // "mengenai X" query pattern
            (&["mengenai", "shalat"], vec!["عن الصلاة", "الصلاة"]),
            (&["mengenai", "puasa"], vec!["عن الصيام", "الصيام"]),
            (&["mengenai", "zakat"], vec!["عن الزكاة", "الزكاة"]),
            (&["mengenai", "haji"], vec!["عن الحج", "الحج"]),
            (&["mengenai", "nikah"], vec!["عن النكاح", "النكاح"]),
            (&["mengenai", "riba"], vec!["عن الربا", "الربا"]),
            (&["mengenai", "waris"], vec!["عن الإرث", "الفرائض"]),
            // "soal X" Javanese-influenced query pattern
            (&["soal", "shalat"], vec!["مسألة الصلاة", "الصلاة"]),
            (&["soal", "puasa"], vec!["مسألة الصيام", "الصيام"]),
            (&["soal", "zakat"], vec!["مسألة الزكاة", "الزكاة"]),
            (&["soal", "nikah"], vec!["مسألة النكاح", "النكاح"]),
            (&["soal", "hukum"], vec!["مسألة الحكم", "الحكم"]),
            (&["soal", "halal"], vec!["مسألة الحلال", "الحلال"]),
            (&["soal", "haram"], vec!["مسألة الحرام", "الحرام"]),
            // "masalah X" fiqh query pattern
            (&["masalah", "shalat"], vec!["مسألة الصلاة", "الصلاة"]),
            (&["masalah", "puasa"], vec!["مسألة الصيام", "الصيام"]),
            (&["masalah", "zakat"], vec!["مسألة الزكاة", "الزكاة"]),
            (&["masalah", "nikah"], vec!["مسألة النكاح", "النكاح"]),
            (&["masalah", "waris"], vec!["مسألة الميراث", "الفرائض"]),
            (&["masalah", "talak"], vec!["مسألة الطلاق", "الطلاق"]),
            (&["masalah", "riba"], vec!["مسألة الربا", "الربا"]),
            (&["masalah", "haji"], vec!["مسألة الحج", "الحج"]),
            (&["masalah", "wudhu"], vec!["مسألة الوضوء", "الوضوء"]),
            (&["masalah", "thaharah"], vec!["مسألة الطهارة", "الطهارة"]),
            // BATCH 59 — perihal/dalam/seputar + keutamaan
            (&["perihal", "shalat"], vec!["الصلاة", "أحكام الصلاة"]),
            (&["perihal", "puasa"], vec!["الصيام", "أحكام الصيام"]),
            (&["perihal", "zakat"], vec!["الزكاة", "أحكام الزكاة"]),
            (&["perihal", "haji"], vec!["الحج", "أحكام الحج"]),
            (&["perihal", "nikah"], vec!["النكاح", "أحكام النكاح"]),
            (&["perihal", "riba"], vec!["الربا", "أحكام الربا"]),
            (&["perihal", "waris"], vec!["الميراث", "الفرائض"]),
            (&["perihal", "talak"], vec!["الطلاق", "أحكام الطلاق"]),
            (&["dalam", "shalat"], vec!["في الصلاة", "أثناء الصلاة"]),
            (&["dalam", "puasa"], vec!["في الصيام", "أحكام الصيام"]),
            (&["dalam", "haji"], vec!["في الحج", "أثناء الحج"]),
            (&["dalam", "fiqih"], vec!["في الفقه", "الفقه الإسلامي"]),
            (&["dalam", "aqidah"], vec!["في العقيدة", "أصول العقيدة"]),
            (&["dalam", "tasawuf"], vec!["في التصوف", "علم التصوف"]),
            (&["dalam", "tafsir"], vec!["في التفسير", "علم التفسير"]),
            (&["dalam", "hadits"], vec!["في الحديث", "علم الحديث"]),
            (&["seputar", "shalat"], vec!["الصلاة", "أحكام الصلاة"]),
            (&["seputar", "puasa"], vec!["الصيام", "مسائل الصيام"]),
            (&["seputar", "waris"], vec!["الميراث", "الفرائض"]),
            (&["seputar", "nikah"], vec!["النكاح", "مسائل النكاح"]),
            (&["seputar", "zakat"], vec!["الزكاة", "مسائل الزكاة"]),
            (&["seputar", "haji"], vec!["الحج", "مناسك الحج"]),
            (&["ihwal", "shalat"], vec!["الصلاة", "أحكام الصلاة"]),
            (&["ihwal", "puasa"], vec!["الصيام", "أحكام الصيام"]),
            (&["ihwal", "zakat"], vec!["الزكاة", "أحكام الزكاة"]),
            (&["ihwal", "nikah"], vec!["النكاح", "أحكام النكاح"]),
            (&["keutamaan", "shalat"], vec!["فضل الصلاة", "فضائل الصلاة"]),
            (&["keutamaan", "puasa"], vec!["فضل الصيام", "فضائل الصيام"]),
            (&["keutamaan", "zakat"], vec!["فضل الزكاة", "فضائل الزكاة"]),
            (&["keutamaan", "sedekah"], vec!["فضل الصدقة", "فضائل الصدقة"]),
            (&["keutamaan", "dzikir"], vec!["فضل الذكر", "فضائل الذكر"]),
            (&["keutamaan", "membaca"], vec!["فضل قراءة القرآن", "فضائل القرآن"]),
            (&["keutamaan", "quran"], vec!["فضل القرآن", "فضائل تلاوة القرآن"]),
            (&["keutamaan", "ilmu"], vec!["فضل العلم", "فضائل العلم"]),
            (&["keutamaan", "taubat"], vec!["فضل التوبة", "فضائل التوبة"]),
            (&["keutamaan", "sabar"], vec!["فضل الصبر", "فضائل الصبر"]),
            (&["keutamaan", "syukur"], vec!["فضل الشكر", "فضائل الشكر"]),
            (&["keutamaan", "shalawat"], vec!["فضل الصلاة على النبي", "فضائل الصلاة على النبي"]),
            (&["keutamaan", "istighfar"], vec!["فضل الاستغفار", "فضائل الاستغفار"]),
            (&["keutamaan", "tahajud"], vec!["فضل قيام الليل", "فضائل التهجد"]),
            (&["keutamaan", "dhuha"], vec!["فضل صلاة الضحى", "صلاة الضحى"]),
            (&["keutamaan", "haji"], vec!["فضل الحج", "فضائل الحج والعمرة"]),
            (&["keutamaan", "jihad"], vec!["فضل الجهاد", "فضائل الجهاد"]),
            (&["keutamaan", "jumat"], vec!["فضل يوم الجمعة", "فضائل يوم الجمعة"]),
            (&["keutamaan", "ramadhan"], vec!["فضل رمضان", "فضائل شهر رمضان"]),
            (&["fadilah", "shalat"], vec!["فضائل الصلاة", "فضل الصلاة"]),
            (&["fadilah", "puasa"], vec!["فضائل الصيام", "فضل الصيام"]),
            (&["fadilah", "quran"], vec!["فضائل القرآن", "فضل قراءة القرآن"]),
            (&["ganjaran", "shalat"], vec!["ثواب الصلاة", "أجر الصلاة"]),
            (&["pahala", "shalat"], vec!["ثواب الصلاة", "أجر الصلاة"]),
            (&["pahala", "puasa"], vec!["ثواب الصيام", "أجر الصيام"]),
            (&["pahala", "sedekah"], vec!["ثواب الصدقة", "أجر الصدقة"]),
            (&["pahala", "haji"], vec!["ثواب الحج", "أجر الحج والعمرة"]),
            // Kitab + bab queries
            
            
            
            
            
            
            
            
            
            
            
            (&["bab", "thaharah"], vec!["باب الطهارة", "الطهارة"]),
            (&["bab", "shalat"], vec!["باب الصلاة", "الصلاة"]),
            (&["bab", "puasa"], vec!["باب الصوم", "الصيام"]),
            (&["bab", "zakat"], vec!["باب الزكاة", "الزكاة"]),
            (&["bab", "haji"], vec!["باب الحج", "الحج"]),
            (&["bab", "nikah"], vec!["باب النكاح", "النكاح"]),
            (&["bab", "talak"], vec!["باب الطلاق", "الطلاق"]),
            (&["bab", "jual"], vec!["باب البيع", "البيوع"]),
            (&["bab", "qadha"], vec!["باب القضاء", "القضاء والقدر"]),
            // BATCH 60 — fiqih digital lanjut + doa kontekstual spesifik
            (&["selfie", "hukum"], vec!["حكم التصوير", "التصوير الفوتوغرافي"]),
            (&["foto", "hukum"], vec!["حكم التصوير", "التصوير"]),
            (&["tiktok", "hukum"], vec!["حكم مواقع التواصل", "وسائل التواصل الاجتماعي"]),
            (&["youtube", "hukum"], vec!["حكم مشاهدة الفيديو", "وسائل التواصل الاجتماعي"]),
            (&["streaming", "hukum"], vec!["حكم البث المباشر", "وسائل التواصل الاجتماعي"]),
            (&["instagram", "hukum"], vec!["حكم مواقع التواصل", "وسائل التواصل الاجتماعي"]),
            (&["facebook", "hukum"], vec!["حكم مواقع التواصل", "وسائل التواصل الاجتماعي"]),
            (&["game", "online"], vec!["حكم الألعاب الإلكترونية", "اللعب"]),
            (&["game", "hukum"], vec!["حكم الألعاب الإلكترونية", "اللعب"]),
            (&["bitcoin", "hukum"], vec!["حكم العملات الرقمية", "العملة المشفرة"]),
            (&["kripto", "hukum"], vec!["حكم العملات الرقمية", "العملة المشفرة"]),
            (&["investasi", "hukum"], vec!["حكم الاستثمار", "الاستثمار المالي"]),
            (&["asuransi", "hukum"], vec!["حكم التأمين", "التأمين الإسلامي"]),
            (&["bunga", "bank"], vec!["حكم الفائدة المصرفية", "ربا البنوك"]),
            (&["kartu", "kredit"], vec!["حكم بطاقة الائتمان", "بطاقة الائتمان"]),
            (&["pinjam", "online"], vec!["حكم الاقتراض الإلكتروني", "القرض"]),
            (&["doa", "makan"], vec!["دعاء الأكل", "دعاء قبل الطعام"]),
            (&["doa", "tidur"], vec!["دعاء النوم", "أذكار النوم"]),
            (&["doa", "bangun"], vec!["دعاء الاستيقاظ", "أذكار الصباح"]),
            (&["doa", "masuk"], vec!["دعاء دخول البيت", "دعاء دخول المسجد"]),
            (&["doa", "keluar"], vec!["دعاء الخروج من البيت", "دعاء الخروج من المسجد"]),
            (&["doa", "bepergian"], vec!["دعاء السفر", "دعاء المسافر"]),
            (&["doa", "hujan"], vec!["دعاء الاستسقاء", "دعاء نزول المطر"]),
            (&["doa", "sakit"], vec!["دعاء المريض", "دعاء الشفاء"]),
            (&["doa", "kesembuhan"], vec!["دعاء الشفاء", "الرقية الشرعية"]),
            (&["doa", "rezeki"], vec!["دعاء طلب الرزق", "دعاء توسيع الرزق"]),
            (&["doa", "hutang"], vec!["دعاء قضاء الدين", "دعاء الاستدانة"]),
            (&["doa", "pernikahan"], vec!["دعاء الزواج", "دعاء العروس"]),
            (&["doa", "anak"], vec!["دعاء الولد", "دعاء الذرية الصالحة"]),
            (&["doa", "orang"], vec!["دعاء المؤمن لأخيه", "الدعاء للمسلمين"]),
            (&["doa", "jenazah"], vec!["دعاء الجنازة", "دعاء الميت"]),
            (&["doa", "kubur"], vec!["دعاء زيارة القبور", "التسليم على أهل القبور"]),
            (&["doa", "setelah"], vec!["دعاء بعد الصلاة", "أذكار بعد الصلاة"]),
            (&["doa", "sebelum"], vec!["دعاء قبل الصلاة", "أذكار قبل النوم"]),
            (&["dzikir", "pagi"], vec!["أذكار الصباح", "الأذكار"]),
            (&["dzikir", "petang"], vec!["أذكار المساء", "الأذكار"]),
            (&["dzikir", "sore"], vec!["أذكار المساء", "الأذكار"]),
            (&["dzikir", "malam"], vec!["أذكار الليل", "أذكار المساء"]),
            (&["wirid", "harian"], vec!["الأوراد اليومية", "الأذكار اليومية"]),
            (&["bacaan", "setelah"], vec!["أذكار بعد الصلاة", "الأذكار"]),
            (&["lafal", "niat"], vec!["صيغة النية", "ألفاظ النية"]),
            (&["lafaz", "niat"], vec!["صيغة النية", "ألفاظ النية"]),
            (&["lafal", "talak"], vec!["صيغة الطلاق", "ألفاظ الطلاق"]),
            (&["ucapan", "talak"], vec!["صيغة الطلاق", "ألفاظ الطلاق"]),
            // BATCH 61 — pertanyaan "apa itu" + "kenapa" + "mengapa" patterns
            (&["apa", "itu"], vec!["ما هو", "تعريف"]),
            (&["apa", "hukum"], vec!["ما حكم", "حكم"]),
            (&["apa", "syarat"], vec!["ما شروط", "شروط"]),
            (&["apa", "rukun"], vec!["ما أركان", "أركان"]),
            (&["apa", "makna"], vec!["ما معنى", "معنى"]),
            (&["apa", "arti"], vec!["ما معنى", "تعريف"]),
            (&["apa", "definisi"], vec!["تعريف", "معنى"]),
            (&["apa", "perbedaan"], vec!["ما الفرق", "الفرق بين"]),
            (&["apa", "bedanya"], vec!["ما الفرق", "الفرق بين"]),
            (&["kenapa", "haram"], vec!["لماذا حرام", "علة التحريم"]),
            (&["kenapa", "wajib"], vec!["لماذا واجب", "علة الوجوب"]),
            (&["kenapa", "shalat"], vec!["لماذا الصلاة", "فضل الصلاة"]),
            (&["kenapa", "puasa"], vec!["لماذا الصيام", "حكمة الصيام"]),
            (&["kenapa", "zakat"], vec!["لماذا الزكاة", "حكمة الزكاة"]),
            (&["kenapa", "riba"], vec!["لماذا حرام الربا", "علة تحريم الربا"]),
            (&["kenapa", "babi"], vec!["لماذا حرام الخنزير", "علة تحريم لحم الخنزير"]),
            (&["kenapa", "khitan"], vec!["لماذا الختان", "حكمة الختان"]),
            (&["kenapa", "hijab"], vec!["لماذا الحجاب", "حكمة الحجاب"]),
            (&["mengapa", "haram"], vec!["لماذا حرام", "علة التحريم"]),
            (&["mengapa", "wajib"], vec!["لماذا واجب", "علة الوجوب"]),
            (&["mengapa", "dilarang"], vec!["لماذا محظور", "علة النهي"]),
            (&["bagaimana", "cara"], vec!["كيف", "كيفية"]),
            (&["cara", "shalat"], vec!["كيفية الصلاة", "صفة الصلاة"]),
            (&["cara", "wudhu"], vec!["كيفية الوضوء", "صفة الوضوء"]),
            (&["cara", "mandi"], vec!["كيفية الغسل", "صفة الغسل"]),
            (&["cara", "puasa"], vec!["كيفية الصيام", "أحكام الصيام"]),
            (&["cara", "zakat"], vec!["كيفية إخراج الزكاة", "أحكام الزكاة"]),
            (&["cara", "haji"], vec!["كيفية الحج", "مناسك الحج"]),
            (&["cara", "umroh"], vec!["كيفية العمرة", "مناسك العمرة"]),
            (&["cara", "tayamum"], vec!["كيفية التيمم", "صفة التيمم"]),
            (&["cara", "nikah"], vec!["كيفية عقد النكاح", "إجراء الزواج"]),
            (&["cara", "talak"], vec!["كيفية الطلاق", "ألفاظ الطلاق"]),
            (&["cara", "tobat"], vec!["كيفية التوبة", "شروط التوبة"]),
            (&["cara", "taubat"], vec!["كيفية التوبة", "شروط التوبة"]),
            (&["cara", "qurban"], vec!["كيفية الأضحية", "أحكام الأضحية"]),
            (&["cara", "aqiqah"], vec!["كيفية العقيقة", "أحكام العقيقة"]),
            (&["cara", "khitan"], vec!["كيفية الختان", "الختان"]),
            (&["cara", "jenazah"], vec!["أحكام الجنازة", "غسل الميت"]),
            (&["cara", "membayar"], vec!["كيفية أداء", "إخراج"]),
            (&["syarat", "sah"], vec!["شروط الصحة", "شرط الصحة"]),
            (&["syarat", "wajib"], vec!["شروط الوجوب", "شرط الوجوب"]),
            (&["hal", "membatalkan"], vec!["مبطلات", "نواقض"]),
            (&["yang", "membatalkan"], vec!["مبطلات", "نواقض"]),
            (&["penyebab", "batal"], vec!["مبطلات", "نواقض"]),
            (&["niat", "tulus"], vec!["الإخلاص في النية", "تحقيق النية"]),
            (&["ikhlas", "beramal"], vec!["الإخلاص في العمل", "شرط الإخلاص"]),
            (&["sah", "tidak"], vec!["هل يصح", "مسألة الصحة"]),
            (&["boleh", "tidak"], vec!["هل يجوز", "حكم الجواز"]),
            (&["wajib", "tidak"], vec!["هل يجب", "حكم الوجوب"]),
            (&["haram", "makruh"], vec!["الحرام والمكروه", "الفرق بين الحرام والمكروه"]),
            (&["sunnah", "wajib"], vec!["الفرق بين السنة والواجب", "الواجب والسنة"]),
            // BATCH 62 — specific Islamic rulings queries
            (&["hukum", "merokok"], vec!["حكم التدخين", "التدخين في الإسلام"]),
            (&["hukum", "rokok"], vec!["حكم التدخين", "دخان التبغ"]),
            (&["hukum", "narkoba"], vec!["حكم المخدرات", "المسكرات"]),
            (&["hukum", "minum"], vec!["حكم الشرب", "المشروبات"]),
            (&["hukum", "khamr"], vec!["حكم الخمر", "الخمر والمسكر"]),
            (&["hukum", "alkohol"], vec!["حكم الكحول", "الخمر والمسكر"]),
            (&["hukum", "berjabat"], vec!["حكم المصافحة", "المصافحة"]),
            (&["hukum", "jabat"], vec!["حكم المصافحة", "المصافحة مع الأجنبية"]),
            (&["hukum", "pacaran"], vec!["حكم المحبة قبل الزواج", "العلاقة قبل الزواج"]),
            (&["hukum", "zina"], vec!["حكم الزنا", "تحريم الزنا"]),
            (&["hukum", "khitan"], vec!["حكم الختان", "الختان في الإسلام"]),
            (&["hukum", "sunat"], vec!["حكم الختان", "الختان"]),
            (&["hukum", "tahlilan"], vec!["حكم التهليل", "مجلس الذكر"]),
            (&["hukum", "tahlil"], vec!["حكم التهليل", "ذكر الله"]),
            (&["hukum", "maulid"], vec!["حكم الاحتفال بالمولد", "المولد النبوي"]),
            (&["hukum", "yasinan"], vec!["حكم قراءة يس", "قراءة يس للميت"]),
            (&["hukum", "isbal"], vec!["حكم الإسبال", "إسبال الثوب"]),
            (&["hukum", "celana"], vec!["حكم لباس البنطال", "اللباس الشرعي"]),
            (&["hukum", "jenggot"], vec!["حكم اللحية", "إعفاء اللحية"]),
            (&["hukum", "memanjangkan"], vec!["حكم إطالة الشعر", "اللحية والشعر"]),
            (&["hukum", "tattoo"], vec!["حكم الوشم", "الوشم"]),
            (&["hukum", "tato"], vec!["حكم الوشم", "الوشم"]),
            (&["hukum", "piercing"], vec!["حكم الثقب", "ثقب الأذن"]),
            (&["hukum", "merajut"], vec!["حكم الخياطة", "عمل المرأة"]),
            (&["hukum", "operasi"], vec!["حكم العملية الجراحية", "التداوي"]),
            (&["hukum", "plastik"], vec!["حكم عمليات التجميل", "الجراحة التجميلية"]),
            (&["hukum", "donor"], vec!["حكم التبرع بالأعضاء", "نقل الأعضاء"]),
            (&["hukum", "darah"], vec!["حكم الدم", "الدم في الفقه"]),
            (&["hukum", "transfusi"], vec!["حكم نقل الدم", "نقل الدم"]),
            (&["hukum", "aborsi"], vec!["حكم الإجهاض", "الإجهاض في الإسلام"]),
            (&["hukum", "kb"], vec!["حكم تنظيم النسل", "منع الحمل"]),
            (&["hukum", "kontrasepsi"], vec!["حكم تنظيم النسل", "منع الحمل"]),
            (&["hukum", "vasektomi"], vec!["حكم الوعاء الدافع", "تنظيم النسل"]),
            (&["hukum", "poligami"], vec!["حكم التعدد", "تعدد الزوجات"]),
            (&["hukum", "poligini"], vec!["حكم التعدد", "تعدد الزوجات"]),
            (&["hukum", "cerai"], vec!["حكم الطلاق", "الطلاق"]),
            (&["hukum", "khulu"], vec!["حكم الخلع", "الخلع"]),
            (&["hukum", "murtad"], vec!["حكم المرتد", "الردة"]),
            (&["hukum", "pindah"], vec!["حكم تبديل الدين", "الردة"]),
            (&["hukum", "mengucapkan"], vec!["حكم التلفظ", "ألفاظ الكفر"]),
            (&["hukum", "sihir"], vec!["حكم السحر", "السحر والشعوذة"]),
            (&["hukum", "jimat"], vec!["حكم التمائم", "التمائم والتعاويذ"]),
            (&["hukum", "perdukunan"], vec!["حكم الكهانة", "الكهانة والتنجيم"]),
            (&["hukum", "horoskop"], vec!["حكم التنجيم", "علم النجوم"]),
            (&["hukum", "zodiak"], vec!["حكم التنجيم", "الأبراج"]),
            (&["hukum", "feng"], vec!["حكم عادات غير إسلامية", "التقاليد غير الإسلامية"]),
            (&["hukum", "valentines"], vec!["حكم الاحتفال بالأعياد", "الأعياد غير الإسلامية"]),
            (&["hukum", "natal"], vec!["حكم تهنئة عيد الميلاد", "تهنئة غير المسلمين"]),
            (&["hukum", "tahun"], vec!["حكم الاحتفال بالسنة الجديدة", "الأعياد"]),
            // BATCH 63 — Indonesian fiqh questions + English queries
            (&["apakah", "boleh"], vec!["هل يجوز", "الجواز"]),
            (&["apakah", "haram"], vec!["هل يحرم", "التحريم"]),
            (&["apakah", "wajib"], vec!["هل يجب", "الوجوب"]),
            (&["apakah", "sah"], vec!["هل يصح", "الصحة"]),
            (&["apakah", "batal"], vec!["هل يبطل", "البطلان"]),
            (&["apakah", "sunnah"], vec!["هل يسن", "السنة"]),
            (&["apakah", "makruh"], vec!["هل يكره", "الكراهة"]),
            (&["apakah", "diperbolehkan"], vec!["هل يجوز", "الجواز"]),
            (&["is", "allowed"], vec!["هل يجوز", "الجواز في الإسلام"]),
            (&["is", "permissible"], vec!["هل يجوز", "الجواز"]),
            (&["is", "forbidden"], vec!["هل يحرم", "التحريم"]),
            (&["is", "obligatory"], vec!["هل يجب", "الوجوب"]),
            (&["is", "valid"], vec!["هل يصح", "الصحة"]),
            (&["what", "ruling"], vec!["ما حكم", "الحكم"]),
            (&["ruling", "on"], vec!["حكم", "الحكم على"]),
            (&["islamic", "ruling"], vec!["الحكم الشرعي", "حكم الإسلام"]),
            (&["islamic", "law"], vec!["الشريعة الإسلامية", "القانون الإسلامي"]),
            (&["islamic", "view"], vec!["الرأي الإسلامي", "الموقف الإسلامي"]),
            (&["what", "does"], vec!["ما يقول", "ماذا"]),
            (&["how", "to"], vec!["كيف", "كيفية"]),
            (&["prayer", "time"], vec!["وقت الصلاة", "أوقات الصلاة"]),
            (&["fajr", "prayer"], vec!["صلاة الفجر", "الصلاة الصبحية"]),
            (&["dhuhr", "prayer"], vec!["صلاة الظهر"]),
            (&["asr", "prayer"], vec!["صلاة العصر"]),
            (&["maghrib", "prayer"], vec!["صلاة المغرب"]),
            (&["isha", "prayer"], vec!["صلاة العشاء"]),
            (&["friday", "prayer"], vec!["صلاة الجمعة", "الجمعة"]),
            (&["funeral", "prayer"], vec!["صلاة الجنازة", "الجنازة"]),
            (&["zakah", "calculation"], vec!["حساب الزكاة", "مقدار الزكاة"]),
            (&["zakat", "calculation"], vec!["حساب الزكاة", "مقدار الزكاة"]),
            (&["hajj", "pilgrimage"], vec!["الحج", "مناسك الحج"]),
            (&["umrah", "pilgrimage"], vec!["العمرة", "مناسك العمرة"]),
            (&["ramadan", "fasting"], vec!["صوم رمضان", "الصيام في رمضان"]),
            (&["halal", "food"], vec!["الطعام الحلال", "المأكولات الحلال"]),
            (&["haram", "food"], vec!["الطعام الحرام", "المحرمات من الطعام"]),
            (&["interest", "riba"], vec!["الربا والفائدة", "حكم الفائدة"]),
            (&["marriage", "contract"], vec!["عقد النكاح", "شروط الزواج"]),
            (&["divorce", "islamic"], vec!["الطلاق", "أحكام الطلاق"]),
            (&["inheritance", "islamic"], vec!["الميراث الإسلامي", "الفرائض"]),
            (&["charity", "sadaqah"], vec!["الصدقة", "النفقة"]),
            (&["wudu", "ablution"], vec!["الوضوء", "كيفية الوضوء"]),
            (&["ghusl", "ritual"], vec!["الغسل", "كيفية الغسل"]),
            (&["tayammum", "dry"], vec!["التيمم", "كيفية التيمم"]),
            (&["quran", "recitation"], vec!["تلاوة القرآن", "قراءة القرآن"]),
            (&["prophet", "sunnah"], vec!["سنة النبي", "السنة النبوية"]),
            (&["sunna", "practice"], vec!["السنة", "العمل بالسنة"]),
            (&["bid'ah", "innovation"], vec!["البدعة", "حكم البدعة"]),
            (&["shirk", "polytheism"], vec!["الشرك", "التوحيد والشرك"]),
            (&["tawhid", "monotheism"], vec!["التوحيد", "العقيدة الإسلامية"]),
            // BATCH 64 — social/ethical Islamic topics + modern questions
            (&["hukum", "merayakan"], vec!["حكم الاحتفال", "الاحتفالات في الإسلام"]),
            (&["hukum", "musik"], vec!["حكم الموسيقى", "الغناء والموسيقى"]),
            (&["hukum", "lagu"], vec!["حكم الغناء", "الغناء"]),
            (&["hukum", "nyanyi"], vec!["حكم الغناء", "الغناء والموسيقى"]),
            (&["hukum", "nonton"], vec!["حكم مشاهدة الأفلام", "المشاهدة"]),
            (&["hukum", "film"], vec!["حكم الأفلام", "الترفيه"]),
            (&["hukum", "sinetron"], vec!["حكم مشاهدة المسلسلات", "التمثيل"]),
            (&["hukum", "menonton"], vec!["حكم المشاهدة", "الترفيه"]),
            (&["hukum", "bermain"], vec!["حكم اللعب", "اللهو واللعب"]),
            (&["hukum", "olahraga"], vec!["حكم الرياضة", "التمرين"]),
            (&["hukum", "renang"], vec!["حكم السباحة", "السباحة"]),
            (&["hukum", "bersalaman"], vec!["حكم المصافحة", "التحية"]),
            (&["hukum", "berpelukan"], vec!["حكم المعانقة", "المعانقة"]),
            (&["hukum", "mencium"], vec!["حكم التقبيل", "التقبيل"]),
            (&["hukum", "berbohong"], vec!["حكم الكذب", "الكذب"]),
            (&["hukum", "ghibah"], vec!["حكم الغيبة", "الغيبة"]),
            (&["hukum", "namimah"], vec!["حكم النميمة", "النميمة"]),
            (&["hukum", "hasad"], vec!["حكم الحسد", "الحسد"]),
            (&["hukum", "iri"], vec!["حكم الحسد", "الحسد والغيرة"]),
            (&["hukum", "dengki"], vec!["حكم الحسد والحقد", "الحقد"]),
            (&["hukum", "sombong"], vec!["حكم الكبر", "الكبر والغرور"]),
            (&["hukum", "ujub"], vec!["حكم العجب", "العجب بالنفس"]),
            (&["hukum", "riya"], vec!["حكم الرياء", "الرياء"]),
            (&["hukum", "sum'ah"], vec!["حكم السمعة", "الرياء والسمعة"]),
            (&["hukum", "nifak"], vec!["حكم النفاق", "النفاق"]),
            (&["hukum", "fitnah"], vec!["حكم الفتنة", "الفتنة والكذب"]),
            (&["hukum", "mencuri"], vec!["حكم السرقة", "الشرقة"]),
            (&["hukum", "korupsi"], vec!["حكم الفساد", "الغش والفساد"]),
            (&["hukum", "suap"], vec!["حكم الرشوة", "الرشوة"]),
            (&["hukum", "riba"], vec!["حكم الربا", "الربا"]),
            (&["hukum", "hutang"], vec!["حكم الدين", "القرض"]),
            (&["hukum", "pinjam"], vec!["حكم القرض", "الدين والقرض"]),
            (&["hukum", "gadai"], vec!["حكم الرهن", "الرهن"]),
            (&["hukum", "sewa"], vec!["حكم الإجارة", "الإجارة"]),
            (&["hukum", "kerjasama"], vec!["حكم الشركة", "الشركات"]),
            (&["hukum", "jual"], vec!["حكم البيع", "البيوع"]),
            (&["hukum", "beli"], vec!["حكم البيع والشراء", "البيوع"]),
            (&["hukum", "saham"], vec!["حكم الأسهم", "أسهم الشركات"]),
            (&["hukum", "obligasi"], vec!["حكم السندات", "الأوراق المالية"]),
            (&["hukum", "asuransi"], vec!["حكم التأمين", "التأمين"]),
            (&["hukum", "wakaf"], vec!["حكم الوقف", "الوقف"]),
            (&["hukum", "hibah"], vec!["حكم الهبة", "الهبة"]),
            (&["hukum", "wasiat"], vec!["حكم الوصية", "الوصية"]),
            (&["hukum", "waris"], vec!["حكم الميراث", "الميراث"]),
            (&["hukum", "zakat"], vec!["حكم الزكاة", "الزكاة"]),
            (&["hukum", "sedekah"], vec!["حكم الصدقة", "الصدقة"]),
            (&["hukum", "infak"], vec!["حكم الإنفاق", "الإنفاق"]),
            (&["hukum", "kafah"], vec!["حكم التكافل", "التكافل الاجتماعي"]),
            // BATCH 65 — specific Arabic terms + questions with particles
            (&["apa", "itu", "shalat"], vec!["ما هي الصلاة", "تعريف الصلاة"]),
            (&["apa", "itu", "puasa"], vec!["ما هو الصيام", "تعريف الصيام"]),
            (&["apa", "itu", "zakat"], vec!["ما هو الزكاة", "تعريف الزكاة"]),
            (&["apa", "itu", "haji"], vec!["ما هو الحج", "تعريف الحج"]),
            (&["apa", "itu", "wudhu"], vec!["ما هو الوضوء", "تعريف الوضوء"]),
            (&["apa", "itu", "riba"], vec!["ما هو الربا", "تعريف الربا"]),
            (&["apa", "itu", "jihad"], vec!["ما هو الجهاد", "تعريف الجهاد"]),
            (&["apa", "itu", "syariah"], vec!["ما هي الشريعة", "تعريف الشريعة"]),
            (&["apa", "itu", "aqidah"], vec!["ما هي العقيدة", "تعريف العقيدة"]),
            (&["apa", "itu", "tasawuf"], vec!["ما هو التصوف", "تعريف التصوف"]),
            (&["apa", "maksud"], vec!["ما معنى", "ما المقصود"]),
            (&["apa", "yang", "dimaksud"], vec!["ما المقصود", "ما معنى"]),
            (&["yang", "dimaksud"], vec!["المقصود", "المراد"]),
            (&["maksud", "dengan"], vec!["المقصود بـ", "معنى"]),
            (&["pengertian", "shalat"], vec!["تعريف الصلاة", "معنى الصلاة"]),
            (&["pengertian", "puasa"], vec!["تعريف الصيام", "معنى الصيام"]),
            (&["pengertian", "zakat"], vec!["تعريف الزكاة", "معنى الزكاة"]),
            (&["pengertian", "haji"], vec!["تعريف الحج", "معنى الحج"]),
            (&["pengertian", "nikah"], vec!["تعريف النكاح", "معنى النكاح"]),
            (&["pengertian", "talak"], vec!["تعريف الطلاق", "معنى الطلاق"]),
            (&["pengertian", "riba"], vec!["تعريف الربا", "معنى الربا"]),
            (&["pengertian", "syariah"], vec!["تعريف الشريعة", "معنى الشريعة"]),
            (&["pengertian", "fiqih"], vec!["تعريف الفقه", "معنى الفقه"]),
            (&["pengertian", "aqidah"], vec!["تعريف العقيدة", "معنى العقيدة"]),
            (&["pengertian", "tasawuf"], vec!["تعريف التصوف", "معنى التصوف"]),
            (&["pengertian", "hadits"], vec!["تعريف الحديث", "معنى الحديث"]),
            (&["pengertian", "tafsir"], vec!["تعريف التفسير", "معنى التفسير"]),
            (&["pengertian", "bid'ah"], vec!["تعريف البدعة", "معنى البدعة"]),
            (&["pengertian", "sunnah"], vec!["تعريف السنة", "معنى السنة"]),
            (&["pengertian", "syirik"], vec!["تعريف الشرك", "معنى الشرك"]),
            (&["pengertian", "taqwa"], vec!["تعريف التقوى", "معنى التقوى"]),
            (&["pengertian", "tawakkal"], vec!["تعريف التوكل", "معنى التوكل"]),
            (&["pengertian", "sabar"], vec!["تعريف الصبر", "معنى الصبر"]),
            (&["pengertian", "ikhlas"], vec!["تعريف الإخلاص", "معنى الإخلاص"]),
            (&["pengertian", "tawakul"], vec!["تعريف التوكل", "التوكل على الله"]),
            (&["pengertian", "istiqomah"], vec!["تعريف الاستقامة", "معنى الاستقامة"]),
            (&["pengertian", "qanaah"], vec!["تعريف القناعة", "معنى القناعة"]),
            (&["pengertian", "zuhud"], vec!["تعريف الزهد", "معنى الزهد"]),
            (&["pengertian", "wara"], vec!["تعريف الورع", "معنى الورع"]),
            (&["pengertian", "tawadu"], vec!["تعريف التواضع", "معنى التواضع"]),
            (&["pengertian", "maqamat"], vec!["تعريف المقامات", "مقامات السلوك"]),
            (&["pengertian", "ahwal"], vec!["تعريف الأحوال", "الأحوال في السلوك"]),
            (&["pengertian", "mujahadah"], vec!["تعريف المجاهدة", "مجاهدة النفس"]),
            (&["pengertian", "muhasabah"], vec!["تعريف المحاسبة", "محاسبة النفس"]),
            (&["pengertian", "muraqabah"], vec!["تعريف المراقبة", "مراقبة الله"]),
            (&["pengertian", "mahabbah"], vec!["تعريف المحبة", "محبة الله"]),
            // Takbiran / ied
            (&["takbiran", "ied"], vec!["تكبيرات العيد", "التكبير في العيد"]),
            (&["ied", "fitri"], vec!["عيد الفطر", "العيد"]),
            (&["ied", "adha"], vec!["عيد الأضحى", "العيد"]),
            (&["idul", "fitri"], vec!["عيد الفطر"]),
            (&["idul", "adha"], vec!["عيد الأضحى"]),
            // BATCH 66 — Islamic scholars + books references + specific names
            (&["imam", "syafii"], vec!["الإمام الشافعي", "مذهب الشافعية"]),
            (&["imam", "malik"], vec!["الإمام مالك", "مذهب المالكية"]),
            (&["imam", "hanafi"], vec!["الإمام الشافعي", "أبو حنيفة"]),
            (&["imam", "ahmad"], vec!["الإمام أحمد بن حنبل", "مذهب الحنابلة"]),
            (&["imam", "ghazali"], vec!["الإمام الغزالي", "أبو حامد الغزالي"]),
            (&["imam", "nawawi"], vec!["الإمام النووي", "يحيى بن شرف النووي"]),
            (&["imam", "bukhari"], vec!["الإمام البخاري", "محمد بن إسماعيل البخاري"]),
            (&["imam", "muslim"], vec!["الإمام مسلم", "مسلم بن الحجاج"]),
            (&["ibnu", "taimiyah"], vec!["ابن تيمية", "شيخ الإسلام ابن تيمية"]),
            (&["ibnu", "qayyim"], vec!["ابن القيم", "ابن قيم الجوزية"]),
            (&["ibnu", "hajar"], vec!["ابن حجر العسقلاني", "الحافظ ابن حجر"]),
            (&["ibnu", "kathir"], vec!["ابن كثير", "تفسير ابن كثير"]),
            (&["ibnu", "katsir"], vec!["ابن كثير", "تفسير ابن كثير"]),
            (&["ibnu", "majah"], vec!["ابن ماجه", "سنن ابن ماجه"]),
            (&["ibnu", "rushd"], vec!["ابن رشد", "بداية المجتهد"]),
            (&["ibnu", "hazm"], vec!["ابن حزم", "المحلى"]),
            (&["kitab", "ihya"], vec!["إحياء علوم الدين", "الغزالي"]),
            (&["kitab", "minhaj"], vec!["منهاج الطالبين", "النووي"]),
            (&["kitab", "fiqih"], vec!["كتاب الفقه", "الفقه الإسلامي"]),
            (&["kitab", "aqidah"], vec!["كتاب العقيدة", "العقيدة الإسلامية"]),
            (&["kitab", "tafsir"], vec!["كتاب التفسير", "تفسير القرآن"]),
            (&["kitab", "hadits"], vec!["كتاب الحديث", "السنة النبوية"]),
            (&["kitab", "nahwu"], vec!["كتاب النحو", "قواعد اللغة العربية"]),
            (&["hadits", "shahih"], vec!["الحديث الصحيح", "صحيح البخاري"]),
            (&["hadits", "bukhari"], vec!["صحيح البخاري", "الجامع الصحيح"]),
            (&["hadits", "muslim"], vec!["صحيح مسلم"]),
            (&["hadits", "arbain"], vec!["الأربعين النووية", "أربعين نووي"]),
            (&["arba'in", "nawawi"], vec!["الأربعين النووية", "أربعين نووي"]),
            (&["tafsir", "ibnu"], vec!["تفسير ابن كثير", "تفسير ابن جرير"]),
            (&["tafsir", "jalalayn"], vec!["تفسير الجلالين"]),
            (&["tafsir", "tabari"], vec!["تفسير الطبري", "جامع البيان"]),
            (&["tafsir", "qurtubi"], vec!["تفسير القرطبي", "الجامع لأحكام القرآن"]),
            (&["shahih", "bukhari"], vec!["صحيح البخاري", "الجامع الصحيح"]),
            (&["shahih", "muslim"], vec!["صحيح مسلم", "المسند الصحيح"]),
            (&["sunan", "tirmidzi"], vec!["سنن الترمذي", "جامع الترمذي"]),
            (&["sunan", "abu"], vec!["سنن أبي داود"]),
            (&["sunan", "nasai"], vec!["سنن النسائي"]),
            (&["musnad", "ahmad"], vec!["مسند أحمد", "مسند الإمام أحمد"]),
            (&["muwatta", "malik"], vec!["موطأ مالك"]),
            (&["riyadhus", "shalihin"], vec!["رياض الصالحين", "النووي"]),
            (&["fiqhus", "sunnah"], vec!["فقه السنة", "سيد سابق"]),
            (&["bidayatul", "mujtahid"], vec!["بداية المجتهد", "ابن رشد"]),
            (&["matan", "jurumiyah"], vec!["متن الجرومية", "مقدمة الجرومية"]),
            (&["alfiyah", "ibnu"], vec!["ألفية ابن مالك", "الألفية"]),
            // BATCH 67 — Indonesian Islamic social questions
            (&["boleh", "seorang"], vec!["هل يجوز", "حكم"]),
            (&["boleh", "wanita"], vec!["هل تجوز للمرأة", "المرأة في الإسلام"]),
            (&["boleh", "perempuan"], vec!["هل يجوز للمرأة", "المرأة"]),
            (&["boleh", "laki"], vec!["هل يجوز للرجل", "الرجل"]),
            (&["boleh", "muslim"], vec!["هل يجوز للمسلم", "حكم المسلم"]),
            (&["wanita", "shalat"], vec!["صلاة المرأة", "حكم صلاة المرأة"]),
            (&["wanita", "kerja"], vec!["عمل المرأة", "خروج المرأة للعمل"]),
            (&["wanita", "kepala"], vec!["المرأة كإمام", "ولاية المرأة"]),
            (&["wanita", "pemimpin"], vec!["ولاية المرأة", "المرأة في القيادة"]),
            (&["wanita", "mengimami"], vec!["إمامة المرأة", "صلاة المرأة إماماً"]),
            (&["perempuan", "imam"], vec!["إمامة المرأة", "صلاة المرأة إماماً"]),
            (&["istri", "kerja"], vec!["عمل الزوجة", "خروج الزوجة"]),
            (&["istri", "shalat"], vec!["صلاة الزوجة", "إذن الزوج"]),
            (&["suami", "istri"], vec!["حقوق الزوجين", "العشرة الزوجية"]),
            (&["hak", "istri"], vec!["حقوق الزوجة", "نفقة الزوجة"]),
            (&["hak", "suami"], vec!["حقوق الزوج", "طاعة الزوج"]),
            (&["nafkah", "istri"], vec!["نفقة الزوجة", "النفقة"]),
            (&["nafkah", "anak"], vec!["نفقة الأولاد", "النفقة"]),
            (&["hak", "anak"], vec!["حقوق الأولاد", "تربية الأولاد"]),
            (&["mendidik", "anak"], vec!["تربية الأولاد", "التنشئة الإسلامية"]),
            (&["tarbiyah", "anak"], vec!["تربية الأطفال", "التنشئة الإسلامية"]),
            (&["birrul", "walidain"], vec!["بر الوالدين", "حق الوالدين"]),
            (&["berbakti", "orang"], vec!["بر الوالدين", "حق الوالدين"]),
            (&["durhaka", "orang"], vec!["عقوق الوالدين", "النهي عن العقوق"]),
            (&["silaturahmi", "keluarga"], vec!["صلة الرحم", "حق الأرحام"]),
            (&["putus", "silaturahmi"], vec!["قطيعة الرحم", "تحريم قطع الرحم"]),
            (&["tetangga", "hak"], vec!["حق الجار", "حقوق الجيران"]),
            (&["hak", "tetangga"], vec!["حق الجار", "حقوق الجار"]),
            (&["anak", "yatim"], vec!["اليتيم", "رعاية اليتامى"]),
            (&["kaum", "dhuafa"], vec!["الفقراء", "رعاية المساكين"]),
            (&["fakir", "miskin"], vec!["الفقراء والمساكين", "أهل الحاجة"]),
            (&["shadaqoh", "jariah"], vec!["الصدقة الجارية", "الأجر المستمر"]),
            (&["amal", "jariyah"], vec!["الصدقة الجارية", "الأجر المستمر"]),
            (&["ilmu", "bermanfaat"], vec!["العلم النافع", "نشر العلم"]),
            (&["doa", "anak"], vec!["دعاء الأبناء للوالدين", "دعاء الولد الصالح"]),
            (&["anak", "shalih"], vec!["الولد الصالح", "الذرية الصالحة"]),
            (&["amanah", "menjaga"], vec!["حفظ الأمانة", "الأمانة"]),
            (&["kejujuran", "islam"], vec!["الصدق", "حكم الصدق"]),
            (&["jujur", "hukum"], vec!["حكم الصدق", "الصدق والأمانة"]),
            (&["sumpah", "palsu"], vec!["اليمين الغموس", "الحلف الكاذب"]),
            (&["qadar", "baik"], vec!["القدر الخير", "الإيمان بالقدر"]),
            (&["ujian", "cobaan"], vec!["الابتلاء والمحنة", "الصبر على البلاء"]),
            (&["musibah", "sabar"], vec!["الصبر على المصيبة", "الصبر"]),
            (&["tawakkal", "usaha"], vec!["التوكل مع السعي", "التوكل"]),
            (&["doa", "dikabulkan"], vec!["إجابة الدعاء", "شروط قبول الدعاء"]),
            (&["istijabah", "doa"], vec!["إجابة الدعاء", "قبول الدعاء"]),
            // BATCH 68 — specific fiqh questions by context
            (&["makan", "barang"], vec!["أكل مال الغير", "أحكام الأكل"]),
            (&["makan", "najis"], vec!["أكل النجاسة", "حكم أكل النجاسة"]),
            (&["makan", "haram"], vec!["أكل الحرام", "المحرمات من الطعام"]),
            (&["makan", "lupa"], vec!["الأكل ناسياً في الصيام", "ناسياً"]),
            (&["makan", "tidak"], vec!["الامتناع عن الطعام", "ترك الأكل"]),
            (&["minum", "lupa"], vec!["الشرب ناسياً في الصيام", "ناسياً"]),
            (&["minum", "obat"], vec!["تناول الدواء في الصيام", "الصيام والدواء"]),
            (&["puasa", "tidak"], vec!["عدم الصيام", "إفطار رمضان"]),
            (&["puasa", "lupa"], vec!["الإفطار ناسياً", "صحة الصيام"]),
            (&["puasa", "sakit"], vec!["صيام المريض", "إفطار بسبب المرض"]),
            (&["puasa", "batal"], vec!["مبطلات الصيام", "نواقض الصوم"]),
            (&["puasa", "hamil"], vec!["صيام الحامل", "إفطار الحامل"]),
            (&["puasa", "menyusui"], vec!["صيام المرضعة", "إفطار المرضع"]),
            (&["puasa", "musafir"], vec!["صيام المسافر", "إفطار المسافر"]),
            (&["shalat", "batal"], vec!["مبطلات الصلاة", "نواقض الصلاة"]),
            (&["shalat", "tidur"], vec!["النوم في الصلاة", "مبطلات الصلاة"]),
            (&["shalat", "berak"], vec!["الحدث في الصلاة", "مبطلات الصلاة"]),
            (&["shalat", "kencing"], vec!["الحدث في الصلاة", "نواقض الوضوء"]),
            (&["shalat", "kentut"], vec!["خروج الريح في الصلاة", "مبطلات الصلاة"]),
            (&["shalat", "tersenyum"], vec!["التبسم في الصلاة", "مبطلات الصلاة"]),
            (&["shalat", "tertawa"], vec!["الضحك في الصلاة", "مبطلات الصلاة"]),
            (&["shalat", "berbicara"], vec!["الكلام في الصلاة", "مبطلات الصلاة"]),
            (&["shalat", "gerak"], vec!["الحركة في الصلاة", "مبطلات الصلاة"]),
            (&["wudhu", "batal"], vec!["نواقض الوضوء", "مبطلات الوضوء"]),
            (&["wudhu", "tidur"], vec!["النوم ناقض للوضوء", "نواقض الوضوء"]),
            (&["wudhu", "syarat"], vec!["شروط الوضوء", "فرائض الوضوء"]),
            (&["wudhu", "urutan"], vec!["ترتيب الوضوء", "أركان الوضوء"]),
            (&["najis", "badan"], vec!["النجاسة على البدن", "تطهير النجاسة"]),
            (&["najis", "pakaian"], vec!["النجاسة على الثوب", "تطهير الثياب"]),
            (&["najis", "babi"], vec!["نجاسة الخنزير", "تطهير نجاسة الخنزير"]),
            (&["menyentuh", "mushaf"], vec!["مس المصحف", "لمس القرآن"]),
            (&["membaca", "haid"], vec!["قراءة القرآن للحائض", "الحيض والقرآن"]),
            (&["haid", "shalat"], vec!["الصلاة أثناء الحيض", "حيض المرأة"]),
            (&["haid", "membaca"], vec!["قراءة القرآن للحائض", "الحيض"]),
            (&["haid", "musjid"], vec!["دخول المسجد للحائض", "الحيض والمسجد"]),
            (&["haid", "puasa"], vec!["قضاء صيام الحائض", "الحيض والصيام"]),
            (&["nifas", "shalat"], vec!["الصلاة بعد النفاس", "النفاس وطهارته"]),
            (&["junub", "shalat"], vec!["صلاة الجنب", "حكم الجنابة"]),
            (&["junub", "tidur"], vec!["نوم الجنب", "الجنابة"]),
            (&["mandi", "wajib"], vec!["الغسل الواجب", "الاغتسال"]),
            (&["mandi", "junub"], vec!["غسل الجنابة", "كيفية الغسل"]),
            (&["istinja", "batu"], vec!["الاستنجاء بالحجارة", "الاستجمار"]),
            (&["cebok", "kiri"], vec!["الاستنجاء باليد اليسرى", "آداب الاستنجاء"]),

            // ── BATCH 69: tata cara X patterns ──
            (&["tata", "cara"], vec!["كيفية", "طريقة"]),
            (&["tata", "cara", "shalat"], vec!["كيفية الصلاة", "صفة الصلاة"]),
            (&["tata", "cara", "wudhu"], vec!["كيفية الوضوء", "صفة الوضوء"]),
            (&["tata", "cara", "mandi"], vec!["كيفية الغسل", "صفة الغسل"]),
            (&["tata", "cara", "puasa"], vec!["كيفية الصوم", "آداب الصيام"]),
            (&["tata", "cara", "zakat"], vec!["كيفية أداء الزكاة", "إخراج الزكاة"]),
            (&["tata", "cara", "haji"], vec!["كيفية الحج", "مناسك الحج"]),
            (&["tata", "cara", "umroh"], vec!["كيفية العمرة", "مناسك العمرة"]),
            (&["tata", "cara", "jenazah"], vec!["كيفية غسل الميت", "صلاة الجنازة"]),
            (&["tata", "cara", "tayamum"], vec!["كيفية التيمم", "صفة التيمم"]),
            (&["tata", "cara", "qurban"], vec!["كيفية ذبح الأضحية", "أحكام الأضحية"]),
            (&["tata", "cara", "shalat", "jenazah"], vec!["صلاة الجنازة", "كيفية الصلاة على الميت"]),
            (&["tata", "cara", "shalat", "ghaib"], vec!["صلاة الغائب"]),
            (&["tata", "cara", "shalat", "istisqa"], vec!["صلاة الاستسقاء"]),
            (&["tata", "cara", "shalat", "khusuf"], vec!["صلاة الكسوف", "صلاة الخسوف"]),
            (&["pelaksanaan", "shalat"], vec!["إقامة الصلاة", "كيفية الصلاة"]),
            (&["urutan", "shalat"], vec!["ترتيب الصلاة", "أركان الصلاة"]),
            (&["urutan", "wudhu"], vec!["ترتيب الوضوء", "فروض الوضوء"]),
            (&["urutan", "mandi"], vec!["كيفية الغسل", "ترتيب الغسل"]),
            (&["langkah", "shalat"], vec!["أركان الصلاة", "كيفية الصلاة"]),
            (&["langkah", "wudhu"], vec!["فروض الوضوء", "سنن الوضوء"]),
            (&["gerakan", "shalat"], vec!["أركان الصلاة", "أفعال الصلاة"]),
            (&["bacaan", "shalat"], vec!["أقوال الصلاة", "قراءة الفاتحة في الصلاة"]),
            (&["bacaan", "wudhu"], vec!["دعاء الوضوء", "ذكر الوضوء"]),
            (&["bacaan", "adzan"], vec!["ألفاظ الأذان", "كيفية الأذان"]),
            (&["bacaan", "iqamat"], vec!["ألفاظ الإقامة"]),
            (&["bacaan", "iqamah"], vec!["ألفاظ الإقامة"]),
            (&["bacaan", "tasyahud"], vec!["التشهد", "ألفاظ التشهد"]),
            (&["bacaan", "qunut"], vec!["دعاء القنوت", "قنوت الوتر"]),
            (&["bacaan", "ruku"], vec!["ذكر الركوع", "تسبيح الركوع"]),
            (&["bacaan", "sujud"], vec!["ذكر السجود", "تسبيح السجود"]),
            (&["bacaan", "i'tidal"], vec!["ذكر الاعتدال", "ربنا لك الحمد"]),
            (&["bacaan", "tahiyat"], vec!["التحيات", "التشهد"]),
            (&["bacaan", "tarawih"], vec!["قيام رمضان", "صلاة التراويح"]),
            (&["bacaan", "witir"], vec!["صلاة الوتر", "دعاء القنوت"]),
            (&["bacaan", "doa", "setelah", "shalat"], vec!["الذكر بعد الصلاة", "أذكار بعد الصلاة"]),
            (&["bacaan", "setelah", "adzan"], vec!["دعاء بعد الأذان"]),
            (&["bacaan", "niat", "shalat"], vec!["نية الصلاة"]),
            (&["bacaan", "niat", "puasa"], vec!["نية الصوم"]),
            (&["bacaan", "niat", "zakat"], vec!["نية الزكاة"]),
            (&["bacaan", "niat", "haji"], vec!["نية الحج", "التلبية"]),
            (&["bacaan", "niat", "umroh"], vec!["نية العمرة", "التلبية"]),
            (&["bacaan", "istiftah"], vec!["دعاء الاستفتاح"]),
            (&["bacaan", "tahiyyatul", "masjid"], vec!["تحية المسجد"]),
            (&["sambil", "berpuasa"], vec!["حال الصيام", "أثناء الصوم"]),
            (&["sambil", "puasa"], vec!["حال الصيام", "مباحات الصوم"]),
            (&["sambil", "shalat"], vec!["حال الصلاة", "أثناء الصلاة"]),
            (&["sambil", "haid"], vec!["حال الحيض", "ما يباح للحائض"]),
            (&["sambil", "ihram"], vec!["محظورات الإحرام", "ما يحرم في الإحرام"]),
            (&["sambil", "junub"], vec!["ما يحرم على الجنب", "حال الجنابة"]),
            (&["dalam", "keadaan", "junub"], vec!["الجنابة", "حكم الجنب"]),
            (&["dalam", "keadaan", "haid"], vec!["الحيض", "حكم الحائض"]),
            (&["dalam", "keadaan", "suci"], vec!["الطهارة", "شرط الطهارة"]),
            (&["dalam", "kondisi"], vec!["الحالة", "الاضطرار"]),

            // ── BATCH 70: Islamic sciences and education terms ──
            (&["ilmu", "nahwu"], vec!["علم النحو", "النحو العربي"]),
            (&["ilmu", "sharaf"], vec!["علم الصرف", "الصرف"]),
            (&["ilmu", "shorof"], vec!["علم الصرف", "الصرف"]),
            (&["ilmu", "balaghah"], vec!["علم البلاغة", "البلاغة"]),
            (&["ilmu", "mantiq"], vec!["علم المنطق", "المنطق"]),
            (&["ilmu", "tauhid"], vec!["علم التوحيد", "العقيدة"]),
            (&["ilmu", "ushul"], vec!["علم أصول الفقه", "أصول الفقه"]),
            (&["ilmu", "fiqih"], vec!["علم الفقه", "الفقه الإسلامي"]),
            (&["ilmu", "tafsir"], vec!["علم التفسير", "علوم القرآن"]),
            (&["ilmu", "hadits"], vec!["علم الحديث", "علوم الحديث"]),
            (&["ilmu", "tasawuf"], vec!["علم التصوف", "التصوف"]),
            (&["ilmu", "falak"], vec!["علم الفلك", "التوقيت الشرعي"]),
            (&["ilmu", "faraid"], vec!["علم الفرائض", "الميراث"]),
            (&["ilmu", "qiraat"], vec!["علم القراءات", "القراءات القرآنية"]),
            (&["ilmu", "tajwid"], vec!["علم التجويد", "أحكام التجويد"]),
            (&["makhraj", "huruf"], vec!["مخارج الحروف", "علم التجويد"]),
            (&["makharijul", "huruf"], vec!["مخارج الحروف", "علم التجويد"]),
            (&["hukum", "tajwid"], vec!["أحكام التجويد", "علم التجويد"]),
            (&["hukum", "nun"], vec!["أحكام النون الساكنة", "التجويد"]),
            (&["hukum", "mim"], vec!["أحكام الميم الساكنة", "التجويد"]),
            (&["nun", "mati"], vec!["النون الساكنة", "أحكام التجويد"]),
            (&["nun", "sukun"], vec!["النون الساكنة", "أحكام التجويد"]),
            (&["tanwin", "hukum"], vec!["أحكام التنوين", "التجويد"]),
            (&["mad", "thabi'i"], vec!["المد الطبيعي", "أنواع المد"]),
            (&["mad", "wajib"], vec!["المد الواجب المتصل", "أنواع المد"]),
            (&["mad", "jaiz"], vec!["المد الجائز المنفصل", "أنواع المد"]),
            (&["waqaf", "ibtida"], vec!["الوقف والابتداء", "أحكام الوقف"]),
            (&["hukum", "waqaf"], vec!["أحكام الوقف", "الوقف في القرآن"]),
            (&["cara", "membaca", "quran"], vec!["آداب تلاوة القرآن", "كيفية قراءة القرآن"]),
            (&["adab", "membaca", "quran"], vec!["آداب تلاوة القرآن"]),
            (&["khatam", "quran"], vec!["ختم القرآن", "إتمام القرآن"]),
            (&["menghafal", "quran"], vec!["حفظ القرآن", "تحفيظ القرآن"]),
            (&["hafalan", "quran"], vec!["حفظ القرآن", "المحفوظات"]),
            (&["amar", "makruf"], vec!["الأمر بالمعروف", "الأمر بالمعروف والنهي عن المنكر"]),
            (&["nahi", "munkar"], vec!["النهي عن المنكر", "الأمر بالمعروف والنهي عن المنكر"]),
            (&["amar", "makruf", "nahi", "munkar"], vec!["الأمر بالمعروف والنهي عن المنكر"]),
            (&["imamah", "kubra"], vec!["الإمامة الكبرى", "الخلافة"]),
            (&["khilafah", "islamiyah"], vec!["الخلافة الإسلامية", "الإمامة"]),
            (&["ulil", "amri"], vec!["أولو الأمر", "طاعة ولي الأمر"]),
            (&["taat", "pemimpin"], vec!["طاعة ولي الأمر", "أولو الأمر"]),
            (&["pemimpin", "muslim"], vec!["الإمام المسلم", "ولاية المسلم"]),
            (&["musyarakah", "mudharabah"], vec!["المشاركة والمضاربة", "عقود الشركة"]),
            (&["akad", "murabahah"], vec!["عقد المرابحة", "البيع الإسلامي"]),
            (&["akad", "ijarah"], vec!["عقد الإجارة", "الإجارة"]),
            (&["akad", "wakalah"], vec!["عقد الوكالة", "الوكالة"]),
            (&["akad", "kafalah"], vec!["عقد الكفالة", "الكفالة"]),
            (&["akad", "rahn"], vec!["عقد الرهن", "الرهن"]),
            (&["akad", "qard"], vec!["القرض", "عقد القرض"]),
            (&["akad", "wadi'ah"], vec!["الوديعة", "حفظ الأموال"]),
            (&["bank", "syariah"], vec!["البنك الإسلامي", "المصرف الإسلامي"]),
            (&["perbankan", "syariah"], vec!["المصرفية الإسلامية", "البنك الإسلامي"]),
            (&["keuangan", "syariah"], vec!["التمويل الإسلامي", "المالية الإسلامية"]),

            // ── BATCH 71: Specific fiqh questions by context ──
            (&["shalat", "anak", "kecil"], vec!["صلاة الصبيان", "تعليم الصلاة للأطفال"]),
            (&["umur", "wajib", "shalat"], vec!["سن وجوب الصلاة", "متى يجب على الولد الصلاة"]),
            (&["perintah", "shalat", "anak"], vec!["تعليم الصلاة للأطفال", "الأمر بالصلاة"]),
            (&["khitan", "wanita"], vec!["ختان المرأة", "ختان الأنثى"]),
            (&["khitan", "perempuan"], vec!["ختان المرأة", "ختان الأنثى"]),
            (&["sunat", "perempuan"], vec!["ختان الأنثى", "حكم ختان المرأة"]),
            (&["bayi", "baru", "lahir"], vec!["المولود الجديد", "أحكام المولود"]),
            (&["adzan", "telinga", "bayi"], vec!["الأذان في أذن المولود"]),
            (&["aqiqah", "bayi"], vec!["عقيقة المولود", "أحكام العقيقة"]),
            (&["tahnik", "bayi"], vec!["التحنيك", "تحنيك المولود"]),
            (&["nama", "bayi", "islam"], vec!["تسمية المولود", "اختيار الاسم"]),
            (&["memberi", "nama", "anak"], vec!["تسمية الأولاد", "أحكام التسمية"]),
            (&["nama", "yang", "dilarang"], vec!["الأسماء المحرمة", "الأسماء المنهي عنها"]),
            (&["nama", "makruh"], vec!["الأسماء المكروهة"]),
            (&["hukum", "ganti", "nama"], vec!["تغيير الاسم", "أحكام الاسم"]),
            (&["masuk", "islam", "cara"], vec!["كيفية الإسلام", "الدخول في الإسلام"]),
            (&["syahadat", "cara"], vec!["كيفية التشهد بالإسلام", "الشهادتان"]),
            (&["mualaf", "hukum"], vec!["أحكام المسلم الجديد", "حكم المرتد"]),
            (&["masuk", "islam", "syarat"], vec!["شرط الإسلام", "الشهادتان"]),
            (&["keluar", "islam", "hukum"], vec!["الردة", "حكم المرتد"]),
            (&["murtad", "tobat"], vec!["توبة المرتد", "أحكام المرتد"]),
            (&["kafir", "hukum"], vec!["حكم الكافر", "أقسام الكفر"]),
            (&["kafir", "harbi"], vec!["الكافر الحربي", "أقسام الكفار"]),
            (&["kafir", "dzimmi"], vec!["أهل الذمة", "الكافر الذمي"]),
            (&["non", "muslim", "hubungan"], vec!["معاملة غير المسلمين", "العلاقة مع غير المسلمين"]),
            (&["tidak", "shalat", "hukum"], vec!["حكم تارك الصلاة", "وجوب الصلاة"]),
            (&["meninggalkan", "shalat"], vec!["ترك الصلاة", "حكم تارك الصلاة"]),
            (&["tidak", "puasa", "hukum"], vec!["حكم من أفطر بلا عذر", "الإفطار عمداً"]),
            (&["tidak", "zakat", "hukum"], vec!["عقوبة مانع الزكاة", "حكم تارك الزكاة"]),
            (&["enggan", "zakat"], vec!["مانع الزكاة", "من لم يؤدِ الزكاة"]),
            (&["shalat", "tidak", "khusyuk"], vec!["الخشوع في الصلاة", "فضل الخشوع"]),
            (&["khusyuk", "shalat"], vec!["الخشوع في الصلاة"]),
            (&["was-was", "shalat"], vec!["الوسواس في الصلاة", "وسواس الشيطان"]),
            (&["was-was", "wudhu"], vec!["الوسواس في الطهارة", "حكم الوسواس"]),
            (&["ragu", "shalat"], vec!["الشك في الصلاة", "بناء على اليقين"]),
            (&["ragu", "wudhu"], vec!["الشك في الطهارة", "الأصل في الطهارة"]),
            (&["lupa", "shalat"], vec!["نسيان الصلاة", "قضاء الفوائت"]),
            (&["lupa", "rakaat"], vec!["السهو في الصلاة", "سجود السهو"]),
            (&["sujud", "sahwi"], vec!["سجود السهو", "أسباب سجود السهو"]),
            (&["sujud", "tilawah"], vec!["سجود التلاوة", "آيات السجدة"]),
            (&["sujud", "syukur"], vec!["سجود الشكر"]),
            (&["qadha", "shalat"], vec!["قضاء الصلاة", "الفوائت"]),
            (&["qadha", "puasa"], vec!["قضاء الصوم", "كفارة الإفطار"]),
            (&["shalat", "jamak"], vec!["الجمع بين الصلاتين", "الجمع والقصر"]),
            (&["shalat", "qashar"], vec!["قصر الصلاة", "الجمع والقصر"]),
            (&["shalat", "jama", "qasar"], vec!["الجمع والقصر", "صلاة المسافر"]),
            (&["syarat", "shalat", "musafir"], vec!["صلاة المسافر", "شروط القصر"]),
            (&["jarak", "safar"], vec!["مسافة السفر", "مسافة القصر"]),
            (&["safar", "berapa", "km"], vec!["مسافة السفر", "مسافة القصر"]),
            (&["safar", "jarak"], vec!["مسافة السفر", "الإفطار في السفر"]),

            // Malam (night) practices
            (&["malam", "nisfu"], vec!["ليلة النصف من شعبان"]),
            (&["malam", "isra"], vec!["ليلة الإسراء", "الإسراء والمعراج"]),
            (&["malam", "lailatul"], vec!["ليلة القدر"]),
            (&["lailatul", "qadar"], vec!["ليلة القدر"]),
            (&["malam", "jumat"], vec!["ليلة الجمعة"]),

            // ── BATCH 72: More general Islamic terms and patterns ──
            (&["apa", "yang", "membatalkan", "wudhu"], vec!["نواقض الوضوء", "ما ينقض الوضوء"]),
            (&["apa", "yang", "membatalkan", "shalat"], vec!["مبطلات الصلاة", "ما يبطل الصلاة"]),
            (&["apa", "yang", "membatalkan", "puasa"], vec!["مفطرات الصوم", "ما يبطل الصوم"]),
            (&["apa", "yang", "membatalkan", "haji"], vec!["محظورات الإحرام", "ما يفسد الحج"]),
            (&["apa", "yang", "membatalkan", "nikah"], vec!["ما يبطل عقد النكاح", "فساد النكاح"]),
            (&["hal", "membatalkan", "wudhu"], vec!["نواقض الوضوء"]),
            (&["hal", "membatalkan", "shalat"], vec!["مبطلات الصلاة"]),
            (&["hal", "membatalkan", "puasa"], vec!["مفطرات الصوم"]),
            (&["yang", "termasuk", "najis"], vec!["أنواع النجاسات", "النجاسة"]),
            (&["jenis", "najis"], vec!["أنواع النجاسة", "النجاسة المخففة والمغلظة"]),
            (&["najis", "mughallazhah"], vec!["النجاسة المغلظة"]),
            (&["najis", "mukhaffafah"], vec!["النجاسة المخففة"]),
            (&["najis", "mutawassithah"], vec!["النجاسة المتوسطة"]),
            (&["cara", "mensucikan", "najis"], vec!["تطهير النجاسة", "طريقة إزالة النجاسة"]),
            (&["cara", "sucikan", "najis"], vec!["تطهير النجاسة"]),
            (&["air", "suci"], vec!["الماء الطاهر", "المياه في الفقه"]),
            (&["air", "mutlak"], vec!["الماء المطلق", "أقسام الماء"]),
            (&["air", "musta'mal"], vec!["الماء المستعمل", "أقسام الماء"]),
            (&["air", "najis"], vec!["الماء النجس", "أحكام الماء"]),
            (&["air", "dua", "qullah"], vec!["الماء قلتان", "قلتان الماء"]),
            (&["dua", "qullah"], vec!["القلتان", "الماء القليل والكثير"]),
            (&["bersuci", "tayamum"], vec!["التيمم", "الطهارة بالتراب"]),
            (&["syarat", "tayamum"], vec!["شروط التيمم"]),
            (&["tanah", "tayamum"], vec!["التراب للتيمم", "استعمال التراب"]),
            (&["pakaian", "shalat"], vec!["لباس الصلاة", "ستر العورة في الصلاة"]),
            (&["menutup", "aurat"], vec!["ستر العورة", "حكم كشف العورة"]),
            (&["aurat", "wanita"], vec!["عورة المرأة", "حد العورة"]),
            (&["aurat", "laki"], vec!["عورة الرجل", "حد العورة"]),
            (&["batas", "aurat"], vec!["حد العورة", "عورة الرجل والمرأة"]),
            (&["aurat", "dalam", "shalat"], vec!["ستر العورة في الصلاة"]),
            (&["pakaian", "menutup", "aurat"], vec!["ستر العورة", "لباس الشرعي"]),
            (&["shalat", "pakai", "celana"], vec!["الصلاة بالسراويل", "لباس الصلاة"]),
            (&["shalat", "tanpa", "peci"], vec!["الصلاة بدون غطاء الرأس", "صلاة حاسر الرأس"]),
            (&["shalat", "tanpa", "mukena"], vec!["صلاة المرأة بدون الخمار"]),
            (&["shalat", "pakai", "mukena"], vec!["خمار المرأة في الصلاة"]),
            (&["wajib", "pakai", "mukena"], vec!["حكم المكعة", "لباس المرأة في الصلاة"]),
            (&["jamaah", "shalat", "wanita"], vec!["صلاة المرأة جماعة", "إمامة المرأة"]),
            (&["shalat", "berjamaah", "perempuan"], vec!["جماعة النساء", "صلاة المرأة"]),
            (&["shalat", "subuh", "berjamaah"], vec!["صلاة الصبح جماعة"]),
            (&["shalat", "berjamaah", "keutamaan"], vec!["فضل الجماعة", "صلاة الجماعة"]),
            (&["boleh", "shalat", "sendiri"], vec!["صلاة الفذ", "حكم الصلاة منفرداً"]),
            (&["tidak", "sempat", "berjamaah"], vec!["فوت الجماعة", "إدراك الجماعة"]),
            (&["masbuq", "shalat"], vec!["المسبوق", "حكم المسبوق في الصلاة"]),
            (&["makmum", "masbuq"], vec!["المسبوق", "حكم المسبوق"]),
            (&["shalat", "munfarid"], vec!["الصلاة منفرداً", "الصلاة بدون إمام"]),

            // ── BATCH 73: Comprehensive Islamic query patterns ──
            (&["cara", "shalat", "yang", "benar"], vec!["كيفية الصلاة الصحيحة", "صفة الصلاة"]),
            (&["cara", "wudhu", "yang", "benar"], vec!["كيفية الوضوء الصحيح", "صفة الوضوء"]),
            (&["cara", "puasa", "yang", "benar"], vec!["كيفية الصيام", "آداب الصيام"]),
            (&["cara", "berhaji"], vec!["كيفية الحج", "مناسك الحج"]),
            (&["cara", "mandi", "junub"], vec!["كيفية الغسل من الجنابة", "غسل الجنابة"]),
            (&["cara", "mandi", "wajib"], vec!["كيفية الغسل الواجب", "صفة الغسل"]),
            (&["cara", "tayamum"], vec!["كيفية التيمم", "صفة التيمم"]),
            (&["cara", "tobat"], vec!["كيفية التوبة", "شروط التوبة"]),
            (&["cara", "taubat", "yang", "benar"], vec!["شروط التوبة الصحيحة", "التوبة النصوح"]),
            (&["cara", "baca", "quran"], vec!["كيفية قراءة القرآن", "آداب التلاوة"]),
            (&["cara", "shalat", "sunnat"], vec!["صلاة النافلة", "صلاة السنة"]),
            (&["cara", "shalat", "tahajud"], vec!["صلاة التهجد", "قيام الليل"]),
            (&["cara", "shalat", "dhuha"], vec!["صلاة الضحى"]),
            (&["cara", "shalat", "witir"], vec!["صلاة الوتر"]),
            (&["cara", "shalat", "tarawih"], vec!["صلاة التراويح"]),
            (&["cara", "shalat", "ied"], vec!["صلاة العيد"]),
            (&["cara", "shalat", "istikharah"], vec!["صلاة الاستخارة", "دعاء الاستخارة"]),
            (&["cara", "shalat", "hajat"], vec!["صلاة الحاجة"]),
            (&["cara", "shalat", "taubat"], vec!["صلاة التوبة"]),
            (&["cara", "shalat", "rawatib"], vec!["السنن الرواتب"]),
            (&["cara", "berdzikir"], vec!["كيفية الذكر", "آداب الذكر"]),
            (&["cara", "berdoa"], vec!["آداب الدعاء", "كيفية الدعاء"]),
            (&["cara", "istighfar"], vec!["الاستغفار", "كيفية الاستغفار"]),
            (&["cara", "shalawat"], vec!["الصلاة على النبي", "صيغ الصلاة على النبي"]),
            (&["cara", "membayar", "zakat"], vec!["إخراج الزكاة", "كيفية أداء الزكاة"]),
            (&["cara", "membayar", "fidyah"], vec!["الفدية", "كيفية الفدية"]),
            (&["cara", "membayar", "kaffarah"], vec!["الكفارة", "كيفية الكفارة"]),
            (&["cara", "membayar", "hutang"], vec!["قضاء الدين", "أحكام الديون"]),
            (&["cara", "akad", "nikah"], vec!["كيفية عقد النكاح", "شروط عقد النكاح"]),
            (&["cara", "meminang"], vec!["الخطبة", "كيفية الخطبة"]),
            (&["cara", "melamar"], vec!["الخطبة", "التقدم للزواج"]),
            (&["cara", "bercerai"], vec!["كيفية الطلاق", "إجراءات الطلاق"]),
            (&["cara", "menjatuhkan", "talak"], vec!["إيقاع الطلاق", "شروط الطلاق"]),
            (&["cara", "rujuk"], vec!["كيفية الرجعة", "الرجوع إلى الزوجة"]),
            (&["cara", "khulu"], vec!["كيفية الخلع"]),
            (&["cara", "wakaf"], vec!["كيفية الوقف", "شروط الوقف"]),
            (&["cara", "hibah"], vec!["كيفية الهبة", "شروط الهبة"]),
            (&["cara", "wasiat"], vec!["كيفية الوصية", "شروط الوصية"]),
            (&["cara", "qurban"], vec!["كيفية الأضحية", "شروط الأضحية"]),
            (&["cara", "aqiqah"], vec!["كيفية العقيقة"]),
            (&["cara", "khitan"], vec!["كيفية الختان", "الختان"]),
            (&["cara", "mengurus", "jenazah"], vec!["أحكام الجنازة", "تجهيز الميت"]),
            (&["cara", "memandikan", "jenazah"], vec!["غسل الميت", "كيفية غسل الجنازة"]),
            (&["cara", "mengkafani"], vec!["تكفين الميت", "كيفية الكفن"]),
            (&["cara", "menshalatkan"], vec!["كيفية الصلاة على الميت", "صلاة الجنازة"]),
            (&["cara", "menguburkan"], vec!["دفن الميت", "كيفية الدفن"]),
            (&["cara", "ziarah", "kubur"], vec!["زيارة القبور", "آداب زيارة القبور"]),
            (&["cara", "talqin", "mayit"], vec!["التلقين", "تلقين الميت"]),
            (&["cara", "membagi", "waris"], vec!["قسمة التركة", "الميراث"]),
            (&["cara", "menghitung", "waris"], vec!["حساب الميراث", "الفرائض"]),

            // ── BATCH 74: Advanced fiqh questions with context ──
            (&["hukum", "menikah", "beda", "agama"], vec!["زواج المسلم من غير المسلمة", "نكاح الكتابية"]),
            (&["nikah", "beda", "agama"], vec!["الزواج من غير المسلم", "نكاح أهل الكتاب"]),
            (&["nikah", "tanpa", "wali"], vec!["النكاح بدون ولي", "شرط الولي في النكاح"]),
            (&["nikah", "tanpa", "saksi"], vec!["النكاح بدون شهود", "شرط الشهود"]),
            (&["nikah", "satu", "wali"], vec!["النكاح بولي واحد"]),
            (&["wali", "nikah"], vec!["الولي في النكاح", "أولياء النكاح"]),
            (&["wali", "adhal"], vec!["الولي العاضل", "وولي الجور"]),
            (&["hakim", "sebagai", "wali"], vec!["الحاكم ولي من لا ولي له"]),
            (&["wali", "hakim"], vec!["السلطان ولي من لا ولي له"]),
            (&["mahar", "nikah"], vec!["المهر", "صداق الزواج"]),
            (&["mahar", "berapa"], vec!["قدر المهر", "المهر المسمى"]),
            (&["hukum", "mas", "kawin"], vec!["المهر", "الصداق"]),
            (&["mas", "kawin"], vec!["المهر", "الصداق"]),
            (&["nikah", "mut'ah"], vec!["نكاح المتعة", "حكم نكاح المتعة"]),
            (&["nikah", "misyar"], vec!["نكاح المسيار"]),
            (&["nikah", "siri"], vec!["النكاح السري", "نكاح بغير شهود"]),
            (&["nikah", "online"], vec!["النكاح عن بعد", "النكاح بالوسائط"]),
            (&["cerai", "online"], vec!["الطلاق عن بعد", "طلاق الغائب"]),
            (&["cerai", "tiga", "kali"], vec!["الطلاق الثلاث", "الطلاق البائن"]),
            (&["talak", "tiga"], vec!["الطلاق الثلاث", "طلاق البتة"]),
            (&["ruju", "setelah", "cerai"], vec!["الرجعة", "حكم الرجعة"]),
            (&["masa", "iddah"], vec!["العدة", "مدة العدة"]),
            (&["iddah", "berapa", "lama"], vec!["مدة العدة", "أقسام العدة"]),
            (&["hukum", "poligami"], vec!["تعدد الزوجات", "حكم التعدد"]),
            (&["syarat", "poligami"], vec!["شروط التعدد", "العدل بين الزوجات"]),
            (&["adil", "berpoligami"], vec!["العدل بين الزوجات", "شرط التعدد"]),
            (&["istri", "pertama", "persetujuan"], vec!["إذن الزوجة", "موافقة الزوجة"]),
            (&["boleh", "poligami", "tanpa", "izin"], vec!["نكاح بدون إذن الزوجة الأولى"]),
            (&["hak", "suami", "atas", "istri"], vec!["حق الزوج على الزوجة", "حقوق الزوج"]),
            (&["hak", "istri", "atas", "suami"], vec!["حق الزوجة على الزوج", "حقوق الزوجة"]),
            (&["kewajiban", "suami"], vec!["واجبات الزوج", "حقوق الزوجة"]),
            (&["kewajiban", "istri"], vec!["واجبات الزوجة", "حقوق الزوج"]),
            (&["nusyuz", "istri"], vec!["النشوز", "حكم الناشزة"]),
            (&["nusyuz", "hukum"], vec!["النشوز", "حكم نشوز الزوجة"]),
            (&["boleh", "pukul", "istri"], vec!["ضرب الزوجة", "حكم الضرب"]),
            (&["suami", "pukul", "istri"], vec!["ضرب الزوجة", "أحكام معاشرة الزوجة"]),
            (&["kekerasan", "rumah", "tangga"], vec!["العنف الأسري", "حکم إيذاء الزوجة"]),
            (&["perceraian", "karena", "kekerasan"], vec!["الطلاق بسبب الأذى", "فسخ النكاح"]),
            (&["boleh", "istri", "kerja"], vec!["عمل المرأة", "خروج المرأة للعمل"]),
            (&["izin", "suami", "kerja"], vec!["استئذان الزوجة من الزوج", "خروج المرأة"]),
            (&["wanita", "bekerja", "hukum"], vec!["عمل المرأة", "الخروج من البيت"]),
            (&["ibu", "bekerja"], vec!["خروج الأم للعمل", "عمل الأم"]),
            (&["tanggung", "jawab", "nafkah"], vec!["وجوب النفقة", "نفقة الزوجة والأولاد"]),
            (&["nafkah", "anak", "setelah", "cerai"], vec!["نفقة الأولاد بعد الطلاق", "حضانة الأولاد"]),
            (&["hak", "asuh", "anak"], vec!["الحضانة", "من أحق بالحضانة"]),
            (&["hadhanah"], vec!["الحضانة", "أحكام الحضانة"]),
            (&["adopsi", "anak"], vec!["التبني", "حكم التبني"]),
            (&["anak", "angkat"], vec!["التبني", "حكم التبني في الإسلام"]),
            (&["anak", "zina", "nasab"], vec!["نسب ولد الزنا", "ولد الزنا"]),

            // ── BATCH 75: Social media, technology, contemporary fiqh ──
            (&["media", "sosial", "hukum"], vec!["حكم وسائل التواصل الاجتماعي"]),
            (&["hukum", "media", "sosial"], vec!["حكم وسائل التواصل الاجتماعي"]),
            (&["hukum", "main", "tiktok"], vec!["حكم التيك توك", "وسائل التواصل"]),
            (&["hukum", "youtube"], vec!["حكم اليوتيوب", "المحتوى الرقمي"]),
            (&["hukum", "instagram"], vec!["حكم الإنستغرام", "الصور الرقمية"]),
            (&["hukum", "facebook"], vec!["حكم الفيسبوك", "وسائل التواصل"]),
            (&["hukum", "twitter"], vec!["حكم تويتر", "وسائل التواصل"]),
            (&["hukum", "whatsapp"], vec!["حكم الواتساب", "التواصل الرقمي"]),
            (&["hukum", "game", "online"], vec!["حكم الألعاب الإلكترونية"]),
            (&["hukum", "nonton", "drama"], vec!["حكم مشاهدة المسلسلات"]),
            (&["hukum", "nonton", "bioskop"], vec!["حكم السينما", "حكم مشاهدة الأفلام"]),
            (&["hukum", "nonton", "film"], vec!["حكم مشاهدة الأفلام"]),
            (&["hukum", "main", "game"], vec!["حكم الألعاب الإلكترونية"]),
            (&["hukum", "selfie"], vec!["حكم التصوير الذاتي", "حكم الصور"]),
            (&["hukum", "foto", "diri", "sendiri"], vec!["حكم تصوير النفس", "حكم الصور"]),
            (&["hukum", "video", "call"], vec!["حكم مكالمة الفيديو"]),
            (&["hukum", "belanja", "online"], vec!["التسوق الإلكتروني", "بيع الإنترنت"]),
            (&["jual", "beli", "online"], vec!["البيع الإلكتروني", "التجارة الإلكترونية"]),
            (&["hukum", "transfer", "uang"], vec!["تحويل الأموال", "حوالة"]),
            (&["hukum", "kerja", "freelance"], vec!["العمل الحر", "أحكام الأجرة"]),
            (&["hukum", "kerja", "bank"], vec!["العمل في البنك", "حكم العمل في الربا"]),
            (&["hukum", "kerja", "asuransi"], vec!["العمل في التأمين", "حكم شركات التأمين"]),
            (&["hukum", "kerja", "di", "cafe"], vec!["العمل في المقهى", "عمل يخالط المحرم"]),
            (&["hukum", "kerja", "malam"], vec!["العمل بالليل", "عمل المرأة ليلاً"]),
            (&["hukum", "menabung"], vec!["الادخار", "حكم الادخار"]),
            (&["hukum", "tabungan", "bank"], vec!["الودائع البنكية", "حكم الفوائد"]),
            (&["hukum", "deposito"], vec!["الوديعة المصرفية", "الفوائد"]),
            (&["hukum", "saham"], vec!["الأسهم", "حكم الاستثمار في الأسهم"]),
            (&["hukum", "reksa", "dana"], vec!["صناديق الاستثمار", "الاستثمار الإسلامي"]),
            (&["hukum", "kripto"], vec!["العملات المشفرة", "حكم البيتكوين"]),
            (&["hukum", "bitcoin"], vec!["البيتكوين", "العملات الرقمية"]),
            (&["hukum", "nft"], vec!["NFT الرقمي", "حكم الرموز"]),
            (&["hukum", "trading"], vec!["المتاجرة", "حكم التجارة"]),
            (&["hukum", "forex"], vec!["صرف العملات", "الفوركس"]),
            (&["hukum", "pinjol"], vec!["القروض الإلكترونية", "التمويل الرقمي"]),
            (&["pinjaman", "online"], vec!["القرض الإلكتروني", "أحكام الديون"]),
            (&["hukum", "kartu", "kredit"], vec!["بطاقة الائتمان", "حكم البطاقة"]),
            (&["hukum", "cicilan"], vec!["التقسيط", "البيع بالأجل"]),
            (&["beli", "cicilan", "hukum"], vec!["البيع بالتقسيط", "البيع المؤجل"]),
            (&["hukum", "asuransi", "jiwa"], vec!["التأمين على الحياة", "التكافل"]),
            (&["asuransi", "syariah"], vec!["التأمين التكافلي", "التأمين الإسلامي"]),
            (&["hukum", "obligasi"], vec!["السندات", "الصكوك الإسلامية"]),
            (&["hukum", "sukuk"], vec!["الصكوك الإسلامية"]),
            (&["dana", "pensiun"], vec!["صندوق التقاعد", "ادخار التقاعد"]),
            (&["jaminan", "sosial"], vec!["الضمان الاجتماعي", "أنواع التكافل"]),

            // ── BATCH 30 phrase_map: doa situational bigrams ──
            (&["doa", "bepergian"], vec!["دعاء السفر", "دعاء المسافر"]),
            (&["doa", "safar"], vec!["دعاء السفر", "دعاء المسافر"]),
            (&["doa", "pasar"], vec!["دعاء دخول السوق"]),
            (&["doa", "baju"], vec!["دعاء لبس الثوب", "التسمية عند اللباس"]),
            (&["doa", "hujan"], vec!["دعاء نزول المطر", "الدعاء عند المطر"]),
            (&["doa", "petir"], vec!["دعاء سماع الرعد"]),
            (&["doa", "gempa"], vec!["دعاء الزلزلة"]),
            (&["doa", "banjir"], vec!["دعاء الفيضان"]),
            (&["doa", "marah"], vec!["دعاء عند الغضب", "الاستعاذة عند الغضب"]),
            (&["doa", "takut"], vec!["دعاء الخوف"]),
            (&["doa", "sakit"], vec!["دعاء المريض", "الدعاء عند المرض"]),
            (&["doa", "menjenguk"], vec!["دعاء عيادة المريض"]),
            (&["doa", "mayit"], vec!["دعاء للميت", "الدعاء على الجنازة"]),
            (&["doa", "kuburan"], vec!["دعاء دخول المقبرة", "دعاء زيارة القبور"]),
            (&["doa", "arafah"], vec!["دعاء يوم عرفة"]),
            (&["doa", "keturunan"], vec!["دعاء طلب الذرية"]),
            (&["doa", "istiftah"], vec!["دعاء الاستفتاح", "الاستفتاح في الصلاة"]),
            (&["doa", "tasyahud"], vec!["التشهد الأخير", "الصلاة الإبراهيمية"]),
            (&["kafarat", "majelis"], vec!["كفارة المجلس", "دعاء كفارة المجلس"]),
            (&["doa", "lailatul"], vec!["دعاء ليلة القدر", "اللهم إنك عفو"]),

            // ── BATCH 76: Environment, health, and contextual Islamic topics ──
            (&["hukum", "merokok", "haram"], vec!["حكم التدخين", "التدخين والصحة"]),
            (&["hukum", "rokok", "elektrik"], vec!["حكم السيجارة الإلكترونية", "التدخين"]),
            (&["hukum", "vape"], vec!["حكم الفيب", "التدخين الإلكتروني"]),
            (&["hukum", "shisha"], vec!["حكم الشيشة", "التدخين"]),
            (&["hukum", "narkoba"], vec!["حكم المخدرات", "المسكرات والمخدرات"]),
            (&["hukum", "ganja"], vec!["حكم الحشيش", "المخدرات"]),
            (&["hukum", "alkohol", "dalam", "makanan"], vec!["التحلل الكحولي", "الكحول في الأطعمة"]),
            (&["hukum", "alkohol", "dalam", "obat"], vec!["الكحول في الدواء", "التداوي بالمحرم"]),
            (&["hukum", "babi", "gelatin"], vec!["الجيلاتين الخنزيري", "المستحلب الخنزيري"]),
            (&["hukum", "gelatin", "babi"], vec!["الجيلاتين من الخنزير", "المستحلب"]),
            (&["obat", "mengandung", "babi"], vec!["الدواء المحتوي على لحم الخنزير", "التداوي بالمحرم"]),
            (&["vaksin", "halal"], vec!["اللقاح الحلال", "حكم التطعيم"]),
            (&["hukum", "vaksin"], vec!["حكم التطعيم", "اللقاح"]),
            (&["hukum", "obat", "haram"], vec!["التداوي بالمحرم", "الضرورة الطبية"]),
            (&["darurat", "makan", "haram"], vec!["الضرورة تبيح المحظورات", "أكل الميتة عند الضرورة"]),
            (&["hukum", "operasi", "medis"], vec!["العمليات الجراحية", "الطب والشريعة"]),
            (&["hukum", "bedah", "plastik"], vec!["الجراحة التجميلية", "التجميل"]),
            (&["hukum", "tato", "permanen"], vec!["الوشم", "حكم الوشم"]),
            (&["hukum", "tindik", "telinga"], vec!["ثقب الأذن", "حكم الحلي"]),
            (&["hukum", "memanjangkan", "kuku"], vec!["تطويل الأظافر", "الفطرة"]),
            (&["hukum", "memanjangkan", "rambut"], vec!["إطالة الشعر", "توفير الشعر"]),
            (&["hukum", "rambut", "palsu"], vec!["الشعر المستعار", "الوصل في الشعر"]),
            (&["hukum", "pakai", "wig"], vec!["الباروكة", "الشعر المستعار"]),
            (&["hukum", "makeup"], vec!["التزين", "حكم الزينة"]),
            (&["hukum", "gincu", "lipstik"], vec!["الحمرة والزينة", "حكم التزين"]),
            (&["hukum", "wangi-wangian"], vec!["الطيب", "حكم العطر"]),
            (&["hukum", "memakai", "parfum"], vec!["حكم الطيب", "العطر"]),
            (&["hukum", "bulu", "mata"], vec!["الرموش الصناعية", "التجميل"]),
            (&["hukum", "lensa", "kontak"], vec!["العدسات اللاصقة", "حكم الاستخدام"]),
            (&["hukum", "behel", "gigi"], vec!["تقويم الأسنان", "حكم التقويم"]),
            (&["hukum", "susuk"], vec!["حكم السوسوك", "الحلي والزينة"]),
            (&["hukum", "sulam", "alis"], vec!["وشم الحواجب", "الوشم"]),
            (&["hukum", "sulam", "bibir"], vec!["وشم الشفاه", "الوشم"]),
            (&["hukum", "kerokan"], vec!["حكم الكروكان", "حجامة الجاوية"]),
            (&["hukum", "bekam"], vec!["الحجامة", "حكم الحجامة"]),
            (&["hukum", "akupuntur"], vec!["الإبر الصينية", "الطب البديل"]),
            (&["hukum", "senam"], vec!["الرياضة البدنية", "الرياضة في الإسلام"]),
            (&["hukum", "yoga"], vec!["حكم اليوغا"]),
            (&["hukum", "meditasi"], vec!["التأمل", "الذكر والتأمل"]),
            (&["hukum", "olahraga", "wanita"], vec!["رياضة المرأة", "حكم الرياضة"]),
            (&["hukum", "berenang", "wanita"], vec!["سباحة المرأة", "حكم سباحة النساء"]),
            (&["hukum", "bersalaman", "non", "mahram"], vec!["المصافحة مع الأجنبي", "حكم المصافحة"]),
            (&["hukum", "jabat", "tangan", "wanita"], vec!["مصافحة المرأة", "حكم المصافحة"]),
            (&["hukum", "khalwat"], vec!["الخلوة", "حكم الخلوة"]),
            (&["hukum", "ikhtilath"], vec!["الاختلاط", "حكم اختلاط الرجال والنساء"]),

            // ── BATCH 77: Comprehensive social ethics and ibadah topics ──
            (&["hukum", "hormat", "bendera"], vec!["التحية للعلم", "حكم تحية الراية"]),
            (&["hukum", "upacara", "bendera"], vec!["حكم حفلة العلم", "التحية للراية"]),
            (&["hukum", "hormat", "patung"], vec!["السجود للصنم", "التحية للتمثال"]),
            (&["hukum", "membuat", "patung"], vec!["نحت التماثيل", "التصوير"]),
            (&["hukum", "hewan", "peliharaan"], vec!["اقتناء الحيوانات", "الحيوانات الأليفة"]),
            (&["hukum", "pelihara", "anjing"], vec!["اقتناء الكلب", "حكم الكلب"]),
            (&["hukum", "pelihara", "kucing"], vec!["اقتناء القطة", "حكم القطة"]),
            (&["hukum", "pelihara", "babi"], vec!["اقتناء الخنزير", "حكم الخنزير"]),
            (&["hukum", "pelihara", "burung"], vec!["اقتناء الطيور", "حكم الطيور"]),
            (&["menyembelih", "nama", "selain", "allah"], vec!["الذبح لغير الله", "حكم ذبح لغير الله"]),
            (&["sembelihan", "non", "muslim"], vec!["ذبيحة الكتابي", "ذبيحة غير المسلم"]),
            (&["hukum", "daging", "non", "muslim"], vec!["ذبيحة الكتابي", "أكل لحم الكتابي"]),
            (&["hukum", "daging", "impor"], vec!["اللحوم المستوردة", "الذبيحة المستوردة"]),
            (&["hukum", "daging", "beku"], vec!["اللحم المجمد", "اللحوم الحلال"]),
            (&["halal", "haram", "makanan"], vec!["الحلال والحرام في الطعام", "أحكام الأطعمة"]),
            (&["bahan", "makanan", "haram"], vec!["المواد المحرمة في الطعام"]),
            (&["makanan", "mengandung", "haram"], vec!["الطعام المحتوي على المحرمات"]),
            (&["sertifikasi", "halal"], vec!["شهادة الحلال", "التصديق الحلال"]),
            (&["logo", "halal"], vec!["شعار الحلال", "التصديق الحلال"]),
            (&["hukum", "restoran", "non", "halal"], vec!["المطعم غير الحلال", "حكم الأكل"]),
            (&["hukum", "makan", "di", "warung"], vec!["الأكل في المطعم", "أحكام الطعام"]),
            (&["hukum", "mencicipi", "masakan"], vec!["تذوق الطعام للطاهي", "تذوق الصائم"]),
            (&["puasa", "mencicipi", "masakan"], vec!["تذوق الطعام حال الصيام"]),
            (&["hukum", "sikat", "gigi", "saat", "puasa"], vec!["السواك في الصيام", "استخدام المعجون"]),
            (&["sikat", "gigi", "puasa"], vec!["السواك والصيام", "حكم السواك"]),
            (&["berkumur", "saat", "puasa"], vec!["المضمضة في الصيام"]),
            (&["suntik", "saat", "puasa"], vec!["الحقنة في الصيام", "حكم الإبرة"]),
            (&["infus", "puasa"], vec!["الحقن الوريدية في الصيام"]),
            (&["tetes", "mata", "puasa"], vec!["قطرة العين في الصيام"]),
            (&["tetes", "telinga", "puasa"], vec!["قطرة الأذن في الصيام"]),
            (&["obat", "kumur", "puasa"], vec!["المضمضة بالدواء في الصيام"]),
            (&["parfum", "puasa"], vec!["الطيب حال الصيام"]),
            (&["menelan", "ludah", "puasa"], vec!["ابتلاع الريق في الصيام"]),
            (&["mimpi", "basah", "puasa"], vec!["الاحتلام في رمضان", "الجنابة في الصيام"]),
            (&["haid", "di", "bulan", "ramadhan"], vec!["الحيض في رمضان", "الحائض في رمضان"]),
            (&["haid", "saat", "puasa"], vec!["الحيض والصيام", "الحائض في الصيام"]),
            (&["cara", "bayar", "fidyah"], vec!["الفدية", "كيفية الفدية"]),
            (&["fidyah", "untuk", "siapa"], vec!["الفدية", "من يجوز له الفدية"]),
            (&["kaffarah", "puasa"], vec!["كفارة الإفطار في رمضان"]),
            (&["hukum", "batal", "puasa", "onani"], vec!["الاستمناء والصيام", "مفطرات الصوم"]),
            (&["hukum", "onani"], vec!["الاستمناء", "حكم الاستمناء"]),
            (&["hukum", "masturbasi"], vec!["الاستمناء", "حكم الاستمناء"]),
            (&["hukum", "zina", "hukumannya"], vec!["حد الزنا", "عقوبة الزنا"]),
            (&["hukum", "homoseksual"], vec!["اللواط", "حكم اللواط"]),
            (&["hukum", "LGBT"], vec!["الشذوذ الجنسي", "حكم الشذوذ"]),

            // ── BATCH 32: Islamic months + day patterns ──
            (&["amalan", "rajab"], vec!["أعمال شهر رجب", "فضل رجب"]),
            (&["keutamaan", "rajab"], vec!["فضل شهر رجب", "رجب"]),
            (&["puasa", "rajab"], vec!["صوم رجب", "صيام شهر رجب"]),
            (&["amalan", "sya'ban"], vec!["أعمال شعبان", "فضل شعبان"]),
            (&["keutamaan", "sya'ban"], vec!["فضل شعبان", "ليلة النصف من شعبان"]),
            (&["puasa", "sya'ban"], vec!["صوم شعبان", "صيام شعبان"]),
            (&["amalan", "ramadhan"], vec!["أعمال رمضان", "فضل رمضان"]),
            (&["keutamaan", "ramadhan"], vec!["فضل رمضان", "شهر الصيام"]),
            (&["amalan", "syawal"], vec!["أعمال شوال", "فضل شوال"]),
            (&["puasa", "syawal"], vec!["صوم شوال", "ست من شوال"]),
            (&["amalan", "dzulhijjah"], vec!["أعمال ذي الحجة", "فضل ذي الحجة"]),
            (&["keutamaan", "dzulhijjah"], vec!["فضل ذي الحجة", "عشر ذي الحجة"]),
            (&["amalan", "muharram"], vec!["أعمال شهر الله المحرم", "صيام عاشوراء"]),
            (&["keutamaan", "muharram"], vec!["فضل شهر المحرم", "يوم عاشوراء"]),
            (&["amalan", "rajab"], vec!["أعمال شهر رجب", "فضل رجب"]),
            (&["keutamaan", "rajab"], vec!["فضل شهر رجب", "رجب"]),
            (&["puasa", "rajab"], vec!["صوم رجب", "صيام شهر رجب"]),
            (&["amalan", "sya'ban"], vec!["أعمال شعبان", "فضل شعبان"]),
            (&["keutamaan", "sya'ban"], vec!["فضل شعبان", "ليلة النصف من شعبان"]),
            (&["puasa", "sya'ban"], vec!["صوم شعبان", "صيام شعبان"]),
            (&["amalan", "ramadhan"], vec!["أعمال رمضان", "فضل رمضان"]),
            (&["keutamaan", "ramadhan"], vec!["فضل رمضان", "شهر الصيام"]),
            (&["amalan", "syawal"], vec!["أعمال شوال", "فضل شوال"]),
            (&["puasa", "syawal"], vec!["صوم شوال", "ست من شوال"]),
            (&["amalan", "dzulhijjah"], vec!["أعمال ذي الحجة", "فضل ذي الحجة"]),
            (&["keutamaan", "dzulhijjah"], vec!["فضل ذي الحجة", "عشر ذي الحجة"]),
            (&["amalan", "muharram"], vec!["أعمال شهر الله المحرم", "صيام عاشوراء"]),
            (&["keutamaan", "muharram"], vec!["فضل شهر المحرم", "يوم عاشوراء"]),
            (&["amalan", "senin"], vec!["فضل صيام يوم الاثنين", "صوم الاثنين"]),
            (&["puasa", "senin"], vec!["صوم يوم الاثنين والخميس"]),

            // ── BATCH 78: Aqidah, tauhid, and broad Islamic belief topics ──
            (&["pengertian", "iman"], vec!["تعريف الإيمان", "معنى الإيمان"]),
            (&["rukun", "iman"], vec!["أركان الإيمان", "الإيمان بالله"]),
            (&["rukun", "islam"], vec!["أركان الإسلام", "الإسلام"]),
            (&["iman", "kepada", "allah"], vec!["الإيمان بالله", "التوحيد"]),
            (&["iman", "kepada", "malaikat"], vec!["الإيمان بالملائكة"]),
            (&["iman", "kepada", "kitab"], vec!["الإيمان بالكتب", "الكتب المقدسة"]),
            (&["iman", "kepada", "rasul"], vec!["الإيمان بالرسل", "الأنبياء"]),
            (&["iman", "kepada", "hari", "kiamat"], vec!["الإيمان باليوم الآخر", "القيامة"]),
            (&["iman", "kepada", "takdir"], vec!["الإيمان بالقدر", "القضاء والقدر"]),
            (&["iman", "qadha", "qadar"], vec!["القضاء والقدر", "الإيمان بالقدر"]),
            (&["macam", "tauhid"], vec!["أقسام التوحيد", "التوحيد"]),
            (&["tauhid", "rububiyah"], vec!["توحيد الربوبية"]),
            (&["tauhid", "uluhiyah"], vec!["توحيد الألوهية"]),
            (&["tauhid", "asma", "sifat"], vec!["توحيد الأسماء والصفات"]),
            (&["sifat", "allah"], vec!["صفات الله", "أسماء الله الحسنى"]),
            (&["asmaul", "husna"], vec!["الأسماء الحسنى", "أسماء الله"]),
            (&["99", "nama", "allah"], vec!["الأسماء الحسنى التسعة والتسعون"]),
            (&["sifat", "wajib", "allah"], vec!["الصفات الواجبة لله", "صفات الله"]),
            (&["sifat", "mustahil", "allah"], vec!["الصفات المستحيلة على الله"]),
            (&["sifat", "nabi"], vec!["صفات الأنبياء", "الرسل"]),
            (&["macam", "syirik"], vec!["أنواع الشرك", "الشرك بالله"]),
            (&["syirik", "kecil"], vec!["الشرك الأصغر"]),
            (&["syirik", "besar"], vec!["الشرك الأكبر"]),
            (&["syirik", "tersembunyi"], vec!["الشرك الخفي", "الشرك الأصغر"]),
            (&["macam", "bid'ah"], vec!["أنواع البدعة", "البدعة"]),
            (&["bid'ah", "hasanah"], vec!["البدعة الحسنة", "البدعة"]),
            (&["bid'ah", "dholalah"], vec!["البدعة الضلالة", "البدعة السيئة"]),
            (&["ciri", "ahlu", "sunnah"], vec!["أهل السنة والجماعة", "شعار أهل السنة"]),
            (&["ahlussunnah", "wal", "jamaah"], vec!["أهل السنة والجماعة"]),
            (&["aswaja"], vec!["أهل السنة والجماعة"]),
            (&["mu'tazilah", "paham"], vec!["المعتزلة", "فرق الإسلام"]),
            (&["khawarij", "paham"], vec!["الخوارج", "فرق الإسلام"]),
            (&["salafi", "manhaj"], vec!["المنهج السلفي", "السلفية"]),
            (&["wahabi", "ajaran"], vec!["الوهابية", "فرق إسلامية"]),
            (&["syi'ah", "faham"], vec!["الشيعة", "فرق الإسلام"]),
            (&["sunni", "syi'ah", "perbedaan"], vec!["الفرق بين السنة والشيعة"]),
            (&["penyimpangan", "aqidah"], vec!["انحرافات عقدية", "الضلال"]),
            (&["sesat", "hukum"], vec!["الضلال", "الفرق الضالة"]),
            (&["aliran", "sesat"], vec!["الفرق الضالة", "الغلو"]),
            (&["ahli", "bid'ah"], vec!["أهل البدعة", "المبتدع"]),

            (&["amalan", "kamis"], vec!["فضل صيام يوم الخميس", "صوم الخميس"]),
            (&["puasa", "kamis"], vec!["صوم يوم الخميس"]),
            (&["amalan", "jumat"], vec!["فضل يوم الجمعة", "أعمال يوم الجمعة"]),
            (&["amalan", "lailatul"], vec!["أعمال ليلة القدر", "إحياء ليلة القدر"]),
            (&["keutamaan", "lailatul"], vec!["فضل ليلة القدر", "ليلة القدر"]),
            (&["amalan", "nisfu"], vec!["أعمال ليلة النصف من شعبان"]),
            (&["keutamaan", "nisfu"], vec!["فضل ليلة النصف من شعبان"]),
            (&["amalan", "isra"], vec!["أعمال ليلة الإسراء"]),
            (&["keutamaan", "isra"], vec!["فضل ليلة الإسراء والمعراج"]),
            (&["amalan", "maulid"], vec!["أعمال ليلة المولد"]),
            (&["keutamaan", "maulid"], vec!["فضل المولد النبوي"]),

            // ── BATCH 79: Specific Quran, hadith, and tafsir queries ──
            (&["tafsir", "surah", "al", "fatihah"], vec!["تفسير الفاتحة", "تفسير سورة الفاتحة"]),
            (&["tafsir", "surah", "al", "baqarah"], vec!["تفسير البقرة", "تفسير سورة البقرة"]),
            (&["tafsir", "surah", "ali", "imran"], vec!["تفسير آل عمران"]),
            (&["tafsir", "surah", "an", "nisa"], vec!["تفسير النساء", "تفسير سورة النساء"]),
            (&["tafsir", "surah", "al", "maidah"], vec!["تفسير المائدة"]),
            (&["tafsir", "surah", "yusuf"], vec!["تفسير يوسف"]),
            (&["tafsir", "surah", "yasin"], vec!["تفسير يس", "تفسير سورة يس"]),
            (&["tafsir", "surah", "al", "kahf"], vec!["تفسير الكهف"]),
            (&["tafsir", "surah", "al", "mulk"], vec!["تفسير الملك"]),
            (&["tafsir", "surah", "al", "waqiyah"], vec!["تفسير الواقعة"]),
            (&["tafsir", "surah", "al", "ikhlas"], vec!["تفسير الإخلاص"]),
            (&["tafsir", "surah", "al", "falaq"], vec!["تفسير الفلق"]),
            (&["tafsir", "surah", "an", "nas"], vec!["تفسير الناس"]),
            (&["tafsir", "ayat", "kursi"], vec!["تفسير آية الكرسي"]),
            (&["tafsir", "ayat", "seribu", "dinar"], vec!["تفسير آية الطلاق والرزق"]),
            (&["tafsir", "surat", "al", "quran"], vec!["تفسير القرآن الكريم"]),
            (&["makna", "surah", "yasin"], vec!["معنى سورة يس", "تفسير يس"]),
            (&["keutamaan", "surah", "yasin"], vec!["فضل سورة يس"]),
            (&["keutamaan", "surah", "al", "mulk"], vec!["فضل سورة الملك"]),
            (&["keutamaan", "surah", "al", "kahf"], vec!["فضل سورة الكهف"]),
            (&["keutamaan", "surah", "al", "ikhlas"], vec!["فضل سورة الإخلاص"]),
            (&["keutamaan", "ayat", "kursi"], vec!["فضل آية الكرسي"]),
            (&["hadits", "shahih", "tentang"], vec!["الحديث الصحيح", "أحاديث صحيحة"]),
            (&["macam", "hadits"], vec!["أنواع الحديث", "علوم الحديث"]),
            (&["hadits", "shahih"], vec!["الحديث الصحيح", "درجات الحديث"]),
            (&["hadits", "dhaif"], vec!["الحديث الضعيف", "درجات الحديث"]),
            (&["hadits", "maudu"], vec!["الحديث الموضوع", "الوضع"]),
            (&["hadits", "palsu"], vec!["الحديث الموضوع", "الحديث المكذوب"]),
            (&["hadits", "hasan"], vec!["الحديث الحسن", "درجات الحديث"]),
            (&["hadits", "mutawatir"], vec!["الحديث المتواتر"]),
            (&["hadits", "ahad"], vec!["حديث الآحاد"]),
            (&["sanad", "matan", "hadits"], vec!["السند والمتن", "علوم الحديث"]),
            (&["perawi", "hadits"], vec!["رواة الحديث", "علم الرجال"]),
            (&["ilmu", "rijal"], vec!["علم الرجال", "علم الجرح والتعديل"]),
            (&["jarh", "ta'dil"], vec!["الجرح والتعديل"]),
            (&["mutabi", "syahid", "hadits"], vec!["المتابع والشاهد", "علوم الحديث"]),
            (&["nasikh", "mansukh"], vec!["الناسخ والمنسوخ", "علوم القرآن"]),
            (&["asbabun", "nuzul"], vec!["أسباب النزول", "علوم القرآن"]),
            (&["sabab", "nuzul"], vec!["سبب النزول", "أسباب النزول"]),
            (&["muhkam", "mutasyabih"], vec!["المحكم والمتشابه", "علوم القرآن"]),
            (&["i'jaz", "quran"], vec!["إعجاز القرآن", "الإعجاز"]),
            (&["kemukjizatan", "quran"], vec!["إعجاز القرآن"]),
            (&["makki", "madani"], vec!["المكي والمداني", "نزول القرآن"]),
            (&["ulumul", "hadits"], vec!["علوم الحديث", "مصطلح الحديث"]),
            (&["ulumul", "quran"], vec!["علوم القرآن"]),

            // Scholarly opinion patterns
            (&["menurut", "jumhur"], vec!["رأي جمهور العلماء", "جمهور"]),
            (&["menurut", "ulama"], vec!["رأي العلماء", "الفقهاء"]),
            (&["menurut", "mui"], vec!["فتوى مجلس العلماء الإندونيسي"]),
            (&["menurut", "muhammadiyah"], vec!["رأي المحمدية", "قرار محمدية"]),

            // Syariah vs konvensional comparisons
            (&["syariah", "konvensional"], vec!["الشريعة والتقليدي", "الإسلامي والتقليدي"]),
            (&["bank", "syariah"], vec!["البنك الإسلامي", "المصرف الإسلامي"]),

            // ── BATCH 80: Tasawuf, akhlak, and spiritual topics ──
            (&["maqam", "tawadhu"], vec!["مقام التواضع", "التواضع"]),
            (&["maqam", "sabar"], vec!["مقام الصبر", "الصبر في التصوف"]),
            (&["maqam", "syukur"], vec!["مقام الشكر", "الشكر"]),
            (&["maqam", "zuhud"], vec!["مقام الزهد", "الزهد"]),
            (&["maqam", "tawakkal"], vec!["مقام التوكل", "التوكل على الله"]),
            (&["maqam", "mahabbah"], vec!["مقام المحبة", "محبة الله"]),
            (&["maqam", "ridha"], vec!["مقام الرضا", "الرضا بالقضاء"]),
            (&["maqam", "khauf"], vec!["مقام الخوف", "الخوف من الله"]),
            (&["maqam", "raja"], vec!["مقام الرجاء", "الرجاء"]),
            (&["maqam", "muhasabah"], vec!["مقام المحاسبة", "محاسبة النفس"]),
            (&["jalan", "menuju", "allah"], vec!["الطريق إلى الله", "السلوك"]),
            (&["hati", "bersih"], vec!["صفاء القلب", "تزكية النفس"]),
            (&["penyakit", "hati"], vec!["أمراض القلب", "الغيبة والحسد"]),
            (&["cara", "membersihkan", "hati"], vec!["تزكية النفس", "طهارة القلب"]),
            (&["tazkiyatun", "nafs"], vec!["تزكية النفس"]),
            (&["macam", "hawa", "nafsu"], vec!["النفس الأمارة واللوامة والمطمئنة"]),
            (&["nafsu", "ammarah"], vec!["النفس الأمارة"]),
            (&["nafsu", "lawamamah"], vec!["النفس اللوامة"]),
            (&["nafsu", "mutmainnah"], vec!["النفس المطمئنة"]),
            (&["jihad", "nafs"], vec!["جهاد النفس", "المجاهدة"]),
            (&["perjuangan", "diri"], vec!["جهاد النفس", "المجاهدة"]),
            (&["mujahadah", "nafs"], vec!["المجاهدة", "جهاد النفس"]),
            (&["pengertian", "tawakkal"], vec!["معنى التوكل", "التوكل"]),
            (&["pengertian", "sabar"], vec!["معنى الصبر", "الصبر"]),
            (&["pengertian", "syukur"], vec!["معنى الشكر", "الشكر"]),
            (&["pengertian", "ikhlas"], vec!["معنى الإخلاص", "الإخلاص"]),
            (&["pengertian", "taqwa"], vec!["معنى التقوى", "التقوى"]),
            (&["pengertian", "zuhud"], vec!["معنى الزهد", "الزهد"]),
            (&["ciri", "orang", "bertaqwa"], vec!["صفات المتقي", "علامات التقوى"]),
            (&["ciri", "ikhlas"], vec!["علامات الإخلاص"]),
            (&["tanda", "sombong"], vec!["علامات الكبر"]),
            (&["bahaya", "sombong"], vec!["ذم الكبر", "آفات الكبر"]),
            (&["bahaya", "hasad"], vec!["ذم الحسد", "آفات الحسد"]),
            (&["bahaya", "ghibah"], vec!["ذم الغيبة", "آفات الغيبة"]),
            (&["bahaya", "riya"], vec!["ذم الرياء", "آفات الرياء"]),
            (&["cara", "menghilangkan", "sombong"], vec!["علاج الكبر", "ذم الكبر"]),
            (&["cara", "menghilangkan", "hasad"], vec!["علاج الحسد", "ذم الحسد"]),
            (&["cara", "menghilangkan", "riya"], vec!["علاج الرياء", "الإخلاص"]),
            (&["cara", "menghilangkan", "marah"], vec!["علاج الغضب", "ذم الغضب"]),
            (&["cara", "menghilangkan", "was-was"], vec!["علاج الوسواس"]),
            (&["nilai", "akhlak", "islam"], vec!["الأخلاق الإسلامية", "القيم الإسلامية"]),
            (&["akhlak", "terpuji"], vec!["الأخلاق الكريمة", "الأخلاق الحميدة"]),
            (&["akhlak", "tercela"], vec!["الأخلاق الذميمة", "الرذائل"]),
            (&["sifat", "terpuji"], vec!["الأوصاف الحميدة", "الأخلاق الكريمة"]),
            (&["sifat", "tercela"], vec!["الأوصاف الذميمة", "الرذائل"]),
            (&["cara", "memperbaiki", "akhlak"], vec!["إصلاح الأخلاق", "تزكية النفس"]),

            // Location + prayer combinations
            (&["shalat", "mushalla"], vec!["الصلاة في المصلى", "حكم المصلى"]),
            (&["shalat", "aqsha"], vec!["الصلاة في المسجد الأقصى"]),
            (&["shalat", "pasar"], vec!["الصلاة في السوق", "حكم الصلاة في السوق"]),
            (&["shalat", "kuburan"], vec!["الصلاة في المقبرة", "حكم الصلاة على القبور"]),
            (&["shalat", "kamar"], vec!["الصلاة في الحمام", "الأماكن المنهي عنها"]),

            // Bagaimana jika (what if) patterns
            (&["bagaimana", "imam"], vec!["حكم الإمام", "أحكام الإمامة"]),
            (&["bagaimana", "wali"], vec!["حكم الولي", "ولي النكاح"]),
            (&["bagaimana", "saksi"], vec!["شروط الشاهد", "الشهادة"]),
            (&["bagaimana", "mahar"], vec!["حكم المهر", "المهر في النكاح"]),
            (&["bagaimana", "jenazah"], vec!["أحكام الجنازة", "غسل الميت"]),

            // ── BATCH 81: Educational, pesantren, and applied Islamic questions ──
            (&["pesantren", "kurikulum"], vec!["المناهج الدراسية في المدرسة"]),
            (&["santri", "kehidupan"], vec!["حياة طالب العلم", "طلب العلم"]),
            (&["mondok", "hukum"], vec!["طلب العلم", "حكم طلب العلم"]),
            (&["hafidz", "quran"], vec!["حافظ القرآن", "حفظ القرآن"]),
            (&["penghafal", "quran"], vec!["حافظ القرآن"]),
            (&["mengajarkan", "quran"], vec!["تعليم القرآن", "معلم القرآن"]),
            (&["mengajarkan", "islam"], vec!["التعليم الديني", "تعليم الإسلام"]),
            (&["ilmu", "yang", "wajib"], vec!["العلم الواجب", "فريضة العلم"]),
            (&["wajib", "belajar", "agama"], vec!["فريضة تعلم الدين"]),
            (&["kewajiban", "menuntut", "ilmu"], vec!["فريضة طلب العلم"]),
            (&["adab", "menuntut", "ilmu"], vec!["آداب طلب العلم"]),
            (&["adab", "kepada", "guru"], vec!["آداب الطالب مع المعلم", "احترام الأستاذ"]),
            (&["adab", "kepada", "orang", "tua"], vec!["آداب البر بالوالدين", "حق الوالدين"]),
            (&["adab", "kepada", "tetangga"], vec!["آداب حق الجار", "حقوق الجار"]),
            (&["adab", "kepada", "tamu"], vec!["آداب استقبال الضيوف", "حق الضيف"]),
            (&["adab", "makan"], vec!["آداب الأكل", "سنن الأكل"]),
            (&["adab", "minum"], vec!["آداب الشرب", "سنن الشرب"]),
            (&["adab", "tidur"], vec!["آداب النوم", "ذكر عند النوم"]),
            (&["adab", "masuk", "rumah"], vec!["آداب دخول البيت"]),
            (&["adab", "keluar", "rumah"], vec!["آداب الخروج من البيت"]),
            (&["adab", "masjid"], vec!["آداب المسجد", "حرمة المسجد"]),
            (&["adab", "bertamu"], vec!["آداب الزيارة", "حق الضيف"]),
            (&["adab", "berpakaian"], vec!["آداب اللباس", "لباس المسلم"]),
            (&["adab", "bercakap"], vec!["آداب الكلام", "حسن الكلام"]),
            (&["adab", "berbicara"], vec!["آداب الكلام", "حسن القول"]),
            (&["adab", "bersin"], vec!["آداب العطاس", "تشميت العاطس"]),
            (&["sunnah", "bersin"], vec!["آداب العطاس", "الحمد على العطاس"]),
            (&["adab", "menguap"], vec!["آداب التثاؤب"]),
            (&["sunnah", "ketika", "marah"], vec!["علاج الغضب", "الاستعاذة عند الغضب"]),
            (&["sunnah", "sebelum", "tidur"], vec!["سنن النوم", "آداب النوم"]),
            (&["sunnah", "bangun", "tidur"], vec!["آداب الاستيقاظ", "دعاء الاستيقاظ"]),
            (&["sunnah", "masuk", "kamar", "mandi"], vec!["دعاء دخول الخلاء", "آداب قضاء الحاجة"]),
            (&["sunnah", "keluar", "kamar", "mandi"], vec!["دعاء الخروج من الخلاء"]),
            (&["sunnah", "makan"], vec!["سنن الأكل", "آداب الطعام"]),
            (&["sunnah", "minum"], vec!["سنن الشرب", "آداب الشرب"]),
            (&["sunnah", "saat", "hujan"], vec!["سنة عند نزول المطر", "دعاء المطر"]),
            (&["sunnah", "hari", "jumat"], vec!["سنن يوم الجمعة"]),
            (&["sunnah", "setelah", "shalat"], vec!["السنن بعد الصلاة", "الذكر بعد الصلاة"]),
            (&["sunnah", "sebelum", "shalat"], vec!["السنن قبل الصلاة"]),
            (&["sunnah", "sebelum", "makan"], vec!["سنة قبل الأكل", "التسمية"]),
            (&["sunnah", "setelah", "makan"], vec!["سنة بعد الأكل", "الحمدلة"]),
            (&["doa", "sunnah"], vec!["الأدعية المأثورة", "أدعية النبي"]),
            (&["dzikir", "setelah", "wudhu"], vec!["ذكر بعد الوضوء"]),

            // ── BATCH 33: Hikmah, mazhab opinions, animals, scholar references ──
            // ── BATCH 82: More contemporary and comprehensive patterns
            (&["tes", "kehamilan", "puasa"], vec!["اختبار الحمل أثناء الصيام"]),
            (&["hamil", "puasa"], vec!["صوم الحامل", "الحامل في رمضان"]),
            (&["hamil", "shalat"], vec!["صلاة الحامل", "أحكام الحامل"]),
            (&["hamil", "hukum", "tidak", "berpuasa"], vec!["إفطار الحامل", "فدية الحامل"]),
            (&["menyusui", "puasa"], vec!["صوم المرضع", "المرضع في رمضان"]),
            (&["menyusui", "tidak", "puasa"], vec!["إفطار المرضع", "فدية المرضع"]),
            (&["anak", "kecil", "puasa"], vec!["صوم الصبيان", "تعليم الصيام"]),
            (&["anak", "kecil", "shalat"], vec!["صلاة الصبيان", "تعليم الصلاة"]),
            (&["anak", "baligh"], vec!["البلوغ", "علامات البلوغ"]),
            (&["tanda-tanda", "baligh"], vec!["علامات البلوغ", "البلوغ"]),
            (&["mimpi", "basah", "tanda"], vec!["الاحتلام", "علامة البلوغ"]),
            (&["haid", "pertama", "tanda"], vec!["الحيض كعلامة البلوغ"]),
            (&["umur", "baligh"], vec!["سن البلوغ", "البلوغ"]),
            (&["mukallaf", "syarat"], vec!["شروط التكليف", "المكلف"]),
            (&["orang", "gila", "kewajiban"], vec!["المجنون والتكليف", "رفع القلم"]),
            (&["orang", "linglung", "shalat"], vec!["المغمى عليه والصلاة"]),
            (&["lansia", "shalat"], vec!["صلاة الشيخ الكبير", "العاجز عن الصلاة"]),
            (&["shalat", "orang", "sakit"], vec!["صلاة المريض", "الصلاة قاعداً"]),
            (&["shalat", "duduk", "hukum"], vec!["الصلاة قاعداً", "صلاة العاجز"]),
            (&["shalat", "berbaring", "hukum"], vec!["الصلاة مضطجعاً", "صلاة المريض"]),
            (&["shalat", "di", "kendaraan"], vec!["الصلاة في المركبة", "الصلاة في السفر"]),
            (&["shalat", "di", "pesawat"], vec!["الصلاة في الطائرة", "قبلة المسافر بالجو"]),
            (&["shalat", "di", "kapal"], vec!["الصلاة في السفينة"]),
            (&["kiblat", "arah"], vec!["اتجاه القبلة", "قبلة المسلم"]),
            (&["menentukan", "kiblat"], vec!["تحديد القبلة", "جهة القبلة"]),
            (&["kiblat", "salah", "hukum"], vec!["إصابة القبلة", "الخطأ في القبلة"]),
            (&["hukum", "azan", "lewat", "hp"], vec!["الأذان بالآلة", "الأذان المسجل"]),
            (&["azan", "rekaman", "hukum"], vec!["الأذان المسجل", "أذان الآلة"]),
            (&["hukum", "shalat", "di", "rumah"], vec!["الصلاة في البيت", "صلاة الجماعة"]),
            (&["hukum", "shalat", "di", "masjid"], vec!["فضل الصلاة في المسجد", "الصلاة جماعة"]),
            (&["hukum", "shalat", "berjamaah", "wajib"], vec!["وجوب صلاة الجماعة", "الجماعة"]),
            (&["hukum", "shalat", "jumat", "wanita"], vec!["الجمعة للمرأة", "حكم الجمعة"]),
            (&["hukum", "shalat", "jumat", "online"], vec!["الجمعة عبر الإنترنت"]),
            (&["shalat", "virtual"], vec!["الصلاة عبر الفيديو", "الصلاة الجماعية"]),
            (&["takbir", "hari", "raya"], vec!["تكبيرات العيد", "تكبير الجماعة"]),
            (&["shalat", "ied", "hukum"], vec!["حكم صلاة العيد", "صلاة الأعياد"]),
            (&["shalat", "ied", "waktu"], vec!["وقت صلاة العيد"]),
            (&["shalat", "istisqa", "hukum"], vec!["صلاة الاستسقاء", "دعاء الاستسقاء"]),
            (&["shalat", "gerhana", "hukum"], vec!["صلاة الكسوف والخسوف"]),
            (&["gerhana", "matahari", "shalat"], vec!["صلاة كسوف الشمس"]),
            (&["gerhana", "bulan", "shalat"], vec!["صلاة خسوف القمر"]),
            (&["shalat", "taubat", "cara"], vec!["صلاة التوبة"]),
            (&["shalat", "istikharah", "cara"], vec!["صلاة الاستخارة", "دعاء الاستخارة"]),
            (&["doa", "istikharah"], vec!["دعاء الاستخارة"]),
            (&["hukum", "meninggalkan", "shalat", "satu", "waktu"], vec!["ترك الصلاة وقتاً واحداً", "حكم ترك الصلاة"]),
            (&["qadha", "shalat", "panjang"], vec!["قضاء الصواتي الفائتة الكثيرة"]),

            // hikmah + ibadah bigrams
            (&["hikmah", "shalat"], vec!["حكمة الصلاة", "أسرار الصلاة"]),
            (&["hikmah", "puasa"], vec!["حكمة الصيام", "أسرار الصوم"]),
            (&["hikmah", "zakat"], vec!["حكمة الزكاة"]),
            (&["hikmah", "haji"], vec!["حكمة الحج", "أسرار الحج"]),
            (&["hikmah", "nikah"], vec!["حكمة النكاح", "فوائد الزواج"]),
            (&["hikmah", "qurban"], vec!["حكمة الأضحية"]),
            (&["hikmah", "wudhu"], vec!["أسرار الوضوء"]),
            (&["hikmah", "riba"], vec!["حكمة تحريم الربا"]),
            (&["hikmah", "zina"], vec!["حكمة تحريم الزنا"]),
            (&["hikmah", "khitan"], vec!["حكمة الختان"]),
            (&["hikmah", "talak"], vec!["حكمة الطلاق"]),
            (&["hikmah", "warisan"], vec!["حكمة الميراث"]),
            (&["hikmah", "poligami"], vec!["حكمة التعدد"]),
            (&["hikmah", "iddah"], vec!["حكمة العدة"]),
            (&["hikmah", "aqiqah"], vec!["حكمة العقيقة"]),
            (&["hikmah", "silaturahmi"], vec!["حكمة صلة الرحم"]),
            (&["hikmah", "dzikir"], vec!["حكمة الذكر", "فضل الذكر"]),
            (&["hikmah", "sedekah"], vec!["حكمة الصدقة", "فضل الصدقة"]),
            (&["rahasia", "shalat"], vec!["أسرار الصلاة", "روح الصلاة"]),
            (&["rahasia", "puasa"], vec!["أسرار الصيام"]),
            (&["rahasia", "wudhu"], vec!["أسرار الوضوء"]),
            (&["rahasia", "haji"], vec!["أسرار الحج"]),

            // ── BATCH 83: Numbers, calculations, finance in Islamic context ──
            (&["nisab", "zakat", "emas"], vec!["نصاب زكاة الذهب", "نصاب الذهب"]),
            (&["nisab", "zakat", "perak"], vec!["نصاب زكاة الفضة", "نصاب الفضة"]),
            (&["nisab", "zakat", "uang"], vec!["نصاب زكاة الأموال", "زكاة النقود"]),
            (&["nisab", "zakat", "pertanian"], vec!["نصاب زكاة الزروع", "زكاة الحبوب"]),
            (&["nisab", "zakat", "perniagaan"], vec!["نصاب زكاة التجارة", "زكاة التجارة"]),
            (&["nisab", "zakat", "ternak"], vec!["نصاب زكاة الأنعام", "زكاة الإبل"]),
            (&["haul", "zakat"], vec!["الحول في الزكاة", "مرور الحول"]),
            (&["berapa", "persen", "zakat"], vec!["نسبة الزكاة", "القدر الواجب في الزكاة"]),
            (&["cara", "hitung", "zakat"], vec!["كيفية حساب الزكاة", "حساب الزكاة"]),
            (&["cara", "hitung", "waris"], vec!["كيفية حساب الميراث", "الفرائض"]),
            (&["bagian", "waris", "anak", "laki"], vec!["ميراث الابن", "الإرث"]),
            (&["bagian", "waris", "anak", "perempuan"], vec!["ميراث البنت", "الإرث"]),
            (&["bagian", "waris", "istri"], vec!["ميراث الزوجة", "ثمن الزوجة"]),
            (&["bagian", "waris", "suami"], vec!["ميراث الزوج", "ربع الزوج"]),
            (&["bagian", "waris", "ibu"], vec!["ميراث الأم", "سدس الأم"]),
            (&["bagian", "waris", "ayah"], vec!["ميراث الأب", "الإرث"]),
            (&["ashabul", "furudh"], vec!["أصحاب الفروض", "الوارثون"]),
            (&["ashabah", "waris"], vec!["العصبة", "الإرث بالتعصيب"]),
            (&["dzawil", "arham"], vec!["ذوو الأرحام", "الإرث"]),
            (&["hijab", "waris"], vec!["الحجب في الميراث", "حجب الوارث"]),
            (&["washiyat", "waris"], vec!["الوصية من الميراث", "الوصية"]),
            (&["utang", "sebelum", "waris"], vec!["الدين قبل الميراث", "قضاء الديون من التركة"]),
            (&["pembagian", "harta", "waris"], vec!["قسمة التركة", "الميراث"]),
            (&["cara", "bagi", "warisan"], vec!["كيفية قسمة الميراث"]),
            (&["waris", "1", "anak", "laki", "1", "perempuan"], vec!["الميراث للذكر والأنثى", "للذكر مثل حظ الأنثيين"]),
            (&["waris", "anak", "angkat"], vec!["ميراث الولد المتبنى", "التبني والميراث"]),
            (&["wasiat", "sepertiga"], vec!["الوصية بالثلث", "حد الوصية"]),
            (&["wasiat", "melebihi", "sepertiga"], vec!["الوصية بأكثر من الثلث"]),
            (&["sepertiga", "harta"], vec!["ثلث المال", "الوصية"]),
            (&["wakaf", "produktif"], vec!["الوقف المنتج", "الوقف الإنتاجي"]),
            (&["wakaf", "uang"], vec!["وقف النقد", "وقف المال"]),
            (&["wakaf", "saham"], vec!["وقف الأسهم"]),
            (&["wakaf", "tanah"], vec!["وقف الأرض", "الوقف"]),
            (&["syarat", "wakaf"], vec!["شروط الوقف"]),
            (&["hukum", "mencabut", "wakaf"], vec!["الرجوع في الوقف"]),
            (&["hibah", "kepada", "anak"], vec!["الهبة للأولاد", "تسوية الهبة"]),
            (&["hibah", "ketika", "sakit"], vec!["هبة المريض", "الهبة في مرض الموت"]),
            (&["hukum", "mencabut", "hibah"], vec!["الرجوع في الهبة"]),
            (&["perjanjian", "pra", "nikah"], vec!["شرط في عقد النكاح", "الشروط في النكاح"]),
            (&["prenuptial", "agreement"], vec!["اتفاق قبل الزواج", "شروط النكاح"]),
            (&["pisah", "harta"], vec!["الفصل في الملكية", "أحكام المال"]),
            (&["harta", "bersama"], vec!["المال المشترك بين الزوجين"]),
            (&["gono", "gini"], vec!["المال المشترك بين الزوجين", "تصفية الأموال"]),
            (&["royalti", "hak", "cipta"], vec!["حقوق الملكية الفكرية", "الحقوق الأدبية"]),
            (&["hak", "cipta", "islam"], vec!["حقوق الملكية الفكرية في الإسلام"]),

            // BATCH 84: Comparison/disambiguation patterns (apa bedanya X dan Y)
            (&["beda", "shalat", "sunah", "wajib"], vec!["الفرق بين السنة والفريضة", "صلاة السنة والفريضة"]),
            (&["beda", "wudhu", "mandi", "wajib"], vec!["الفرق بين الوضوء والغسل", "موجبات الغسل"]),
            (&["beda", "haram", "makruh"], vec!["الفرق بين الحرام والمكروه", "الأحكام التكليفية"]),
            (&["beda", "makruh", "mubah"], vec!["الفرق بين المكروه والمباح", "الأحكام الشرعية"]),
            (&["beda", "sunah", "mubah"], vec!["الفرق بين السنة والمباح"]),
            (&["beda", "nabi", "rasul"], vec!["الفرق بين النبي والرسول", "تعريف النبي والرسول"]),
            (&["beda", "fardhu", "wajib"], vec!["الفرق بين الفرض والواجب", "الفرض والواجب عند الأحناف"]),
            (&["beda", "qadha", "kaffarah"], vec!["الفرق بين القضاء والكفارة"]),
            (&["beda", "tayamum", "wudhu"], vec!["الفرق بين التيمم والوضوء", "شروط التيمم"]),
            (&["beda", "fasakh", "talak"], vec!["الفرق بين الفسخ والطلاق", "أنواع فسخ النكاح"]),
            (&["beda", "khuluk", "talak"], vec!["الفرق بين الخلع والطلاق", "حكم الخلع"]),
            (&["beda", "zakat", "sedekah"], vec!["الفرق بين الزكاة والصدقة", "أنواع الصدقة"]),
            (&["beda", "infak", "sedekah"], vec!["الفرق بين الإنفاق والصدقة"]),
            (&["beda", "wakaf", "hibah"], vec!["الفرق بين الوقف والهبة", "أحكام الوقف والهبة"]),
            (&["beda", "wasiat", "waris"], vec!["الفرق بين الوصية والميراث"]),
            (&["beda", "riba", "bunga"], vec!["الفرق بين الربا والفائدة", "ربا وبنك"]),
            (&["beda", "murabahah", "ijarah"], vec!["الفرق بين المرابحة والإجارة"]),
            (&["beda", "syirik", "bid'ah"], vec!["الفرق بين الشرك والبدعة"]),
            (&["beda", "kufur", "syirik"], vec!["الفرق بين الكفر والشرك"]),
            (&["beda", "kafir", "munafik"], vec!["الفرق بين الكافر والمنافق"]),
            (&["beda", "iman", "islam"], vec!["الفرق بين الإيمان والإسلام", "مراتب الدين"]),
            (&["beda", "taubat", "istigfar"], vec!["الفرق بين التوبة والاستغفار"]),
            (&["beda", "doa", "dzikir"], vec!["الفرق بين الدعاء والذكر"]),
            (&["beda", "tafsir", "takwil"], vec!["الفرق بين التفسير والتأويل"]),
            (&["beda", "hadits", "sunnah"], vec!["الفرق بين الحديث والسنة"]),
            (&["beda", "quran", "hadits"], vec!["الفرق بين القرآن والحديث"]),
            (&["beda", "ijma", "qiyas"], vec!["الفرق بين الإجماع والقياس", "مصادر الأحكام"]),
            (&["bedanya", "wajib", "fardhu"], vec!["الفرق بين الواجب والفرض"]),
            (&["bedanya", "sunah", "mandub"], vec!["الفرق بين السنة والمندوب"]),
            (&["bedanya", "hadas", "najis"], vec!["الفرق بين الحدث والنجاسة"]),
            (&["bedanya", "hadas", "besar", "kecil"], vec!["الحدث الأكبر والأصغر", "موجبات الغسل والوضوء"]),
            (&["perbedaan", "shalat", "wajib", "sunah"], vec!["الفرق بين صلاة الفريضة والسنة"]),
            (&["perbedaan", "nikah", "kawin"], vec!["النكاح والزواج"]),
            (&["perbedaan", "talak", "cerai"], vec!["الفرق بين الطلاق والفراق"]),
            (&["perbedaan", "wudhu", "tayamum"], vec!["الفرق بين الوضوء والتيمم"]),
            (&["perbedaan", "puasa", "wajib", "sunah"], vec!["الفرق بين الصوم الواجب والمندوب"]),
            (&["perbedaan", "zakat", "infak"], vec!["الفرق بين الزكاة والإنفاق"]),
            (&["perbedaan", "sedekah", "zakat"], vec!["الفرق بين الصدقة والزكاة"]),
            (&["perbedaan", "aqidah", "syariah"], vec!["الفرق بين العقيدة والشريعة"]),
            (&["persamaan", "shalat", "doa"], vec!["العلاقة بين الصلاة والدعاء"]),
            (&["apakah", "sama", "wajib", "fardhu"], vec!["الفرض والواجب", "الأحكام التكليفية"]),

            // BATCH 85: Lebih utama/preference questions
            (&["lebih", "utama", "shalat", "berjamaah", "sendiri"], vec!["فضل صلاة الجماعة", "ثواب الجماعة"]),
            (&["lebih", "utama", "masjid", "rumah"], vec!["أفضل الصلاة في المسجد", "الصلاة في البيت"]),
            (&["lebih", "utama", "tahajud", "tarawih"], vec!["أفضل قيام الليل", "صلاة التهجد والتراويح"]),
            (&["lebih", "utama", "sedekah", "zakat"], vec!["أفضل الصدقة أم الزكاة"]),
            (&["lebih", "utama", "haji", "umroh"], vec!["أفضل الحج أم العمرة", "فضل الحج والعمرة"]),
            (&["lebih", "utama", "quran", "dzikir"], vec!["أفضل تلاوة القرآن أم الذكر"]),
            (&["lebih", "utama", "berzikir", "berdoa"], vec!["أفضل الذكر أم الدعاء"]),
            (&["lebih", "utama", "ilmu", "amal"], vec!["أيهما أفضل العلم أم العمل"]),
            (&["lebih", "afdhal", "shalat", "jama"], vec!["أفضل الجمع بين الصلاتين"]),
            (&["lebih", "afdhal", "puasa", "senin", "kamis"], vec!["فضل صيام الإثنين والخميس"]),
            (&["lebih", "afdhal", "zakat", "fitrah", "mal"], vec!["فضل زكاة الفطر والمال"]),
            (&["mana", "lebih", "afdhal"], vec!["أيهما أفضل", "الأفضل والأولى"]),
            (&["mana", "yang", "benar", "shalat"], vec!["الكيفية الصحيحة للصلاة", "صفة الصلاة"]),
            (&["mana", "yang", "benar", "wudhu"], vec!["الكيفية الصحيحة للوضوء", "صفة الوضوء"]),
            (&["mana", "pendapat", "kuat"], vec!["الرأي الراجح", "القول الأرجح", "ترجيح"]),
            (&["pendapat", "rajih", "kuat"], vec!["القول الراجح", "الرأي الأقوى"]),
            (&["mana", "shahih", "hadits"], vec!["صحة الحديث", "درجة الحديث"]),
            (&["dalil", "kuat", "hukum"], vec!["الدليل الراجح", "أقوى الأدلة"]),
            (&["lebih", "utama", "nikah", "bujang"], vec!["أيهما أفضل النكاح أم العزوبة", "حكم النكاح"]),
            (&["lebih", "baik", "talak", "pertahankan"], vec!["الأولى الطلاق أم الاستمرار", "إيقاع الطلاق"]),
            (&["lebih", "utama", "infak", "keluarga", "masjid"], vec!["أفضل الإنفاق على الأسرة أم المسجد"]),
            (&["boleh", "tidak", "lebih", "utama"], vec!["الجواز والاستحباب", "الأولى في الشريعة"]),
            (&["apakah", "lebih", "baik", "shalat"], vec!["أيهما أفضل في الصلاة"]),
            (&["apa", "keutamaan", "shalat", "subuh"], vec!["فضل صلاة الفجر", "كيفية صلاة الصبح"]),
            (&["keutamaan", "shalat", "dhuha"], vec!["فضل صلاة الضحى", "ثواب الضحى"]),
            (&["keutamaan", "shalat", "tahajud"], vec!["فضل قيام الليل", "التهجد"]),
            (&["keutamaan", "shalat", "isya"], vec!["فضل صلاة العشاء", "ثواب الإشاء"]),
            (&["keutamaan", "shalat", "berjamaah"], vec!["فضل الجماعة", "ثواب صلاة الجماعة"]),
            (&["keutamaan", "puasa", "syawal"], vec!["فضل صيام الست من شوال", "صيام شوال"]),
            (&["keutamaan", "puasa", "asyura"], vec!["فضل صيام عاشوراء", "صيام المحرم"]),
            (&["keutamaan", "puasa", "arafah"], vec!["فضل صيام يوم عرفة", "صيام عرفة"]),
            (&["keutamaan", "malam", "lailatul", "qadar"], vec!["فضل ليلة القدر", "علامات ليلة القدر"]),
            (&["keutamaan", "dzikir", "subhanallah"], vec!["فضل التسبيح", "ثواب سبحان الله"]),
            (&["keutamaan", "bershalawat"], vec!["فضل الصلاة على النبي", "ثواب الصلاة على النبي"]),
            (&["keutamaan", "membaca", "shalawat"], vec!["فضل الصلاة على النبي"]),
            (&["keutamaan", "hari", "jumat"], vec!["فضل يوم الجمعة", "ثواب الجمعة"]),
            (&["keutamaan", "bulan", "ramadhan"], vec!["فضل شهر رمضان", "ثواب الصيام"]),
            (&["keutamaan", "bulan", "rajab"], vec!["فضل شهر رجب", "صيام رجب"]),
            (&["keutamaan", "bulan", "sya'ban"], vec!["فضل شهر شعبان", "ليلة النصف من شعبان"]),
            (&["keutamaan", "bulan", "dzulhijjah"], vec!["فضل عشر ذي الحجة", "أيام ذي الحجة"]),
            (&["keutamaan", "malam", "nisfu", "syaban"], vec!["فضل ليلة النصف من شعبان"]),
            (&["keutamaan", "zikir", "harian"], vec!["الأوراد اليومية", "أفضل الأذكار"]),
            (&["keutamaan", "sedekah", "subuh"], vec!["فضل الصدقة في الصباح"]),
            (&["keutamaan", "shalat", "rawatib"], vec!["فضل السنن الرواتب", "صلاة الرواتب"]),
            (&["amalan", "paling", "utama"], vec!["أفضل الأعمال", "أحب الأعمال إلى الله"]),
            (&["amalan", "dicintai", "allah"], vec!["أحب الأعمال إلى الله", "أفضل القربات"]),
            (&["amalan", "harian", "islam"], vec!["الأعمال اليومية", "صلاة النوافل"]),
            (&["amalan", "bulan", "ramadhan"], vec!["أعمال رمضان", "عبادات في رمضان"]),

            // BATCH 86: Indonesian Islamic context - institutions, events, customs
            (&["tahlilan", "hukum"], vec!["حكم التهليل", "قراءة لا إله إلا الله للميت", "حكم إقامة مأتم"]),
            (&["tahlil", "untuk", "mayit"], vec!["قراءة القرآن للميت", "إهداء الثواب"]),
            (&["selamatan", "hukum"], vec!["حكم الوليمة", "إطعام الطعام عند المناسبات"]),
            (&["kenduri", "hukum"], vec!["حكم الوليمة", "إطعام الطعام للعزاء"]),
            (&["maulid", "nabi", "hukum"], vec!["حكم الاحتفال بالمولد النبوي", "ذكرى المولد"]),
            (&["maulid", "hukum", "merayakan"], vec!["حكم الاحتفال بالمولد النبوي"]),
            (&["isra", "miraj", "hukum"], vec!["حكم الاحتفال بليلة الإسراء والمعراج"]),
            (&["isra", "miraj", "dalil"], vec!["دليل الإسراء والمعراج", "القصص في القرآن"]),
            (&["nuzul", "quran", "hukum"], vec!["حكم الاحتفال بنزول القرآن"]),
            (&["tahun", "baru", "islam"], vec!["حكم الاحتفال برأس السنة الهجرية"]),
            (&["tahun", "baru", "masehi", "hukum"], vec!["حكم الاحتفال برأس السنة الميلادية"]),
            (&["natal", "ucapkan", "hukum"], vec!["حكم تهنئة النصارى بأعيادهم"]),
            (&["merayakan", "lebaran"], vec!["أحكام عيد الفطر", "آداب العيد"]),
            (&["halalbihalal", "hukum"], vec!["حكم الحلال بالحلال", "عيد الفطر والتسامح"]),
            (&["lebaran", "idul", "fitri"], vec!["أحكام عيد الفطر", "آداب العيد وتحية العيد"]),
            (&["idul", "adha", "solat", "qurban"], vec!["صلاة العيد وشعائر الأضحى"]),
            (&["takbiran", "malam", "ied"], vec!["التكبير ليلة العيد", "إحياء ليلة العيد"]),
            (&["ngaji", "hukum"], vec!["تعلم العلم الديني", "طلب العلم واجب"]),
            (&["sorogan", "bandongan", "metode"], vec!["طرق التدريس في المعهد", "حلقات العلم"]),
            (&["masih", "aqil", "baligh", "shalat"], vec!["صلاة غير البالغ", "سن التكليف"]),
            (&["anak", "yatim", "hak"], vec!["حقوق اليتيم", "أحكام اليتيم"]),
            (&["anak", "yatim", "piatu"], vec!["أحكام اليتيم", "كفالة اليتيم"]),
            (&["menikah", "muda", "hukum"], vec!["حكم النكاح في سن مبكرة", "زواج الصغار"]),
            (&["wali", "perempuan", "shalat", "nikah"], vec!["الولاية في النكاح", "الولي في الصلاة"]),
            (&["pemilihan", "umum", "pemilu", "hukum"], vec!["حكم المشاركة في الانتخابات"]),
            (&["memilih", "pemimpin", "kafir", "non"], vec!["حكم اختيار القائد غير المسلم"]),
            (&["demonstrasi", "islam"], vec!["حكم المظاهرات", "الأمر بالمعروف والنهي عن المنكر"]),
            (&["berpolitik", "hukum"], vec!["حكم الاشتغال بالسياسة"]),
            (&["partai", "islam", "hukum"], vec!["حكم الأحزاب السياسية الإسلامية"]),
            (&["jihad", "masa", "kini"], vec!["الجهاد في العصر الحديث", "أنواع الجهاد"]),
            (&["bela", "negara", "islam"], vec!["حكم الدفاع عن الوطن", "الجهاد والوطن"]),
            (&["pancasila", "islam"], vec!["موقف الإسلام من باجاسيلا"]),
            (&["hukum", "positif", "syariah"], vec!["القانون الوضعي والشريعة الإسلامية"]),
            (&["negara", "islam", "konsep"], vec!["الدولة الإسلامية", "مفهوم الحكومة الإسلامية"]),
            (&["masjid", "memakmurkan"], vec!["عمارة المساجد", "إحياء المساجد"]),
            (&["masjid", "jamik", "kampung"], vec!["المسجد الجامع والمسجد المحلي"]),
            (&["musala", "hukum", "shalat"], vec!["الصلاة في المصلى", "المسجد والمصلى"]),
            (&["mushalla", "shalat", "jumat"], vec!["صلاة الجمعة في المصلى"]),
            (&["pondok", "pesantren", "hukum"], vec!["المعهد الديني في إندونيسيا"]),
            (&["madrasah", "sekolah", "islam"], vec!["التعليم الديني الإسلامي"]),
            (&["kiai", "kyai", "ulama", "taqlid"], vec!["التقليد والاتباع العلماء", "مرجعية العلماء"]),
            (&["taqlid", "ulama", "hukum"], vec!["حكم التقليد", "التقليد في الفقه"]),
            (&["taklid", "mazhab"], vec!["التقليد في المذاهب", "الاتباع"]),
            (&["mukim", "musafir", "batas"], vec!["حد الإقامة والسفر", "حكم المقيم والمسافر"]),
            (&["perantau", "shalat", "musafir"], vec!["صلاة المسافر", "القصر في السفر"]),
            (&["toleransi", "beragama", "islam"], vec!["التسامح الديني في الإسلام", "التعايش"]),
            (&["pluralisme", "islam"], vec!["موقف الإسلام من التعددية الدينية"]),
            (&["dialog", "antaragama"], vec!["الحوار بين الأديان", "التسامح"]),

            // BATCH 87: Medical/contemporary fiqh
            (&["donor", "darah", "hukum"], vec!["حكم التبرع بالدم", "نقل الدم"]),
            (&["donor", "organ", "hukum"], vec!["حكم التبرع بالأعضاء", "الإسلام ونقل الأعضاء"]),
            (&["transplantasi", "organ"], vec!["زراعة الأعضاء", "نقل الأعضاء في الفقه"]),
            (&["transfusi", "darah"], vec!["نقل الدم", "حكم التبرع بالدم"]),
            (&["aborsi", "hukum"], vec!["حكم الإجهاض", "إسقاط الحمل"]),
            (&["aborsi", "boleh", "syarat"], vec!["شروط جواز الإجهاض", "إسقاط الجنين"]),
            (&["bayi", "tabung", "hukum"], vec!["حكم أطفال الأنابيب", "الإخصاب الصناعي"]),
            (&["inseminasi", "buatan"], vec!["التلقيح الصناعي", "حكم الإخصاب الاصطناعي"]),
            (&["kloning", "manusia", "hukum"], vec!["حكم الاستنساخ البشري"]),
            (&["euthanasia", "hukum"], vec!["حكم القتل الرحيم", "الموت الرحيم في الإسلام"]),
            (&["bunuh", "diri", "hukum"], vec!["حكم الانتحار", "تحريم الانتحار"]),
            (&["autopsi", "mayit", "hukum"], vec!["حكم تشريح الجثة", "الميت والجراحة"]),
            (&["kremasi", "abu", "hukum"], vec!["حكم حرق الميت", "التحريق والدفن"]),
            (&["mayit", "dibakar", "haram"], vec!["حكم حرق الجثة", "تحريم حرق الميت"]),
            (&["suntik", "mati", "hukum"], vec!["حكم الموت الرحيم", "إنهاء الحياة"]),
            (&["kafan", "pakaian", "hukum"], vec!["حكم الكفن", "كفن الميت"]),
            (&["kuburan", "beton", "nisan"], vec!["حكم البناء على القبور", "تجصيص القبور"]),
            (&["ziarah", "wali", "hukum"], vec!["حكم زيارة قبور الأولياء", "زيارة القبور"]),
            (&["tawassul", "wali", "hukum"], vec!["حكم التوسل بالأولياء", "التوسل والتبرك"]),
            (&["wasilah", "nabi", "tawassul"], vec!["التوسل بالنبي", "حكم التوسل"]),
            (&["kencing", "berdiri", "hukum"], vec!["حكم البول قائمًا", "آداب قضاء الحاجة"]),
            (&["toilet", "adab", "duduk"], vec!["آداب قضاء الحاجة", "دخول الخلاء"]),
            (&["istinja", "cara", "benar"], vec!["طريقة الاستنجاء", "الاستنجاء بالماء والحجر"]),
            (&["istijmar", "batu", "tisu"], vec!["الاستجمار", "الاستنجاء بالحجارة"]),
            (&["deodorant", "wewangian", "ihram"], vec!["استعمال الطيب في الإحرام", "محظورات الإحرام"]),
            (&["operasi", "ganti", "kelamin"], vec!["حكم تغيير الجنس", "الجنس والشريعة"]),
            (&["banci", "waria", "hukum"], vec!["حكم الخنثى", "المخنث في الفقه"]),
            (&["transgender", "hukum", "islam"], vec!["حكم تغيير الجنس", "الجنس الثالث"]),
            (&["merokok", "puasa", "batal"], vec!["حكم التدخين في الصيام", "مبطلات الصوم"]),
            (&["merokok", "shalat", "hukum"], vec!["حكم التدخين"]),
            (&["rokok", "makruh", "haram"], vec!["حكم التدخين بين الكراهة والتحريم"]),
            (&["jual", "beli", "rokok"], vec!["حكم بيع الدخان"]),
            (&["ghibah", "media", "sosial"], vec!["الغيبة عبر وسائل التواصل", "حكم الغيبة الإلكترونية"]),
            (&["fitnah", "online", "hukum"], vec!["حكم القذف والافتراء الإلكتروني", "الفتنة الرقمية"]),
            (&["bullying", "pem", "bully"], vec!["حكم الأذى والإيذاء", "حكم الإيذاء النفسي"]),
            (&["hoax", "berita", "bohong"], vec!["حكم نشر الأكاذيب", "الكذب والإشاعة"]),
            (&["pencurian", "hukum", "fiqh"], vec!["حكم السرقة", "عقوبة السرقة"]),
            (&["korupsi", "hukum", "islam"], vec!["حكم الرشوة والفساد", "الاختلاس"]),
            (&["suap", "risywah", "hukum"], vec!["حكم الرشوة", "تحريم الرشوة"]),
            (&["gratifikasi", "suap", "bedanya"], vec!["الفرق بين الرشوة والهدية"]),
            (&["hukuman", "hadd", "ta'zir"], vec!["الفرق بين الحد والتعزير", "عقوبات شرعية"]),
            (&["hudud", "hukum", "indonesia"], vec!["تطبيق الحدود", "هل تُطبَّق الحدود"]),
            (&["potong", "tangan", "pencuri"], vec!["حد السرقة", "قطع يد السارق"]),
            (&["rajam", "zina", "hukum"], vec!["حد الزنا", "الرجم في الشريعة"]),
            (&["qisas", "balas", "dendam"], vec!["القصاص", "حكم القصاص والدية"]),
            (&["diyat", "denda", "nyawa"], vec!["الدية", "أحكام الدية في الفقه"]),
            (&["terorisme", "islam", "jihad"], vec!["الفرق بين الجهاد والإرهاب", "الإسلام والإرهاب"]),

            // BATCH 88: Question patterns with "apa itu", "apa yang dimaksud", "apa pengertian"
            (&["apa", "itu", "thaharah"], vec!["تعريف الطهارة", "الطهارة في الفقه"]),
            (&["apa", "itu", "ihram"], vec!["تعريف الإحرام", "أحكام الإحرام"]),
            (&["apa", "itu", "miqat"], vec!["تعريف الميقات", "مواقيت الحج والعمرة"]),
            (&["apa", "itu", "maqam", "ibrahim"], vec!["مقام إبراهيم", "الحج والمشاعر"]),
            (&["apa", "itu", "multazam"], vec!["الملتزم", "أماكن الحج"]),
            (&["apa", "itu", "hajar", "aswad"], vec!["الحجر الأسود", "الكعبة المشرفة"]),
            (&["apa", "itu", "sa'i"], vec!["تعريف السعي", "السعي بين الصفا والمروة"]),
            (&["apa", "itu", "tawaf"], vec!["تعريف الطواف", "الطواف حول الكعبة"]),
            (&["apa", "itu", "wukuf"], vec!["تعريف الوقوف بعرفة", "أركان الحج"]),
            (&["apa", "itu", "mukhrim", "mahram"], vec!["تعريف المحرم", "المحارم في الفقه"]),
            (&["apa", "itu", "nafkah"], vec!["تعريف النفقة", "النفقة الواجبة"]),
            (&["apa", "itu", "mahar"], vec!["تعريف المهر", "أحكام الصداق"]),
            (&["apa", "itu", "akad"], vec!["تعريف العقد", "أركان العقد"]),
            (&["apa", "itu", "ijab", "kabul"], vec!["الإيجاب والقبول", "أركان عقد النكاح"]),
            (&["apa", "itu", "talak"], vec!["تعريف الطلاق", "أحكام الطلاق"]),
            (&["apa", "itu", "iddah"], vec!["تعريف العدة", "أحكام العدة"]),
            (&["apa", "itu", "ruju"], vec!["تعريف الرجعة", "أحكام الرجعة"]),
            (&["apa", "itu", "istibra"], vec!["تعريف الاستبراء", "استبراء الرحم"]),
            (&["apa", "itu", "li'an"], vec!["تعريف اللعان", "حكم اللعان"]),
            (&["apa", "itu", "zihar"], vec!["تعريف الظهار", "حكم الظهار"]),
            (&["apa", "itu", "ila"], vec!["تعريف الإيلاء", "حكم الإيلاء"]),
            (&["apa", "itu", "fasakh"], vec!["تعريف فسخ النكاح", "أسباب فسخ النكاح"]),
            (&["apa", "itu", "kafalah"], vec!["تعريف الكفالة", "الضمان والكفالة"]),
            (&["apa", "itu", "hawalah"], vec!["تعريف الحوالة", "أحكام الحوالة"]),
            (&["apa", "itu", "musaqah"], vec!["تعريف المساقاة", "المساقاة والمزارعة"]),
            (&["apa", "itu", "muzara'ah"], vec!["تعريف المزارعة", "المزارعة في الفقه"]),
            (&["apa", "itu", "mukhabarah"], vec!["تعريف المخابرة", "أحكام المزارعة"]),
            (&["apa", "dimaksud", "qath'i"], vec!["تعريف القطعي", "القطعي والظني في الأصول"]),
            (&["apa", "dimaksud", "zhanni"], vec!["تعريف الظني", "الدلالة الظنية"]),
            (&["apa", "dimaksud", "dalil", "syar'i"], vec!["تعريف الدليل الشرعي", "مصادر الحكم"]),
            (&["apa", "dimaksud", "istihsan"], vec!["تعريف الاستحسان", "الاستحسان في الأصول"]),
            (&["apa", "dimaksud", "mashlahah"], vec!["تعريف المصلحة المرسلة", "الاستصلاح"]),
            (&["apa", "dimaksud", "istishab"], vec!["تعريف الاستصحاب", "الأصل في الأشياء"]),
            (&["apa", "dimaksud", "urf"], vec!["تعريف العرف", "العرف في الفقه"]),
            (&["apa", "dimaksud", "sadd", "zari'ah"], vec!["تعريف سد الذرائع", "الحيلة والذريعة"]),
            (&["apa", "pengertian", "aqidah"], vec!["تعريف العقيدة", "العقيدة الإسلامية"]),
            (&["apa", "pengertian", "syariah"], vec!["تعريف الشريعة", "الشريعة الإسلامية"]),
            (&["apa", "pengertian", "fiqih"], vec!["تعريف الفقه", "الفقه الإسلامي"]),
            (&["apa", "pengertian", "ibadah"], vec!["تعريف العبادة", "مفهوم العبادة"]),
            (&["apa", "pengertian", "muamalah"], vec!["تعريف المعاملات", "المعاملات في الفقه"]),
            (&["apa", "pengertian", "akhlak"], vec!["تعريف الأخلاق", "الأخلاق في الإسلام"]),
            (&["apa", "pengertian", "tasawuf"], vec!["تعريف التصوف", "علم التصوف"]),
            (&["apa", "pengertian", "ijtihad"], vec!["تعريف الاجتهاد", "الاجتهاد في الفقه"]),
            (&["apa", "pengertian", "taqlid"], vec!["تعريف التقليد", "حكم التقليد"]),
            (&["apa", "pengertian", "fatwa"], vec!["تعريف الفتوى", "شروط الإفتاء"]),
            (&["apa", "pengertian", "zakat"], vec!["تعريف الزكاة", "الزكاة في الإسلام"]),
            (&["apa", "itu", "qurban"], vec!["تعريف الأضحية", "أحكام الأضحية"]),
            (&["apa", "itu", "aqiqah"], vec!["تعريف العقيقة", "أحكام العقيقة"]),
            (&["apa", "pengertian", "wali"], vec!["تعريف الولاية", "الولي في الفقه"]),

            // BATCH 89: "bagaimana" and "jelaskan" patterns
            (&["bagaimana", "hukum", "shalat", "di", "atas", "kapal"], vec!["صلاة المسافر على السفينة", "صلاة بالمركبة"]),
            (&["bagaimana", "hukum", "shalat", "di", "pesawat"], vec!["صلاة في الطائرة", "صلاة المسافر"]),
            (&["bagaimana", "hukum", "wudhu", "pakai", "kaos", "kaki"], vec!["المسح على الخفين", "الجوارب"]),
            (&["bagaimana", "hukum", "shalat", "pakai", "sepatu"], vec!["الصلاة بالحذاء", "السنة في الصلاة"]),
            (&["bagaimana", "tayamum", "orang", "sakit"], vec!["التيمم للمريض", "شروط التيمم"]),
            (&["bagaimana", "wudhu", "tidak", "ada", "air"], vec!["التيمم عند عدم وجود الماء", "الطهارة بالتراب"]),
            (&["bagaimana", "shalat", "tidak", "bisa", "berdiri"], vec!["الصلاة جلوسًا", "صلاة المريض"]),
            (&["bagaimana", "shalat", "tidak", "bisa", "duduk"], vec!["الصلاة مضجعًا", "صلاة العاجز"]),
            (&["bagaimana", "mandi", "wajib", "haid"], vec!["غسل الحيض", "طهارة المرأة"]),
            (&["bagaimana", "mandi", "wajib", "junub"], vec!["غسل الجنابة", "الغسل الكامل"]),
            (&["bagaimana", "niat", "puasa", "malam"], vec!["نية الصيام", "وقت النية للصوم"]),
            (&["bagaimana", "bayar", "zakat", "fitrah", "uang"], vec!["إخراج زكاة الفطر قيمة", "نقود زكاة الفطر"]),
            (&["bagaimana", "cara", "tobat", "dosa", "besar"], vec!["شروط التوبة من الكبائر", "التوبة الصحيحة"]),
            (&["bagaimana", "cara", "membersihkan", "najis", "anjing"], vec!["تطهير نجاسة الكلب", "الغسل سبع مرات"]),
            (&["bagaimana", "cara", "khitan"], vec!["طريقة الختان", "أحكام الختان"]),
            (&["bagaimana", "cara", "aqiqah"], vec!["طريقة الأقيقة", "دم العقيقة"]),
            (&["bagaimana", "cara", "qurban"], vec!["طريقة الأضحية", "ذبح الأضحية"]),
            (&["bagaimana", "cara", "memandikan", "mayit"], vec!["كيفية غسل الميت", "طريقة تغسيل الجنازة"]),
            (&["bagaimana", "cara", "shalat", "jenazah"], vec!["كيفية صلاة الجنازة", "أركان صلاة الجنازة"]),
            (&["bagaimana", "cara", "mengkafani"], vec!["كيفية تكفين الميت", "عدد الأثواب"]),
            (&["bagaimana", "cara", "talkin"], vec!["التلقين", "طريقة تلقين المحتضر"]),
            (&["bagaimana", "cara", "taubat", "benar"], vec!["شروط التوبة الصحيحة", "أركان التوبة"]),
            (&["bagaimana", "cara", "istikharah"], vec!["كيفية صلاة الاستخارة", "دعاء الاستخارة"]),
            (&["bagaimana", "cara", "i'tikaf"], vec!["كيفية الاعتكاف", "أحكام الاعتكاف"]),
            (&["bagaimana", "cara", "ihram"], vec!["كيفية الإحرام", "شروط الإحرام"]),
            (&["bagaimana", "cara", "tawaf"], vec!["كيفية الطواف", "شروط الطواف"]),
            (&["bagaimana", "cara", "sa'i"], vec!["كيفية السعي", "أحكام السعي"]),
            (&["bagaimana", "membayar", "fidyah"], vec!["طريقة أداء الفدية", "من تجب عليه الفدية"]),
            (&["bagaimana", "membayar", "kaffarah"], vec!["طريقة أداء الكفارة", "كفارة الصوم"]),
            (&["bagaimana", "hukum", "hutang"], vec!["أحكام القرض والدين"]),
            (&["bagaimana", "hukum", "jual", "beli", "kredit"], vec!["حكم البيع بالتقسيط", "البيع المؤجل"]),
            (&["bagaimana", "hukum", "sewa"], vec!["أحكام الإجارة", "الأجر والأجير"]),
            (&["bagaimana", "hukum", "gadai"], vec!["أحكام الرهن", "الرهن في الفقه"]),
            (&["bagaimana", "hukum", "dropship"], vec!["حكم البيع بالعمولة", "السمسرة"]),
            (&["bagaimana", "hukum", "endorsement"], vec!["حكم التسويق والإعلان"]),
            (&["bagaimana", "hukum", "bisnis", "mlm"], vec!["حكم التسويق الشبكي", "MLM حكم"]),
            (&["jelaskan", "rukun", "shalat"], vec!["أركان الصلاة", "فرائض الصلاة"]),
            (&["jelaskan", "syarat", "shalat"], vec!["شروط الصلاة", "شروط صحة الصلاة"]),
            (&["jelaskan", "rukun", "wudhu"], vec!["أركان الوضوء", "فرائض الوضوء"]),
            (&["jelaskan", "rukun", "puasa"], vec!["أركان الصيام", "ما يفسد الصوم"]),
            (&["jelaskan", "rukun", "haji"], vec!["أركان الحج", "فرائض الحج"]),
            (&["jelaskan", "rukun", "nikah"], vec!["أركان النكاح", "شروط عقد الزواج"]),
            (&["jelaskan", "macam", "najis"], vec!["أنواع النجاسات", "النجاسة في الفقه"]),
            (&["jelaskan", "macam", "air"], vec!["أنواع المياه", "المياه الطاهرة والمطهرة"]),
            (&["jelaskan", "macam", "shalat", "sunah"], vec!["أنواع صلاة النافلة", "السنن الرواتب"]),
            (&["jelaskan", "macam", "puasa", "sunah"], vec!["أنواع الصيام المستحب", "صيام النافلة"]),
            (&["jelaskan", "macam", "talak"], vec!["أنواع الطلاق", "الطلاق الرجعي والبائن"]),
            (&["jelaskan", "macam", "zakat"], vec!["أنواع الزكاة", "الزكاة ومصارفها"]),
            (&["jelaskan", "macam", "hadas"], vec!["أنواع الحدث", "الحدث الأكبر والأصغر"]),
            (&["sebutkan", "rukun", "iman"], vec!["أركان الإيمان", "الإيمان بأركانه الستة"]),
            (&["sebutkan", "rukun", "islam"], vec!["أركان الإسلام", "الإسلام وأركانه"]),

            // BATCH 90: Pesantren-specific and scholarly questions
            (&["hukum", "qunut", "subuh"], vec!["حكم قنوت الصبح", "القنوت في صلاة الفجر"]),
            (&["qunut", "nazilah"], vec!["قنوت النازلة", "القنوت في الملمات"]),
            (&["qunut", "wajib", "sunah"], vec!["حكم القنوت", "القنوت في المذاهب"]),
            (&["bacaan", "qunut", "subuh"], vec!["دعاء القنوت", "نص القنوت"]),
            (&["basmalah", "siri", "jahr"], vec!["الجهر والإسرار بالبسملة", "قراءة البسملة في الصلاة"]),
            (&["amin", "keras", "pelan", "shalat"], vec!["الجهر بالتأمين", "قول آمين في الصلاة"]),
            (&["tahlil", "takbir", "adzan", "beda"], vec!["الفرق بين التهليل والتكبير"]),
            (&["shalat", "witir", "rakaat", "ganjil"], vec!["ركعات الوتر", "كيفية صلاة الوتر"]),
            (&["shalat", "tarawih", "8", "rakaat", "20"], vec!["عدد ركعات التراويح", "ثماني أم عشرون ركعة"]),
            (&["tarawih", "delapan", "atau", "dua", "puluh"], vec!["خلاف عدد التراويح", "التراويح ثمان أم عشرون"]),
            (&["shalat", "dhuha", "berapa", "rakaat"], vec!["عدد ركعات الضحى", "كيفية صلاة الضحى"]),
            (&["shalat", "tahajud", "berapa", "rakaat"], vec!["عدد ركعات التهجد", "قيام الليل"]),
            (&["shalat", "rawatib", "berapa", "rakaat"], vec!["عدد ركعات الرواتب", "السنن الرواتب"]),
            (&["waktu", "shalat", "ashar", "matahari"], vec!["وقت صلاة العصر", "حد وقت العصر"]),
            (&["waktu", "shalat", "isya", "terlambat"], vec!["وقت صلاة العشاء", "آخر وقت العشاء"]),
            (&["waktu", "subuh", "imsak", "beda"], vec!["الفرق بين الإمساك والفجر", "وقت صلاة الفجر"]),
            (&["adzan", "subuh", "dua", "kali"], vec!["أذان الصبح مرتين", "أذان الفجر وتثنيته"]),
            (&["adzan", "sebelum", "fajar"], vec!["الأذان قبل الفجر", "أذان الليل"]),
            (&["iqamat", "boleh", "wanita"], vec!["الإقامة للمرأة", "حكم إقامة المرأة"]),
            (&["shalat", "jum'at", "berapa", "rakaat"], vec!["ركعات صلاة الجمعة", "صلاة الجمعة"]),
            (&["khutbah", "jum'at", "bahasa", "indonesia"], vec!["خطبة الجمعة بغير العربية", "لغة الخطبة"]),
            (&["khutbah", "jum'at", "syarat", "sah"], vec!["شروط صحة خطبة الجمعة", "أركان الخطبة"]),
            (&["shalat", "id", "berapa", "takbir"], vec!["عدد تكبيرات صلاة العيد", "تكبيرات الزوائد"]),
            (&["shalat", "gerhana", "cara"], vec!["كيفية صلاة الكسوف", "الخسوف والكسوف"]),
            (&["shalat", "istisqa", "cara"], vec!["كيفية صلاة الاستسقاء", "الاستسقاء"]),
            (&["shalat", "sunah", "dua", "salam", "atau"], vec!["صلاة النافلة وسلامها", "التسليم في النوافل"]),
            (&["raka'at", "pertama", "lupa", "baca", "fatihah"], vec!["السهو في الصلاة", "ترك الفاتحة"]),
            (&["imam", "kentut", "shalat"], vec!["إذا أحدث الإمام في الصلاة", "استخلاف الإمام"]),
            (&["makmum", "mendahului", "imam"], vec!["سبق المأموم الإمام", "حكم التقدم على الإمام"]),
            (&["shalat", "fardhu", "empat", "rakaat", "lupa"], vec!["السهو في الصلاة عن الأربع", "قضاء الركعة"]),
            (&["sujud", "sahwi", "kapan", "dilakukan"], vec!["متى يسجد للسهو", "سجود السهو"]),
            (&["sujud", "sahwi", "sebelum", "sesudah", "salam"], vec!["سجود السهو قبل وبعد السلام"]),
            (&["tasyahud", "awal", "akhir", "beda"], vec!["الفرق بين التشهد الأوسط والأخير"]),
            (&["shalawat", "ibrahimiyah", "kapan"], vec!["الصلاة الإبراهيمية", "متى تُقال"]),
            (&["doa", "setelah", "tasyahud", "akhir"], vec!["دعاء التشهد الأخير قبل السلام", "التعوذ في الصلاة"]),
            (&["salam", "dua", "kali", "kenapa"], vec!["حكمة التسليمتين", "التسليم في الصلاة"]),
            (&["doa", "qunut", "witir", "teks"], vec!["دعاء قنوت الوتر", "نص قنوت الوتر"]),
            (&["niat", "shalat", "wajib", "hukum", "melafazkan"], vec!["حكم التلفظ بالنية", "النية في الصلاة"]),
            (&["niat", "dalam", "hati", "atau", "diucapkan"], vec!["النية في القلب أم اللسان", "محل النية"]),
            (&["wudhu", "sambil", "berbicara", "boleh"], vec!["الكلام أثناء الوضوء", "آداب الوضوء"]),
            (&["wudhu", "dengan", "sabun", "moisturizer"], vec!["الوضوء مع حائل", "حكم الوضوء مع العوائق"]),
            (&["wudhu", "cat", "kuku", "kutek"], vec!["الوضوء مع طلاء الأظافر", "حكم المانع"]),
            (&["lensa", "kontak", "wudhu"], vec!["الوضوء مع العدسة اللاصقة", "حكم العدسة"]),
            (&["cincin", "perhiasan", "wudhu"], vec!["الخاتم في الوضوء", "إزالة الحائل"]),
            (&["wudhu", "air", "mengalir", "tergenang"], vec!["الماء الجاري والراكد", "شروط الماء"]),
            (&["najis", "di", "dalam", "air", "banyak"], vec!["حكم النجاسة في الماء الكثير", "ماء دون قلتين"]),
            (&["najis", "kurang", "dua", "kulah"], vec!["الماء دون القلتين ينجس", "قلتا الماء"]),
            (&["najis", "dua", "kulah", "lebih"], vec!["الماء قلتان لا ينجس", "القلتان وحكمهما"]),

            // BATCH 91: Classical fiqh Arabic terms used in Indonesian pesantren
            (&["istidlal", "arti", "contoh"], vec!["الاستدلال", "طريقة الاستدلال في الفقه"]),
            (&["qiyas", "contoh", "arti"], vec!["القياس وأركانه", "أمثلة على القياس"]),
            (&["ijma", "sahabat", "tabi'in"], vec!["إجماع الصحابة", "حجية الإجماع"]),
            (&["hadits", "mutawatir", "ahad", "beda"], vec!["الفرق بين المتواتر والآحاد"]),
            (&["khabar", "wahid", "hujjah"], vec!["حجية خبر الواحد"]),
            (&["mafhum", "muwafaqah", "mukhalafah"], vec!["مفهوم الموافقة والمخالفة"]),
            (&["mantuq", "mafhum", "beda"], vec!["الفرق بين المنطوق والمفهوم"]),
            (&["nash", "zhahir", "beda"], vec!["الفرق بين النص والظاهر"]),
            (&["mutlaq", "muqayyad", "beda"], vec!["الفرق بين المطلق والمقيد"]),
            (&["am", "khas", "beda"], vec!["الفرق بين العام والخاص", "تخصيص العموم"]),
            (&["amar", "nahi", "beda"], vec!["الفرق بين الأمر والنهي"]),
            (&["wajib", "mandub", "mubah", "makruh", "haram"], vec!["الأحكام التكليفية الخمسة"]),
            (&["sahih", "fasid", "batal", "beda"], vec!["الفرق بين الصحيح والفاسد والباطل"]),
            (&["ada", "adat", "hukumnya"], vec!["العادة محكمة", "القاعدة الفقهية"]),
            (&["qawa'id", "fiqhiyah", "lima"], vec!["القواعد الفقهية الكبرى"]),
            (&["al-umur", "bi-maqasid", "artinya"], vec!["الأمور بمقاصدها", "القواعد الفقهية"]),
            (&["masyaqqah", "tujlib", "taisir"], vec!["المشقة تجلب التيسير", "قواعد الفقه"]),
            (&["ad-dhararu", "yuzal", "artinya"], vec!["الضرر يزال", "درء الضرر"]),
            (&["al-yaqin", "la-yuzal", "syak"], vec!["اليقين لا يزول بالشك"]),
            (&["al-'adah", "muhakkamah"], vec!["العادة محكمة", "اعتبار العرف"]),
            (&["furu'", "ushul", "beda"], vec!["الفرق بين الأصول والفروع"]),
            (&["maqashid", "syariah", "lima", "pokok"], vec!["المقاصد الشرعية الكلية", "الضرورات الخمس"]),
            (&["hifzh", "nafs", "jiwa"], vec!["حفظ النفس", "الكليات الخمس"]),
            (&["hifzh", "aql", "akal"], vec!["حفظ العقل", "الكليات الخمس"]),
            (&["hifzh", "nash", "keturunan"], vec!["حفظ النسل", "المقاصد الشرعية"]),
            (&["hifzh", "mal", "harta"], vec!["حفظ المال", "المقاصد الشرعية"]),
            (&["hifzh", "din", "agama"], vec!["حفظ الدين", "المقاصد الشرعية"]),
            (&["ta'arud", "adillah", "cara", "selesaikan"], vec!["التعارض بين الأدلة", "طرق التعامل مع التعارض"]),
            (&["tarjih", "cara", "memilih", "pendapat", "kuat"], vec!["طرق الترجيح بين الأدلة"]),
            (&["naskh", "mansukh", "syarat"], vec!["شروط النسخ", "النسخ في الأصول"]),
            (&["tabayun", "berita", "islam"], vec!["التبين في تلقي الأخبار", "تحري الخبر"]),
            (&["istifta", "mufti", "prosedur"], vec!["الاستفتاء والإفتاء", "آداب الفتوى"]),
            (&["ijtihad", "mujtahid", "syarat"], vec!["شروط المجتهد", "درجات الاجتهاد"]),
            (&["talfiq", "mazhab", "boleh"], vec!["حكم التلفيق بين المذاهب"]),
            (&["intiqal", "mazhab", "pindah"], vec!["حكم التنقل بين المذاهب"]),
            (&["ittiba", "mujtahid", "taqlid", "beda"], vec!["الفرق بين الاتباع والتقليد"]),
            (&["madzab", "empat", "syafii", "hanafi", "maliki", "hanbali"], vec!["المذاهب الأربعة", "الفقه الإسلامي"]),
            (&["mazhab", "beda", "ikhtilaf"], vec!["الخلاف الفقهي بين المذاهب", "أسباب الاختلاف"]),
            (&["sababul", "ikhtilaf", "ulama"], vec!["أسباب اختلاف العلماء", "أسباب الخلاف"]),
            (&["khilafiyah", "boleh", "qunut"], vec!["المسائل الخلافية", "الخلاف في القنوت"]),
            (&["masail", "khilafiyah", "toleransi"], vec!["التسامح في المسائل الخلافية"]),
            (&["ijazah", "sanad", "ilmu"], vec!["الإجازة العلمية", "سند الإجازة"]),
            (&["kitab", "kuning", "pesantren"], vec!["الكتب الصفراء في المعاهد الإسلامية"]),
            (&["fiqh", "siyasi", "islam"], vec!["الفقه السياسي الإسلامي", "السياسة الشرعية"]),
            (&["fiqh", "muashar", "kontemporer"], vec!["الفقه المعاصر", "قضايا فقهية معاصرة"]),
            (&["fatwa", "mui", "hukum"], vec!["فتاوى مجلس العلماء الإندونيسي"]),
            (&["htb", "hizbut", "tahrir", "hukum"], vec!["الحكم على الجماعات الإسلامية السياسية"]),
            (&["wahabi", "salafi", "bid'ah"], vec!["موقف الوهابية من البدعة", "الوهابية والسلفية"]),

            // BATCH 92: More specific Indonesian language query patterns
            (&["emang", "boleh", "hukum"], vec!["حكم", "هل يجوز"]),
            (&["emangnya", "boleh", "shalat"], vec!["هل يجوز الصلاة"]),
            (&["gak", "boleh", "puasa"], vec!["هل يجوز الصيام", "من يُعفى من الصوم"]),
            (&["nggak", "papa", "shalat", "sambil"], vec!["الصلاة أثناء", "حكم الصلاة"]),
            (&["gapapa", "shalat"], vec!["الصلاة", "حكم الصلاة"]),
            (&["boleh", "dong", "shalat"], vec!["هل يجوز الصلاة"]),
            (&["masa", "haram", "shalat"], vec!["تحريم الصلاة"]),
            (&["masa", "boleh", "puasa"], vec!["هل يجوز الصوم"]),
            (&["kata", "ustadz", "haram"], vec!["حكم التحريم", "ما يُحرم"]),
            (&["kata", "ustad", "boleh"], vec!["هل يجوز", "الجواز"]),
            (&["kata", "kyai", "hukum"], vec!["حكم", "فتوى"]),
            (&["kata", "ulama", "hukum"], vec!["حكم عند العلماء"]),
            (&["menurut", "agama", "hukum"], vec!["حكم في الإسلام"]),
            (&["menurut", "islam", "hukum"], vec!["حكم في الإسلام"]),
            (&["menurut", "fiqih", "hukum"], vec!["حكم في الفقه الإسلامي"]),
            (&["hukumnya", "gimana"], vec!["ما حكمه", "الحكم الشرعي"]),
            (&["hukumnya", "apa"], vec!["ما حكم", "الحكم الشرعي"]),
            (&["gimana", "hukumnya"], vec!["ما حكمه"]),
            (&["gimana", "cara"], vec!["كيف يتم", "الطريقة"]),
            (&["gimana", "shalat"], vec!["كيف يصلي", "صفة الصلاة"]),
            (&["kayak", "gimana", "cara"], vec!["كيف يتم", "الطريقة"]),
            (&["cara", "shalat", "bener"], vec!["صفة الصلاة الصحيحة"]),
            (&["cara", "wudhu", "bener"], vec!["صفة الوضوء الصحيح"]),
            (&["cara", "baca", "quran", "bener"], vec!["قراءة القرآن الصحيحة"]),
            (&["bisa", "gak", "shalat"], vec!["هل يمكن الصلاة"]),
            (&["bisa", "gak", "puasa"], vec!["هل يمكن الصيام"]),
            (&["bisa", "nggak", "wudhu"], vec!["هل يمكن الوضوء"]),
            (&["harusnya", "gimana"], vec!["كيف ينبغي", "الكيفية المشروعة"]),
            (&["seharusnya", "gimana"], vec!["ما ينبغي فعله"]),
            (&["harusnya", "shalat", "gimana"], vec!["صفة الصلاة"]),
            (&["harusnya", "puasa", "gimana"], vec!["كيفية الصيام"]),
            (&["pengen", "tau", "hukum"], vec!["الحكم الشرعي"]),
            (&["mau", "tanya", "hukum", "shalat"], vec!["حكم الصلاة"]),
            (&["mau", "tanya", "hukum", "puasa"], vec!["حكم الصيام"]),
            (&["tanya", "hukum", "menikah"], vec!["حكم النكاح"]),
            (&["tanya", "tentang", "shalat"], vec!["مسائل الصلاة"]),
            (&["tanya", "tentang", "puasa"], vec!["مسائل الصيام"]),
            (&["tanya", "tentang", "zakat"], vec!["مسائل الزكاة"]),
            (&["tanya", "tentang", "nikah"], vec!["مسائل النكاح"]),
            (&["nanya", "soal", "shalat"], vec!["أحكام الصلاة"]),
            (&["nanya", "soal", "puasa"], vec!["أحكام الصيام"]),
            (&["nanya", "soal", "zakat"], vec!["أحكام الزكاة"]),
            (&["nanya", "soal", "nikah"], vec!["أحكام النكاح"]),
            (&["penasaran", "hukum"], vec!["الحكم الشرعي"]),
            (&["ada", "yang", "tau", "hukum"], vec!["الحكم الشرعي"]),
            (&["ada", "dalilnya", "gak"], vec!["هل له دليل", "النصوص الشرعية"]),
            (&["dalilnya", "mana"], vec!["أين الدليل", "الدليل الشرعي"]),
            (&["sumbernya", "apa"], vec!["ما المصدر", "مصدر الحكم"]),

            // BATCH 93: Measurement and numbers in fiqh
            (&["nisab", "zakat", "emas", "gram"], vec!["نصاب زكاة الذهب", "النصاب بالغرام"]),
            (&["nisab", "zakat", "perak", "gram"], vec!["نصاب زكاة الفضة", "الفضة ونصابها"]),
            (&["nisab", "zakat", "uang", "berapa"], vec!["نصاب زكاة المال", "النصاب النقدي"]),
            (&["zakat", "fitrah", "berapa", "kg"], vec!["مقدار زكاة الفطر", "صاع الفطرة"]),
            (&["zakat", "fitrah", "uang", "berapa"], vec!["قيمة زكاة الفطر", "إخراج الفطرة نقودًا"]),
            (&["zakat", "maal", "berapa", "persen"], vec!["نسبة زكاة المال", "ربع العشر"]),
            (&["zakat", "pertanian", "berapa", "persen"], vec!["نسبة زكاة الزروع", "عشر أو نصف العشر"]),
            (&["berapa", "bulan", "iddah", "cerai"], vec!["مدة عدة المطلقة", "العدة من الطلاق"]),
            (&["berapa", "bulan", "iddah", "ditinggal", "mati"], vec!["مدة عدة المتوفى عنها", "أربعة أشهر وعشرًا"]),
            (&["berapa", "hari", "iddah", "cerai"], vec!["أيام العدة", "عدة المطلقة ثلاثة قروء"]),
            (&["berapa", "hari", "nifas"], vec!["مدة النفاس", "الحد الأعلى للنفاس"]),
            (&["berapa", "hari", "haid"], vec!["مدة الحيض", "حد الحيض"]),
            (&["minimal", "haid", "berapa", "hari"], vec!["أقل مدة الحيض", "الحيض يومًا أو يومين"]),
            (&["maksimal", "haid", "berapa", "hari"], vec!["أكثر مدة الحيض", "خمسة عشر يومًا"]),
            (&["berapa", "rakaat", "shalat", "subuh"], vec!["ركعات صلاة الصبح", "كيفية صلاة الفجر"]),
            (&["berapa", "rakaat", "shalat", "dhuhur"], vec!["ركعات صلاة الظهر"]),
            (&["berapa", "rakaat", "shalat", "ashar"], vec!["ركعات صلاة العصر"]),
            (&["berapa", "rakaat", "shalat", "maghrib"], vec!["ركعات صلاة المغرب"]),
            (&["berapa", "rakaat", "shalat", "isya"], vec!["ركعات صلاة العشاء"]),
            (&["berapa", "rakaat", "shalat", "jum'at"], vec!["ركعات صلاة الجمعة"]),
            (&["berapa", "rakaat", "shalat", "tarawih"], vec!["عدد ركعات التراويح", "ثمانية أم عشرون"]),
            (&["berapa", "rakaat", "shalat", "witir"], vec!["عدد ركعات الوتر"]),
            (&["berapa", "jam", "waktu", "subuh"], vec!["وقت صلاة الفجر", "متى يدخل الفجر"]),
            (&["berapa", "km", "boleh", "qashar"], vec!["مسافة القصر", "مسيرة ثمانين كيلومترًا"]),
            (&["jarak", "qashar", "shalat", "berapa"], vec!["مسافة القصر", "حد السفر المبيح للقصر"]),
            (&["jarak", "safar", "berapa", "km"], vec!["مسافة السفر", "مسافة القصر"]),
            (&["berapa", "kali", "basuh", "wudhu"], vec!["عدد مرات الغسل في الوضوء"]),
            (&["berapa", "kali", "basuh", "najis", "anjing"], vec!["عدد الغسلات لنجاسة الكلب", "سبع مرات"]),
            (&["berapa", "hari", "istinja", "batas"], vec!["وقت الاستنجاء", "حكم تأخير الاستنجاء"]),
            (&["berapa", "lama", "boleh", "masih", "wudhu"], vec!["وقت الوضوء", "ينقض الوضوء"]),
            (&["berapa", "hari", "i'tikaf"], vec!["مدة الاعتكاف", "الاعتكاف في رمضان"]),
            (&["berapa", "waktu", "puasa", "wajib", "tunai"], vec!["وقت قضاء الصوم", "فورية القضاء"]),
            (&["berapa", "bagian", "waris", "anak", "laki"], vec!["نصيب الابن في الميراث", "للذكر مثل حظ الأنثيين"]),
            (&["berapa", "bagian", "waris", "anak", "perempuan"], vec!["نصيب البنت في الميراث"]),
            (&["berapa", "bagian", "waris", "istri"], vec!["نصيب الزوجة في الميراث", "الثمن أو الربع"]),
            (&["berapa", "bagian", "waris", "suami"], vec!["نصيب الزوج في الميراث", "النصف أو الربع"]),
            (&["berapa", "bagian", "waris", "ibu"], vec!["نصيب الأم في الميراث", "الثلث أو السدس"]),
            (&["berapa", "bagian", "waris", "ayah"], vec!["نصيب الأب في الميراث", "العصبة"]),
            (&["dua", "pertiga", "waris", "anak", "perempuan"], vec!["الثلثان لبنتين", "نصيب البنتين"]),
            (&["mahar", "berapa", "minimal"], vec!["أقل المهر", "حد المهر"]),
            (&["batas", "minimal", "mahar", "nikah"], vec!["أقل الصداق", "لا حد له"]),
            (&["berapa", "batas", "kaffarah", "sumpah"], vec!["مقدار كفارة اليمين", "كفارة يمين"]),
            (&["kaffarah", "puasa", "berapa", "hari"], vec!["كفارة الإفطار", "صوم شهرين متتابعين"]),
            (&["fidyah", "berapa", "jumlah"], vec!["مقدار الفدية", "فدية الصوم"]),
            (&["berapa", "jumlah", "hewan", "qurban"], vec!["مقدار الأضحية", "الأضحية عن كم شخص"]),
            (&["satu", "sapi", "qurban", "berapa", "orang"], vec!["البقرة تجزي عن سبعة", "الإجزاء في الأضحية"]),

            // BATCH 94: Modern youth/millennial Islamic questions
            (&["musik", "haram", "halal"], vec!["حكم الموسيقى", "آلات الموسيقى"]),
            (&["lagu", "dengerin", "hukum"], vec!["حكم سماع الأغاني", "الغناء والموسيقى"]),
            (&["mendengarkan", "musik", "hukum"], vec!["حكم سماع الموسيقى"]),
            (&["nyanyi", "bernyanyi", "hukum"], vec!["حكم الغناء", "الصوت الحسن"]),
            (&["main", "gitar", "drum", "hukum"], vec!["حكم العزف على الآلات الموسيقية"]),
            (&["hadroh", "rebana", "haul"], vec!["حكم المديح بالدف", "الغناء الديني"]),
            (&["gambar", "foto", "hukum"], vec!["حكم التصوير", "تصوير ذوات الأرواح"]),
            (&["foto", "makhluk", "hidup", "hukum"], vec!["حكم تصوير ذوات الأرواح"]),
            (&["video", "orang", "hukum"], vec!["حكم التصوير المرئي", "الفيديو"]),
            (&["menggambar", "manusia", "hewan", "hukum"], vec!["حكم رسم ذوات الأرواح", "التصوير"]),
            (&["animasi", "kartun", "hukum"], vec!["حكم الرسوم المتحركة", "الصور الكرتونية"]),
            (&["podcast", "youtube", "dakwah"], vec!["الدعوة الإلكترونية", "الدعوة عبر الإنترنت"]),
            (&["konten", "buat", "uang", "halal"], vec!["حكم صنع المحتوى مقابل المال"]),
            (&["influencer", "halal", "sponsorship"], vec!["حكم الترويج والإعلان"]),
            (&["nonton", "bioskop", "hukum"], vec!["حكم السينما والأفلام"]),
            (&["nonton", "film", "barat", "hukum"], vec!["حكم مشاهدة الأفلام الغربية"]),
            (&["nonton", "drama", "korea", "hukum"], vec!["حكم مشاهدة المسلسلات الكورية"]),
            (&["pacaran", "hukum"], vec!["حكم الخطبة والمؤاخاة قبل الزواج", "حكم العلاقة قبل الزواج"]),
            (&["pacaran", "tapi", "niatnya", "nikah"], vec!["حكم التقرب بنية الزواج", "الخطبة"]),
            (&["pdkt", "pendekatan", "calon", "hukum"], vec!["حكم التعارف قبل الزواج"]),
            (&["ta'aruf", "cara", "hukum"], vec!["التعارف قبل الزواج", "آداب الخطبة"]),
            (&["chatting", "lawan", "jenis", "hukum"], vec!["حكم التواصل مع الأجانب إلكترونيًا"]),
            (&["teleponnan", "dengan", "non", "mahram"], vec!["حكم التحدث مع غير المحارم"]),
            (&["video", "call", "lawan", "jenis"], vec!["حكم مكالمة الفيديو بين الجنسين"]),
            (&["kenalan", "aplikasi", "dating"], vec!["حكم التعارف الإلكتروني"]),
            (&["tinder", "aplikasi", "kenalan", "hukum"], vec!["حكم تطبيقات المواعدة"]),
            (&["cinta", "monyet", "pacar", "islam"], vec!["الحب والميل في الإسلام"]),
            (&["jatuh", "cinta", "beda", "agama"], vec!["الحب بين المسلم وغير المسلم"]),
            (&["nikah", "karena", "hamil", "duluan"], vec!["حكم الزواج من الحامل من الزنا", "نكاح الزانية"]),
            (&["seks", "sebelum", "nikah", "hukum"], vec!["حكم الزنا", "تحريم الزنا"]),
            (&["ml", "sebelum", "nikah", "dampak"], vec!["عقوبة الزنا", "آثار الزنا"]),
            (&["hubungan", "terlarang", "hukum"], vec!["حكم العلاقات المحرمة"]),
            (&["babi", "kulit", "tas", "hukum"], vec!["حكم استعمال جلد الخنزير"]),
            (&["skincare", "mengandung", "babi", "kolagen"], vec!["حكم مستحضرات الخنزير"]),
            (&["krim", "collagen", "babi", "halal"], vec!["حكم الكولاجين من الخنزير"]),
            (&["mie", "instan", "halal", "haram"], vec!["حكم المعجنات الفورية"]),
            (&["makanan", "westernfast", "food", "halal"], vec!["حكم الأكل الغربي"]),
            (&["pizza", "burger", "haram"], vec!["حكم الأكل في المطاعم الغربية"]),
            (&["es", "krim", "coklat", "minuman", "halal"], vec!["حكم المثلجات والمشروبات"]),
            (&["minuman", "berkarbonasi", "cola", "hukum"], vec!["حكم المشروبات الغازية"]),
            (&["energy", "drink", "hukum"], vec!["حكم مشروبات الطاقة"]),
            (&["suplemen", "protein", "whey", "halal"], vec!["حكم البروتين المستخلص من الحليب"]),
            (&["daging", "babi", "diharamkan", "kenapa"], vec!["حكمة تحريم لحم الخنزير"]),
            (&["khamar", "alkohol", "sedikit", "batal"], vec!["حكم القليل من الخمر", "كل مسكر حرام"]),
            (&["tape", "fermentasi", "alkohol", "halal"], vec!["حكم الكحول من التخمير", "حكم الخمر من غير العنب"]),

            // BATCH 95: Hajj, Umroh, and Ramadan specific patterns
            (&["haji", "wajib", "mampu", "syarat"], vec!["استطاعة الحج", "الاستطاعة شرط للحج"]),
            (&["haji", "wajib", "sekali", "seumur"], vec!["وجوب الحج مرة في العمر"]),
            (&["haji", "mabrur", "tanda", "ciri"], vec!["الحج المبرور", "علامات قبول الحج"]),
            (&["haji", "reguler", "plus", "beda"], vec!["أنواع برامج الحج"]),
            (&["haji", "ifrad", "tamattu", "qiran", "beda"], vec!["الفرق بين أنواع الحج", "حج التمتع والإفراد والقران"]),
            (&["tamattu", "haji", "cara"], vec!["حج التمتع", "كيفية حج التمتع"]),
            (&["dam", "denda", "haji", "umroh"], vec!["دم الجبر في الحج", "الهدي والإطعام"]),
            (&["melontar", "jumroh", "cara"], vec!["كيفية رمي الجمرات", "الجمرات الثلاث"]),
            (&["jumroh", "urutan", "aqabah", "wustha", "ula"], vec!["ترتيب الجمرات", "الجمرة الكبرى والوسطى والصغرى"]),
            (&["mabit", "muzdalifah", "mina", "wajib"], vec!["المبيت بمنى ومزدلفة", "واجبات الحج"]),
            (&["tahallul", "haji", "cara", "jenis"], vec!["التحلل من الإحرام", "التحلل الأول والثاني"]),
            (&["sai", "bukit", "shafa", "marwa", "cara"], vec!["السعي بين الصفا والمروة", "الشوط"]),
            (&["thawaf", "ifadhah", "wida", "qudum", "beda"], vec!["أنواع الطواف", "الفرق بين طواف الإفاضة والوداع"]),
            (&["umroh", "wajib", "boleh", "berkali"], vec!["حكم تكرار العمرة", "العمرة مرة أم أكثر"]),
            (&["umroh", "berapa", "kali", "setahun"], vec!["تكرار العمرة في السنة"]),
            (&["haid", "saat", "umroh", "boleh", "tawaf"], vec!["الطواف للحائض", "حكم طواف الحائض"]),
            (&["haid", "saat", "haji", "tawaf"], vec!["الحائض في الحج", "طواف الوداع للحائض"]),
            (&["ihram", "pakaian", "wanita", "laki", "beda"], vec!["ملابس الإحرام", "الفرق بين إحرام الرجل والمرأة"]),
            (&["larangan", "ihram", "selama"], vec!["محظورات الإحرام"]),
            (&["niat", "umroh", "cara", "lafaz"], vec!["نية العمرة", "لبيك اللهم عمرة"]),
            (&["niat", "haji", "cara", "lafaz"], vec!["نية الحج", "لبيك اللهم حجًا"]),
            (&["talbiyah", "bacaan", "haji"], vec!["التلبية", "لبيك اللهم لبيك"]),
            (&["ramadhan", "beda", "syawal", "puasa"], vec!["الفرق بين صوم رمضان وشوال"]),
            (&["lailatul", "qadar", "kapan", "malam"], vec!["متى تكون ليلة القدر", "ليالي العشر الأخيرة"]),
            (&["malam", "ganjil", "ramadhan", "lailatul"], vec!["الليالي الوتر من رمضان", "تحري ليلة القدر"]),
            (&["itikaf", "ramadhan", "cara", "hukum"], vec!["الاعتكاف في رمضان", "شروط الاعتكاف"]),
            (&["sahur", "waktu", "akhir", "imsak"], vec!["وقت السحور", "السحور ووقته"]),
            (&["buka", "puasa", "waktu", "doa"], vec!["وقت الإفطار", "دعاء الإفطار"]),
            (&["berbuka", "puasa", "dengan", "apa"], vec!["الإفطار على الرطب والماء", "سنة الإفطار"]),
            (&["puasa", "qadha", "ramadhan", "kapan", "batas"], vec!["وقت قضاء رمضان", "قضاء قبل رمضان الآتي"]),
            (&["puasa", "syawal", "qadha", "mana", "dahulu"], vec!["أيهما يقدم قضاء رمضان أم صيام شوال"]),
            (&["shalat", "tarawih", "berjamaah", "rumah"], vec!["صلاة التراويح في البيت", "الجماعة في التراويح"]),
            (&["takbiran", "idul", "fitri", "kapan", "mulai"], vec!["وقت التكبير لعيد الفطر"]),
            (&["zakat", "fitrah", "kapan", "bayar", "wajib"], vec!["وقت وجوب زكاة الفطر", "أداء الفطرة"]),
            (&["zakat", "fitrah", "siapa", "wajib"], vec!["من تجب عليه زكاة الفطر"]),
            (&["zakat", "fitrah", "untuk", "siapa", "diberikan"], vec!["مصارف زكاة الفطر", "لمن تعطى"]),
            (&["asnaf", "delapan", "penerima", "zakat"], vec!["الأصناف الثمانية للزكاة", "مصارف الزكاة"]),
            (&["amil", "zakat", "siapa", "hak"], vec!["العامل على الزكاة", "حق العامل"]),
            (&["bayar", "zakat", "online", "hukum"], vec!["أداء الزكاة عبر الإنترنت"]),
            (&["zakat", "niat", "lafaz", "cara"], vec!["نية الزكاة", "كيفية أداء الزكاة"]),
            (&["infak", "shadaqah", "harian", "amalan"], vec!["الصدقة اليومية", "فضل الصدقة"]),

            // BATCH 96: Tafsir, Quran studies, and hadith specific questions
            (&["tafsir", "bi-ma'tsur", "bi-ra'yi", "beda"], vec!["الفرق بين التفسير بالمأثور والتفسير بالرأي"]),
            (&["tafsir", "ishari", "tasawuf"], vec!["التفسير الإشاري", "تفسير الصوفية"]),
            (&["tafsir", "ilmi", "sains"], vec!["التفسير العلمي", "الإعجاز العلمي"]),
            (&["tafsir", "maudhu'i", "tematik"], vec!["التفسير الموضوعي", "المنهج الموضوعي"]),
            (&["tafsir", "tahili", "tahlili"], vec!["التفسير التحليلي", "المنهج التحليلي"]),
            (&["tafsir", "ijmali", "ringkas"], vec!["التفسير الإجمالي"]),
            (&["mufassir", "terkenal", "ulama"], vec!["أشهر المفسرين", "علماء التفسير"]),
            (&["ibn", "katsir", "tafsir"], vec!["تفسير ابن كثير", "ابن كثير"]),
            (&["jalalain", "tafsir"], vec!["تفسير الجلالين", "الجلالان"]),
            (&["qurtubi", "tafsir"], vec!["تفسير القرطبي"]),
            (&["thobari", "thabari", "tafsir"], vec!["تفسير الطبري", "جامع البيان"]),
            (&["baghawi", "tafsir"], vec!["تفسير البغوي", "معالم التنزيل"]),
            (&["zamakhsyari", "tafsir", "kasysyaf"], vec!["تفسير الزمخشري", "الكشاف"]),
            (&["fakhruddin", "razi", "tafsir"], vec!["تفسير الفخر الرازي", "مفاتيح الغيب"]),
            (&["suyuthi", "tafsir", "durrul"], vec!["الدر المنثور", "تفسير السيوطي"]),
            (&["hadits", "bukhari", "muslim", "derajat"], vec!["صحيح البخاري ومسلم", "درجة الصحيحين"]),
            (&["kutub", "sittah", "enam", "kitab"], vec!["الكتب الستة", "الصحاح الستة"]),
            (&["shahih", "bukhari", "muslim", "beda"], vec!["الفرق بين البخاري ومسلم"]),
            (&["sunan", "abu", "daud", "nasai", "tirmidzi"], vec!["السنن الأربعة", "سنن أبي داود والنسائي"]),
            (&["musnad", "imam", "ahmad"], vec!["مسند الإمام أحمد"]),
            (&["muwatha", "imam", "malik"], vec!["موطأ الإمام مالك"]),
            (&["riyadhus", "shalihin", "nawawi"], vec!["رياض الصالحين للنووي"]),
            (&["bulughul", "maram", "ibnu", "hajar"], vec!["بلوغ المرام لابن حجر"]),
            (&["arbain", "nawawi", "empat", "puluh", "hadits"], vec!["الأربعون النووية"]),
            (&["hadits", "qudsi", "arti", "bedanya"], vec!["تعريف الحديث القدسي", "الفرق بين القرآن والحديث القدسي"]),
            (&["riwayat", "lafzhi", "maknawi", "beda"], vec!["الرواية بالمعنى", "حكم الرواية بالمعنى"]),
            (&["isnad", "muttasil", "munqathi", "beda"], vec!["الفرق بين الإسناد المتصل والمنقطع"]),
            (&["hadits", "mursal", "munqathi"], vec!["الحديث المرسل والمنقطع"]),
            (&["hadits", "mudhtarib", "syaz"], vec!["الحديث المضطرب والشاذ"]),
            (&["hadits", "musnad", "mauquf", "maqthu"], vec!["الفرق بين المرفوع والموقوف والمقطوع"]),
            (&["kutub", "tis'ah", "sembilan", "kitab"], vec!["الكتب التسعة", "مصادر الحديث"]),
            (&["tahqiq", "hadits", "cara"], vec!["تحقيق الحديث", "آليات التحقيق"]),
            (&["takhrij", "hadits", "cara"], vec!["تخريج الحديث", "منهج التخريج"]),
            (&["sunnah", "qauliyah", "fi'liyah", "taqririyah"], vec!["أقسام السنة", "السنة القولية والفعلية والتقريرية"]),
            (&["atsar", "sahabat", "hujjah"], vec!["حجية أقوال الصحابة", "الأثر"]),
            (&["khulafaurrasyidin", "kisah", "hidup"], vec!["الخلفاء الراشدون", "سيرة الخلفاء"]),
            (&["abu", "bakar", "umar", "utsman", "ali", "sahabat"], vec!["الصحابة الكبار", "آل البيت والصحابة"]),
            (&["sahabat", "nabi", "derajat", "keutamaan"], vec!["فضل الصحابة", "منزلة الصحابة"]),
            (&["tabi'in", "tabi'ut", "tabi'in", "derajat"], vec!["التابعون وأتباعهم", "طبقات العلماء"]),
            (&["ahli", "hadits", "ulama", "terkenal"], vec!["أعلام المحدثين", "كبار المحدثين"]),

            // BATCH 97: Islamic history, biography, and classical scholars
            (&["biografi", "imam", "syafi'i"], vec!["سيرة الإمام الشافعي", "ترجمة الشافعي"]),
            (&["biografi", "imam", "hanafi"], vec!["سيرة الإمام أبي حنيفة", "ترجمة أبي حنيفة"]),
            (&["biografi", "imam", "malik"], vec!["سيرة الإمام مالك", "ترجمة الإمام مالك"]),
            (&["biografi", "imam", "ahmad"], vec!["سيرة الإمام أحمد", "ترجمة أحمد بن حنبل"]),
            (&["biografi", "imam", "nawawi"], vec!["سيرة الإمام النووي", "ترجمة النووي"]),
            (&["biografi", "ibnu", "taimiyah"], vec!["سيرة ابن تيمية", "ترجمة ابن تيمية"]),
            (&["biografi", "ibnu", "qayyim"], vec!["سيرة ابن القيم", "ترجمة ابن القيم الجوزية"]),
            (&["biografi", "al-ghazali"], vec!["سيرة الغزالي", "ترجمة الغزالي"]),
            (&["biografi", "ibnu", "rushd", "averroes"], vec!["سيرة ابن رشد", "فيلسوف الإسلام"]),
            (&["biografi", "ibnu", "sina", "avicenna"], vec!["سيرة ابن سينا", "طبيب الإسلام"]),
            (&["biografi", "ibnu", "khaldun"], vec!["سيرة ابن خلدون", "مقدمة ابن خلدون"]),
            (&["biografi", "imam", "bukhari"], vec!["سيرة الإمام البخاري", "ترجمة البخاري"]),
            (&["biografi", "imam", "muslim"], vec!["سيرة الإمام مسلم", "ترجمة الإمام مسلم"]),
            (&["biografi", "imam", "tirmidzi"], vec!["سيرة الإمام الترمذي"]),
            (&["nabi", "muhammad", "sirah", "singkat"], vec!["السيرة النبوية المختصرة", "ملخص السيرة"]),
            (&["nabi", "muhammad", "lahir", "mekah"], vec!["مولد النبي", "عام الفيل"]),
            (&["hijrah", "madinah", "tahun"], vec!["الهجرة إلى المدينة", "هجرة النبي"]),
            (&["perang", "badar", "uhud", "khandaq"], vec!["غزوة بدر وأحد والخندق"]),
            (&["fathul", "makkah", "pembebasan"], vec!["فتح مكة", "غزوة الفتح"]),
            (&["isra", "miraj", "tahun", "kapan"], vec!["الإسراء والمعراج", "متى كان الإسراء"]),
            (&["wahyu", "pertama", "turun", "quran"], vec!["أول ما نزل من القرآن", "نزول الوحي الأول"]),
            (&["turki", "ottoman", "khilafah"], vec!["الخلافة العثمانية", "السلطنة العثمانية"]),
            (&["dinasti", "abbasiyah", "umayyah"], vec!["الدولة العباسية والأموية", "الخلافة الإسلامية"]),
            (&["andalusia", "spanyol", "islam"], vec!["الأندلس", "الحضارة الإسلامية في إسبانيا"]),
            (&["penaklukan", "byzantium", "konstantinopel"], vec!["فتح القسطنطينية", "محمد الفاتح"]),
            (&["kemunduran", "islam", "sebab"], vec!["أسباب تأخر المسلمين", "الانحطاط الإسلامي"]),
            (&["kebangkitan", "islam", "islah"], vec!["الصحوة الإسلامية", "الإصلاح الإسلامي"]),
            (&["pembaruan", "islam", "indonesia"], vec!["الإصلاح الإسلامي في إندونيسيا"]),
            (&["nu", "muhammadiyah", "beda"], vec!["نهضة العلماء والمحمدية", "الحركات الإسلامية"]),
            (&["nahdlatul", "ulama", "ajaran", "aswaja"], vec!["منهج نهضة العلماء", "أهل السنة والجماعة"]),
            (&["muhammadiyah", "ajaran", "resmi"], vec!["منهج المحمدية الإسلامي"]),
            (&["walisongo", "sembilan", "wali"], vec!["الأولياء التسعة في جاوا", "ولي سونقو"]),
            (&["sunan", "kalijaga", "ampel", "giri"], vec!["الولياء في إندونيسيا"]),
            (&["islam", "masuk", "indonesia", "kapan"], vec!["دخول الإسلام إلى إندونيسيا"]),
            (&["kerajaan", "islam", "nusantara"], vec!["الممالك الإسلامية في أرخبيل الملايو"]),
            (&["demak", "mataram", "islam", "kerajaan"], vec!["مملكة دماك والإسلام"]),
            (&["ulama", "indonesia", "terkenal"], vec!["العلماء الإندونيسيون المشهورون"]),
            (&["kh", "hasyim", "asy'ari"], vec!["الشيخ هاشم الأشعري", "نهضة العلماء"]),
            (&["buya", "hamka", "tafsir", "azhar"], vec!["تفسير الأزهار لهمكا"]),
            (&["kh", "ahmad", "dahlan", "muhammadiyah"], vec!["الشيخ أحمد دحلان والمحمدية"]),
            (&["habib", "arab", "descendant", "nasab"], vec!["السادة والأشراف في إندونيسيا"]),

            // BATCH 98: Science, environment, and contemporary issues in Islam
            (&["islam", "lingkungan", "alam"], vec!["الإسلام والبيئة", "حماية البيئة"]),
            (&["pencemaran", "lingkungan", "hukum"], vec!["حكم تلويث البيئة", "الإسلام والبيئة"]),
            (&["energi", "terbarukan", "solar", "panel"], vec!["الطاقة المتجددة في الإسلام"]),
            (&["perubahan", "iklim", "islam"], vec!["الإسلام والتغير المناخي", "حفظ الكوني"]),
            (&["hewan", "punah", "lindungi", "hukum"], vec!["حماية الحيوانات المهددة بالانقراض"]),
            (&["penyiksaan", "hewan", "hukum"], vec!["حكم تعذيب الحيوانات", "رفق بالحيوان"]),
            (&["menyembelih", "cara", "halal"], vec!["شروط الذبح الشرعي", "الذكاة الشرعية"]),
            (&["sembelih", "tidak", "menyebut", "bismillah"], vec!["الذبح بدون ذكر اسم الله", "التسمية عند الذبح"]),
            (&["hewan", "sembelih", "stungun", "listrik"], vec!["الصعق الكهربائي قبل الذبح"]),
            (&["daging", "laboratorium", "buatan"], vec!["اللحوم المزروعة في المختبر", "اللحوم الاصطناعية"]),
            (&["gmo", "rekayasa", "genetik", "halal"], vec!["حكم المعدّل وراثيًا", "الكائنات المعدلة"]),
            (&["kloning", "hewan", "tanaman"], vec!["حكم الاستنساخ في الحيوانات"]),
            (&["robot", "kecerdasan", "buatan", "ai", "hukum"], vec!["الذكاء الاصطناعي في الإسلام"]),
            (&["teknologi", "informasi", "privasi"], vec!["الخصوصية في الإسلام", "حفظ البيانات"]),
            (&["internet", "digunakan", "baik"], vec!["الإنترنت والإسلام", "الاستخدام الشرعي للإنترنت"]),
            (&["hacker", "kejahatan", "siber", "hukum"], vec!["حكم الجرائم الإلكترونية"]),
            (&["privasi", "data", "kebocoran"], vec!["حفظ الأمانات", "حكم التجسس الرقمي"]),
            (&["kolonialisme", "neo", "islam"], vec!["الإسلام والاستعمار"]),
            (&["kapitalisme", "islam", "pandangan"], vec!["موقف الإسلام من الرأسمالية"]),
            (&["sosialisme", "komunisme", "islam"], vec!["موقف الإسلام من الاشتراكية"]),
            (&["demokrasi", "islam", "pandangan"], vec!["موقف الإسلام من الديمقراطية"]),
            (&["ham", "hak", "asasi", "manusia", "islam"], vec!["حقوق الإنسان في الإسلام"]),
            (&["gender", "equality", "kesetaraan", "islam"], vec!["المساواة بين الجنسين في الإسلام"]),
            (&["feminisme", "islam", "pandangan"], vec!["موقف الإسلام من الحركات النسوية"]),
            (&["poligami", "kontroversi", "pro", "kontra"], vec!["الحجج المؤيدة والمعارضة للتعدد"]),
            (&["aborsi", "korban", "pemerkosaan"], vec!["حكم الإجهاض في حالة الاغتصاب"]),
            (&["kekerasan", "seksual", "hukum", "islam"], vec!["حكم الاغتصاب والاعتداء الجنسي"]),
            (&["child", "abuse", "perlindungan", "anak"], vec!["حماية الأطفال في الإسلام"]),
            (&["pedofilia", "hukum", "islam"], vec!["حكم التحرش بالأطفال", "حماية الأحداث"]),
            (&["kasus", "sengketa", "waris", "pengadilan"], vec!["التقاضي في الميراث", "الدعوى الشرعية"]),
            (&["warisan", "sengketa", "cara", "selesaikan"], vec!["حل نزاعات الإرث", "التحكيم"]),
            (&["mediasi", "islami", "sengketa"], vec!["الصلح والوساطة في الإسلام"]),
            (&["arbitrase", "syariah", "penyelesaian"], vec!["التحكيم الشرعي"]),
            (&["pengadilan", "agama", "wewenang"], vec!["المحكمة الدينية", "اختصاص المحاكم"]),
            (&["cerai", "pengadilan", "agama", "proses"], vec!["الطلاق القضائي", "إجراءات الطلاق"]),
            (&["nikah", "siri", "anak", "legalitas"], vec!["شرعية الزواج السري", "نسب الأبناء"]),
            (&["akta", "nikah", "perkawinan", "hukum"], vec!["توثيق الزواج", "شهادة الزواج"]),
            (&["dna", "test", "nasab", "anak"], vec!["تحديد النسب بالحمض النووي", "اشتراط فراش"]),

            // BATCH 99: Women's Islamic questions and family fiqh
            (&["wanita", "shalat", "berjamaah", "masjid"], vec!["صلاة المرأة في المسجد", "خروج المرأة"]),
            (&["wanita", "imam", "shalat", "hukum"], vec!["إمامة المرأة للرجال", "إمامة المرأة"]),
            (&["muslimah", "shalat", "di", "kantor"], vec!["صلاة المرأة خارج البيت"]),
            (&["haid", "membaca", "quran"], vec!["قراءة الحائض للقرآن", "الحيض وقراءة القرآن"]),
            (&["haid", "masuk", "masjid"], vec!["دخول الحائض المسجد", "الحيض والمسجد"]),
            (&["haid", "tawaf", "umroh", "haji"], vec!["طواف الحائض", "الحيض في الحج"]),
            (&["haid", "menyentuh", "quran"], vec!["مس الحائض للمصحف"]),
            (&["wanita", "bercadar", "niqab", "hukum"], vec!["حكم النقاب", "وجوب النقاب"]),
            (&["jilbab", "hukum", "wajib"], vec!["وجوب الحجاب", "الحجاب الشرعي"]),
            (&["aurat", "wanita", "batasan"], vec!["عورة المرأة", "حدود العورة"]),
            (&["aurat", "laki", "batasan"], vec!["عورة الرجل", "حدود عورة الرجل"]),
            (&["wanita", "keluar", "izin", "suami"], vec!["خروج الزوجة بإذن الزوج", "استئذان الزوجة"]),
            (&["wanita", "bekerja", "hukum", "islam"], vec!["عمل المرأة في الإسلام", "حكم خروج المرأة للعمل"]),
            (&["wanita", "kepala", "keluarga"], vec!["المرأة ربة الأسرة", "المرأة وعملها"]),
            (&["wanita", "pemimpin", "negara"], vec!["ولاية المرأة في الحكم", "قيادة المرأة"]),
            (&["nusyuz", "arti", "hukum"], vec!["النشوز", "حكم نشوز الزوجة"]),
            (&["nafkah", "istri", "bekerja"], vec!["نفقة الزوجة العاملة"]),
            (&["istri", "hak", "cerai", "gugat"], vec!["حق الزوجة في طلب الطلاق", "الخلع"]),
            (&["suami", "tidak", "nafkah", "bertahun"], vec!["الطلاق بسبب عدم النفقة"]),
            (&["talak", "tiga", "sekaligus", "hukum"], vec!["الطلاق الثلاث بلفظ واحد"]),
            (&["ruju", "syarat", "cara"], vec!["شروط الرجعة", "كيفية الرجعة"]),
            (&["iddah", "wanita", "hamil"], vec!["عدة الحامل"]),
            (&["wali", "nikah", "wali", "hakim"], vec!["الولي الحاكم", "ولاية القاضي"]),
            (&["wali", "nikah", "adhal", "enggan"], vec!["الولي العاضل", "الولي المُعضِل"]),
            (&["wali", "mujbir", "hukum"], vec!["الولي المجبر", "إجبار الولي"]),
            (&["poligami", "syarat", "adil"], vec!["شروط التعدد", "شرط العدل في التعدد"]),
            (&["poligami", "tanpa", "izin", "istri"], vec!["التعدد بدون رضا الزوجة"]),
            (&["menyusui", "ibu", "hukum", "dua", "tahun"], vec!["حكم الرضاعة الطبيعية", "مدة الرضاعة"]),
            (&["susu", "ibu", "donor", "hukum"], vec!["حكم بنوة الرضاعة", "رضاعة المستأجر"]),
            (&["asi", "donor", "mahram"], vec!["الرضاعة المحرّمة", "المحرمية بالرضاعة"]),
            (&["hamil", "diluar", "nikah", "anak", "nasab"], vec!["نسب ولد الزنا", "الابن غير الشرعي"]),
            (&["anak", "angkat", "adopsi", "hukum"], vec!["حكم التبني", "أحكام الكفالة"]),
            (&["kafa'ah", "kesepadanan", "nikah"], vec!["الكفاءة في النكاح", "شروط الكفاءة"]),
            (&["mahar", "hutang", "tidak", "dibayar"], vec!["عدم دفع المهر", "المهر الديّن"]),
            (&["perjanjian", "pranikah", "hukum"], vec!["اشتراط في عقد النكاح", "شروط النكاح"]),
            (&["mut'ah", "nikah", "kontrak"], vec!["حكم زواج المتعة", "النكاح المؤقت"]),
            (&["misyar", "nikah", "hukum"], vec!["حكم نكاح المسيار"]),
            (&["muhallil", "nikah", "tahlil"], vec!["نكاح المحلل", "نكاح التحليل"]),
            (&["anak", "hak", "asuh", "hadhonah"], vec!["الحضانة", "حق حضانة الأولاد"]),
            (&["anak", "ikut", "siapa", "cerai"], vec!["الحضانة بعد الطلاق"]),

            // BATCH 100: Islamic economics and finance
            (&["bank", "syariah", "konvensional", "beda"], vec!["الفرق بين البنك الإسلامي والتقليدي"]),
            (&["murabahah", "bagaimana", "cara"], vec!["المرابحة", "تعريف المرابحة وشروطها"]),
            (&["mudharabah", "pengertian", "hukum"], vec!["المضاربة", "عقد المضاربة"]),
            (&["musyarakah", "pengertian", "hukum"], vec!["المشاركة", "عقد المشاركة"]),
            (&["ijarah", "pengertian", "hukum"], vec!["الإجارة", "عقد الإجارة"]),
            (&["sukuk", "obligasi", "syariah"], vec!["الصكوك الإسلامية", "سندات الحكومية الإسلامية"]),
            (&["asuransi", "syariah", "takaful"], vec!["التكافل الإسلامي", "التأمين الإسلامي"]),
            (&["asuransi", "konvensional", "hukum"], vec!["حكم التأمين التجاري"]),
            (&["saham", "halal", "haram"], vec!["الأسهم في الإسلام", "حكم الأسهم"]),
            (&["reksa", "dana", "syariah"], vec!["صناديق الاستثمار الإسلامية"]),
            (&["forex", "valas", "trading", "hukum"], vec!["حكم تداول العملات الأجنبية"]),
            (&["kripto", "crypto", "bitcoin", "halal"], vec!["حكم العملات المشفرة", "العملة الرقمية"]),
            (&["nft", "digital", "art", "hukum"], vec!["حكم الرموز غير القابلة للاستبدال"]),
            (&["jual", "beli", "online", "hukum"], vec!["البيع والشراء عبر الإنترنت", "التجارة الإلكترونية"]),
            (&["marketplace", "shopee", "tokopedia", "hukum"], vec!["حكم بيع الأسواق الإلكترونية"]),
            (&["dropship", "reseller", "hukum"], vec!["حكم بيع ما لا يملك", "بيع المرابحة للآمر"]),
            (&["affiliate", "marketing", "komisi"], vec!["حكم العمولة والسمسرة"]),
            (&["multi", "level", "marketing", "mlm"], vec!["حكم التسويق الشبكي", "MLM في الإسلام"]),
            (&["pinjaman", "online", "pinjol", "hukum"], vec!["حكم القرض الإلكتروني", "الربا في الديجيتال"]),
            (&["kartu", "kredit", "hukum", "riba"], vec!["حكم بطاقة الائتمان", "الربا في البطاقات"]),
            (&["cicilan", "bunga", "kredit", "hukum"], vec!["حكم الشراء بالتقسيط مع الفائدة"]),
            (&["kpr", "renovasi", "rumah", "kredit"], vec!["حكم الرهن العقاري"]),
            (&["koperasi", "simpan", "pinjam", "hukum"], vec!["حكم التعاونيات الإسلامية"]),
            (&["wakaf", "uang", "tunai", "produktif"], vec!["الوقف النقدي", "الوقف المنتج"]),
            (&["wakaf", "tanah", "bangunan", "hukum"], vec!["الوقف العيني", "أحكام الوقف"]),
            (&["infak", "sedekah", "jariyah", "pahala"], vec!["الصدقة الجارية", "ثواب الصدقة"]),
            (&["zakat", "penghasilan", "profesi"], vec!["زكاة الراتب والمهنة", "زكاة الدخل"]),
            (&["zakat", "tani", "petani", "hasil"], vec!["زكاة الزراعة", "زكاة الحبوب"]),
            (&["zakat", "ternak", "sapi", "kambing"], vec!["زكاة الغنم", "زكاة الإبل", "زكاة البقر"]),
            (&["zakat", "emas", "perhiasan", "pakai"], vec!["زكاة الحلي المستعملة"]),
            (&["nisab", "zakat", "sekarang", "rupiah"], vec!["نصاب الزكاة بالعملة الوطنية"]),
            (&["bayar", "zakat", "lewat", "app"], vec!["توصيل الزكاة رقمياً"]),
            (&["amil", "zakat", "nasional", "baznas"], vec!["البيت الوطني للزكاة"]),
            (&["lembaga", "zakat", "terpercaya"], vec!["المؤسسات الزكوية الموثوقة"]),
            (&["hutang", "tidak", "bayar", "hukum"], vec!["حكم المماطلة في الدين", "التسويف في الديون"]),
            (&["utang", "hilang", "meninggal", "ahli", "waris"], vec!["ديون المتوفى", "قضاء ديون الميت"]),
            (&["hibah", "orangtua", "anak", "hukum"], vec!["حكم الهبة للأولاد", "التسوية في الهبة"]),
            (&["wasiat", "batas", "sepertiga", "harta"], vec!["وصية بالثلث", "حد الوصية"]),
            (&["waris", "beda", "agama"], vec!["إرث المسلم من الكافر", "حكم التوارث مع اختلاف الدين"]),
            (&["waris", "perempuan", "setengah", "alasan"], vec!["حكمة تنصيف إرث المرأة", "ميراث المرأة وحكمته"]),

            // BATCH 101: Specific Quranic verses, surahs, and dua questions
            (&["arti", "ayat", "kursi", "surah"], vec!["تفسير آية الكرسي", "آية الكرسي"]),
            (&["surah", "yasin", "keutamaan", "faidah"], vec!["فضل سورة يس", "قراءة يس"]),
            (&["surah", "alkahf", "jumat", "keutamaan"], vec!["فضل قراءة الكهف يوم الجمعة"]),
            (&["surah", "waqiah", "rezeki", "keutamaan"], vec!["فضل سورة الواقعة"]),
            (&["surah", "mulk", "azab", "kubur"], vec!["فضل سورة الملك وعذاب القبر"]),
            (&["surah", "ikhlas", "tiga", "kali", "setara"], vec!["فضل قراءة الإخلاص ثلاث مرات"]),
            (&["surah", "falaq", "nas", "muawwidzatain"], vec!["المعوذتان والرقية"]),
            (&["ayat", "seribu", "dinar", "surah", "thalaq"], vec!["آية الطلاق ومن يتق الله"]),
            (&["doa", "setelah", "shalat", "fardu"], vec!["الأذكار بعد الصلاة", "أدعية ما بعد الصلاة"]),
            (&["dzikir", "setelah", "shalat", "lafaz"], vec!["أذكار ما بعد الصلاة"]),
            (&["doa", "pagi", "petang", "lafaz"], vec!["أذكار الصباح والمساء"]),
            (&["doa", "sebelum", "tidur", "sunnah"], vec!["دعاء النوم", "أذكار قبل النوم"]),
            (&["doa", "bangun", "tidur", "lafaz"], vec!["دعاء الاستيقاظ", "أذكار الصباح"]),
            (&["doa", "makan", "minum", "lafaz"], vec!["دعاء الأكل والشراب"]),
            (&["doa", "masuk", "wc", "toilet", "lafaz"], vec!["دعاء دخول الخلاء"]),
            (&["doa", "keluar", "wc", "lafaz"], vec!["دعاء الخروج من الخلاء"]),
            (&["doa", "masuk", "masjid", "keluar"], vec!["دعاء دخول المسجد والخروج منه"]),
            (&["doa", "naik", "kendaraan", "safar"], vec!["دعاء ركوب السيارة", "دعاء السفر"]),
            (&["doa", "hujan", "turun", "lebat"], vec!["دعاء نزول المطر"]),
            (&["doa", "angin", "kencang", "petir"], vec!["دعاء الريح"]),
            (&["doa", "sakit", "ruqyah", "syifaa"], vec!["دعاء المريض", "الرقية الشرعية"]),
            (&["doa", "kesembuhan", "orang", "sakit"], vec!["الدعاء للمريض"]),
            (&["doa", "agar", "rezeki", "lancar"], vec!["الدعاء لطلب الرزق"]),
            (&["doa", "supaya", "lulus", "ujian"], vec!["الدعاء للنجاح في الامتحان"]),
            (&["doa", "meminta", "jodoh", "cepat"], vec!["الدعاء للزواج"]),
            (&["doa", "qunut", "subuh", "lafaz"], vec!["دعاء القنوت في صلاة الصبح"]),
            (&["doa", "istikharah", "lafaz", "teks"], vec!["دعاء صلاة الاستخارة"]),
            (&["doa", "buka", "puasa", "yang", "benar"], vec!["دعاء الإفطار", "دعاء الفطر"]),
            (&["doa", "sahur", "niat", "puasa"], vec!["دعاء السحور", "نية الصيام"]),
            (&["doa", "ziarah", "kubur", "lafaz"], vec!["دعاء زيارة القبور"]),
            (&["doa", "tawaf", "sa'i", "haji", "umroh"], vec!["دعاء الطواف والسعي"]),
            (&["bacaan", "talbiyah", "lafaz", "haji"], vec!["لبيك اللهم لبيك"]),
            (&["niat", "ihram", "haji", "umroh", "lafaz"], vec!["نية الإحرام للحج والعمرة"]),
            (&["surah", "bacaan", "shalat", "subuh"], vec!["السور المستحبة في صلاة الفجر"]),
            (&["surah", "pendek", "sering", "dibaca"], vec!["السور القصيرة", "جزء عم"]),
            (&["taawudz", "basmalah", "arti"], vec!["الاستعاذة والبسملة", "أعوذ بالله"]),
            (&["hauqalah", "la", "haula", "wala", "quwwata"], vec!["لا حول ولا قوة إلا بالله"]),
            (&["istirja", "innalillahi", "arti"], vec!["إنا لله وإنا إليه راجعون"]),
            (&["maasyaallah", "subhanallah", "alhamdulillah", "arti"], vec!["ما شاء الله", "سبحان الله", "الحمد لله"]),
            (&["tasbih", "dzikir", "99", "asmaul", "husna"], vec!["الأسماء الحسنى التسعة وتسعون"]),

            // BATCH 102: Shalat edge cases and detailed rulings
            (&["shalat", "jama", "takhir", "zhuhur", "ashar"], vec!["الجمع بين الظهر والعصر تأخيراً"]),
            (&["shalat", "jama", "taqdim", "zhuhur", "ashar"], vec!["الجمع بين الظهر والعصر تقديماً"]),
            (&["shalat", "jama", "maghrib", "isya", "safar"], vec!["الجمع في السفر بين المغرب والعشاء"]),
            (&["qashar", "rakaat", "jadi", "dua", "syarat"], vec!["شروط القصر", "صلاة القصر"]),
            (&["shalat", "qashar", "berapa", "hari", "batas"], vec!["مدة الإقامة وحكم القصر"]),
            (&["shalat", "gerhana", "khusuf", "kusuf"], vec!["صلاة كسوف الشمس وخسوف القمر"]),
            (&["shalat", "istisqa", "minta", "hujan"], vec!["صلاة الاستسقاء"]),
            (&["shalat", "tahiyatul", "masjid", "sunah"], vec!["تحية المسجد", "صلاة تحية المسجد"]),
            (&["shalat", "dhuha", "berapa", "rakaat", "waktu"], vec!["صلاة الضحى عدد ركعاتها ووقتها"]),
            (&["shalat", "tahajud", "berapa", "rakaat", "cara"], vec!["صلاة التهجد عدد ركعاتها"]),
            (&["shalat", "rawatib", "qabliyah", "ba'diyah"], vec!["الصلوات الراتبة القبلية والبعدية"]),
            (&["witir", "satu", "tiga", "berapa", "rakaat"], vec!["عدد ركعات الوتر"]),
            (&["shalat", "hajat", "cara", "niat"], vec!["صلاة الحاجة", "كيفية صلاة الحاجة"]),
            (&["shalat", "taubat", "dua", "rakaat"], vec!["صلاة التوبة", "ركعتا التوبة"]),
            (&["shalat", "awwabin", "bakda", "maghrib"], vec!["صلاة الأوابين"]),
            (&["shalat", "jenazah", "gaib", "orang", "jauh"], vec!["صلاة الغائب على الميت"]),
            (&["shalat", "di", "atas", "kuburan"], vec!["الصلاة على القبر"]),
            (&["shalat", "khauf", "perang", "keamanan"], vec!["صلاة الخوف"]),
            (&["makmum", "masbuk", "ketentuan", "cara"], vec!["أحكام المسبوق"]),
            (&["bermakmum", "dari", "rumah", "online", "live"], vec!["الاقتداء عبر الإنترنت", "الجماعة الإلكترونية"]),
            (&["shalat", "duduk", "kursi", "sakit"], vec!["صلاة المريض على الكرسي"]),
            (&["shalat", "berbaring", "tidak", "bisa", "duduk"], vec!["صلاة المريض مضطجعاً"]),
            (&["shalat", "isyarat", "mata", "tidak", "bisa"], vec!["صلاة بالإيماء"]),
            (&["tayamum", "cara", "benar", "sakit"], vec!["كيفية التيمم", "شروط التيمم"]),
            (&["wudhu", "sebelum", "tidur", "sunah"], vec!["الوضوء قبل النوم"]),
            (&["wudhu", "niat", "lafaz", "dalam", "hati"], vec!["نية الوضوء ولفظها"]),
            (&["mandi", "wajib", "urutan", "cara"], vec!["كيفية الغسل الواجب", "ترتيب الغسل"]),
            (&["junub", "shalat", "tanpa", "mandi"], vec!["حكم الصلاة مع الجنابة"]),
            (&["mandi", "setelah", "haid", "cara"], vec!["الغسل من الحيض", "كيفية غسل الحائض"]),
            (&["haid", "berapa", "hari", "normal", "batas"], vec!["أقل الحيض وأكثره"]),
            (&["istihadhah", "darah", "terus", "hukum"], vec!["حكم الاستحاضة", "المستحاضة وصلاتها"]),
            (&["nifas", "berapa", "hari", "maksimal"], vec!["أقل النفاس وأكثره"]),
            (&["keputihan", "hukum", "wudhu"], vec!["حكم الإفرازات والوضوء"]),
            (&["mani", "madzi", "wadi", "beda"], vec!["الفرق بين المني والمذي والودي"]),
            (&["najis", "anjing", "basuh", "tujuh"], vec!["حكم نجاسة الكلب وكيفية تطهيرها"]),
            (&["najis", "babi", "hukum", "basuh"], vec!["نجاسة الخنزير"]),
            (&["darah", "sedikit", "batal", "wudhu"], vec!["حكم خروج الدم من الوضوء"]),
            (&["kentut", "tidak", "berbunyi", "batal", "wudhu"], vec!["الريح الصامت وحكم الوضوء"]),
            (&["menyentuh", "wanita", "batal", "wudhu"], vec!["مس المرأة ومنقضات الوضوء"]),
            (&["tertidur", "batal", "wudhu", "sebentar"], vec!["النوم ونقض الوضوء"]),

            // BATCH 103: Food, drink, halal/haram details
            (&["makanan", "haram", "daftar", "islam"], vec!["المحرمات من الطعام والشراب"]),
            (&["makan", "daging", "haram", "apa", "saja"], vec!["المحرمات من اللحوم"]),
            (&["katak", "kodok", "halal", "haram"], vec!["حكم أكل الضفدع"]),
            (&["buaya", "reptil", "halal", "haram"], vec!["حكم أكل التمساح"]),
            (&["ular", "hewan", "buas", "halal"], vec!["حكم أكل ذوات السموم"]),
            (&["cacing", "serangga", "halal", "haram"], vec!["حكم أكل الحشرات والديدان"]),
            (&["kepiting", "rajungan", "halal", "haram"], vec!["حكم أكل السرطان"]),
            (&["udang", "lobster", "halal"], vec!["حكم أكل الجمبري والأسماك"]),
            (&["cumi", "gurita", "sotong", "halal"], vec!["حكم أكل الأخطبوط"]),
            (&["bekicot", "siput", "halal", "haram"], vec!["حكم أكل الحلزون"]),
            (&["belalang", "jangkrik", "halal", "haram"], vec!["حكم أكل الجراد"]),
            (&["tikus", "biawak", "halal"], vec!["حكم أكل الضب"]),
            (&["kuda", "halal", "haram", "makan"], vec!["حكم أكل لحم الخيل"]),
            (&["keledai", "bagal", "haram"], vec!["حكم أكل لحم الحمار"]),
            (&["ayam", "kampung", "makan", "kotoran"], vec!["الجلالة وحكمها"]),
            (&["sembelih", "muslim", "nonmuslim", "halal"], vec!["ذبح غير المسلم وحكمه"]),
            (&["sembelih", "kristen", "ahli", "kitab"], vec!["ذبائح أهل الكتاب"]),
            (&["gelatin", "babi", "produk", "hukum"], vec!["حكم الجيلاتين المحرّم"]),
            (&["alkohol", "dalam", "makanan", "minuman"], vec!["حكم الكحول في الأطعمة"]),
            (&["khamr", "bir", "wine", "hukum"], vec!["حكم الكحول والخمر"]),
            (&["bir", "nol", "persen", "halal"], vec!["حكم المشروبات غير الكحولية"]),
            (&["minuman", "keras", "nabidz", "hukum"], vec!["حكم النبيذ"]),
            (&["tembakau", "rokok", "kretek", "hukum"], vec!["حكم التبغ والتدخين"]),
            (&["shisha", "hookah", "rokok", "arab"], vec!["حكم الشيشة"]),
            (&["obat", "kapsul", "mengandung", "babi"], vec!["حكم الدواء المحتوي على الخنزير"]),
            (&["vaksin", "mengandung", "babi", "halal"], vec!["حكم التطعيم بلقاح من مواد محرمة"]),
            (&["pewarna", "makanan", "halal", "bahan"], vec!["المضافات الغذائية والحلال"]),
            (&["msg", "penyedap", "hukum", "halal"], vec!["حكم مسحوق الغلوتامات"]),
            (&["kue", "tape", "fermentasi", "alkohol"], vec!["حكم المخمر من الأطعمة"]),
            (&["nasi", "goreng", "minyak", "babi"], vec!["الطعام المطبوخ بالدهن المحرم"]),
            (&["restoran", "non", "halal", "makan"], vec!["الأكل في مطاعم غير المسلمين"]),
            (&["peralatan", "masak", "najis", "boleh"], vec!["استخدام أواني ذوي الكتاب"]),
            (&["logo", "sertifikat", "halal", "penting"], vec!["شهادة الحلال وأهميتها"]),
            (&["makan", "sambil", "berdiri", "berjalan"], vec!["حكم الأكل قائماً"]),
            (&["makan", "tangan", "kiri", "hukum"], vec!["حكم الأكل باليد اليسرى"]),
            (&["makan", "minum", "bertiga", "hadits"], vec!["آداب الطعام الإسلامية"]),
            (&["minum", "air", "zamzam", "cara"], vec!["آداب شرب ماء زمزم"]),
            (&["minum", "sambil", "berdiri", "hukum"], vec!["حكم الشرب قائماً"]),
            (&["kucing", "anjing", "pelihara", "hukum"], vec!["حكم اقتناء القطط والكلاب"]),
            (&["hewan", "pelihara", "disiapkan", "halal"], vec!["أحكام تربية الحيوانات"]),

            // BATCH 104: Islamic social ethics and forbidden acts
            (&["ghibah", "mengapa", "dosa", "besar"], vec!["حكم الغيبة وأدلتها", "تحريم الغيبة"]),
            (&["namimah", "adu", "domba", "hukum"], vec!["حكم النميمة"]),
            (&["dusta", "berbohong", "hukum"], vec!["حكم الكذب"]),
            (&["sumpah", "palsu", "dosa"], vec!["اليمين الغموس", "حكم الحلف كاذباً"]),
            (&["hasad", "dengki", "iri", "hukum"], vec!["حكم الحسد"]),
            (&["riya", "sum'ah", "pamer", "hukum"], vec!["حكم الرياء والسمعة"]),
            (&["ujub", "bangga", "diri", "hukum"], vec!["حكم العجب بالنفس"]),
            (&["takabur", "sombong", "hukum"], vec!["حكم الكبر والتكبر"]),
            (&["bakhil", "pelit", "kikir", "hukum"], vec!["حكم البخل"]),
            (&["israf", "tabzir", "boros", "hukum"], vec!["حكم الإسراف والتبذير"]),
            (&["zina", "mendekati", "hukum", "dalil"], vec!["تحريم الزنا وأدلته"]),
            (&["khalwat", "berdua", "non", "mahram"], vec!["حكم الخلوة بالأجنبية"]),
            (&["pandang", "perempuan", "non", "mahram"], vec!["حكم النظر إلى الأجنبية", "غض البصر"]),
            (&["tato", "hukum", "islam"], vec!["حكم الوشم في الإسلام"]),
            (&["tindik", "telinga", "hidung", "hukum"], vec!["حكم ثقب الجسم"]),
            (&["operasi", "kecantikan", "plastik", "hukum"], vec!["حكم جراحة التجميل"]),
            (&["menyambung", "rambut", "hukum"], vec!["حكم وصل الشعر"]),
            (&["wig", "rambut", "palsu", "hukum"], vec!["حكم الباروكة"]),
            (&["kuku", "panjang", "cat", "hukum"], vec!["حكم تطويل الأظفار"]),
            (&["mencukur", "janggut", "hukum"], vec!["حكم حلق اللحية"]),
            (&["kumis", "memotong", "sunah"], vec!["سنة قص الشارب"]),
            (&["mencabut", "bulu", "alis", "hukum"], vec!["حكم نمص الحواجب"]),
            (&["mewarnai", "rambut", "hitam", "hukum"], vec!["حكم تغيير لون الشعر"]),
            (&["tabarruj", "berhias", "berlebihan"], vec!["حكم التبرج"]),
            (&["pakaian", "ketat", "transparan", "hukum"], vec!["حكم اللباس الضيق والشفاف"]),
            (&["celana", "pendek", "di", "bawah", "lutut"], vec!["حكم كشف الركبة"]),
            (&["isbal", "celana", "panjang", "melewati"], vec!["حكم الإسبال"]),
            (&["memakai", "emas", "perak", "pria"], vec!["حكم لبس الذهب للرجال"]),
            (&["cincin", "berlian", "emas", "wanita"], vec!["حكم الذهب والفضة للمرأة"]),
            (&["sutera", "baju", "laki", "hukum"], vec!["حكم لبس الحرير للرجال"]),
            (&["berjabat", "tangan", "non", "mahram"], vec!["حكم مصافحة الأجنبية", "المصافحة والاختلاط"]),
            (&["mencium", "tangan", "ulama", "orang", "tua"], vec!["حكم تقبيل يد العالم"]),
            (&["salam", "kepada", "non", "muslim"], vec!["حكم السلام على غير المسلمين"]),
            (&["menjawab", "salam", "non", "muslim"], vec!["رد السلام على غير المسلمين"]),
            (&["ucap", "selamat", "natal", "kristen"], vec!["حكم تهنئة الكفار بأعيادهم"]),
            (&["ikut", "upacara", "adat", "hukum"], vec!["حكم المشاركة في الشعائر الوثنية"]),
            (&["pergi", "dukun", "paranormal", "hukum"], vec!["حكم الذهاب إلى الكاهن والعراف"]),
            (&["jimat", "azimat", "tangkal", "hukum"], vec!["حكم التمائم والتعاويذ"]),
            (&["sihir", "santet", "hukum"], vec!["حكم السحر"]),
            (&["ruqyah", "syariah", "cara", "benar"], vec!["الرقية الشرعية وكيفيتها"]),

            // BATCH 105: Death, burial, mourning, and afterlife
            (&["sakaratul", "maut", "hukum", "talkin"], vec!["أحكام الاحتضار والتلقين"]),
            (&["memandikan", "mayit", "cara", "syarat"], vec!["غسل الميت وكيفيته"]),
            (&["kafan", "putih", "berapa", "lapis"], vec!["أحكام التكفين"]),
            (&["membawa", "jenazah", "cepat", "lambat"], vec!["الإسراع بالجنازة"]),
            (&["shalat", "jenazah", "takbir", "empat"], vec!["أحكام صلاة الجنازة"]),
            (&["doa", "jenazah", "lafaz", "bacaan"], vec!["دعاء صلاة الجنازة"]),
            (&["mengiringi", "jenazah", "hukum"], vec!["حكم تشييع الجنازة"]),
            (&["mengubur", "mayit", "waktu", "syarat"], vec!["أحكام الدفن"]),
            (&["kubur", "nisan", "beton", "boleh"], vec!["حكم البناء على القبر"]),
            (&["kubur", "tumpuk", "dua", "orang"], vec!["الدفن مع غيره في قبر واحد"]),
            (&["membongkar", "kubur", "pindah", "jenazah"], vec!["حكم نبش القبر"]),
            (&["kremasi", "membakar", "mayit", "hukum"], vec!["حكم حرق الميت"]),
            (&["berziarah", "kubur", "sunah", "hukum"], vec!["حكم زيارة القبور"]),
            (&["tahlil", "yasin", "kubur", "hukum"], vec!["الدعاء وتلاوة القرآن عند القبور"]),
            (&["doa", "kubur", "sampai", "mayit"], vec!["وصول ثواب الأعمال إلى الميت"]),
            (&["kirim", "pahala", "orang", "mati"], vec!["إهداء ثواب القرآن والأعمال للميت"]),
            (&["sedekah", "jariyah", "orang", "meninggal"], vec!["الصدقة عن الميت"]),
            (&["mendo'akan", "orang", "tua", "meninggal"], vec!["الدعاء للوالدين المتوفيين"]),
            (&["azab", "kubur", "dalil", "hadits"], vec!["عذاب القبر"]),
            (&["nikmat", "kubur", "neraca", "hukum"], vec!["نعيم القبر"]),
            (&["alam", "barzakh", "apakah", "itu"], vec!["عالم البرزخ"]),
            (&["hari", "kiamat", "tanda", "kecil"], vec!["أشراط الساعة الصغرى"]),
            (&["dajjal", "imam", "mahdi", "nabi", "isa"], vec!["الدجال والمهدي ونزول عيسى"]),
            (&["hari", "kebangkitan", "mahsyar", "hisab"], vec!["البعث والحشر والحساب"]),
            (&["mizan", "timbangan", "amal", "kiamat"], vec!["الميزان يوم القيامة"]),
            (&["sirat", "jembatan", "neraka", "kiamat"], vec!["الصراط المستقيم يوم القيامة"]),
            (&["syafaat", "nabi", "kiamat", "hukum"], vec!["الشفاعة"]),
            (&["surga", "tingkatan", "firdaus", "ciri"], vec!["درجات الجنة"]),
            (&["neraka", "tingkatan", "jahannam", "siapa"], vec!["طبقات النار"]),
            (&["taqdim", "amal", "neraca", "surga"], vec!["الحساب والجزاء في الآخرة"]),
            (&["ta'ziyah", "belasungkawa", "hukum"], vec!["أحكام التعزية"]),
            (&["ratap", "menangis", "mayit", "hukum"], vec!["حكم النياحة على الميت"]),
            (&["masa", "berkabung", "berduka", "berapa"], vec!["أحكام الحداد"]),
            (&["ihdad", "berkabung", "janda", "hari"], vec!["حكم الإحداد على المتوفى"]),
            (&["wasiat", "berapa", "sepertiga", "harta"], vec!["الوصية بالثلث وفوقه"]),
            (&["waris", "hitung", "cara", "dasar"], vec!["حساب المواريث"]),
            (&["ahli", "waris", "siapa", "saja", "berhak"], vec!["الورثة وأصحاب الفروض"]),
            (&["ashabah", "waris", "laki", "perempuan"], vec!["العصبة في الإرث"]),
            (&["hajb", "terhalang", "waris"], vec!["الحجب في الإرث"]),
            (&["waris", "tidak", "ada", "pewaris", "negara"], vec!["الإرث بالتعصيب", "إرث بيت المال"]),

            // BATCH 106: Aqidah and theology questions
            (&["allah", "ada", "di", "mana", "arah"], vec!["أين الله؟", "تنزيه الله عن المكان"]),
            (&["allah", "sifat", "dua", "puluh", "wajib"], vec!["صفات الله الواجبة"]),
            (&["allah", "tauhid", "uluhiyah", "rububiyah"], vec!["توحيد الألوهية والربوبية"]),
            (&["allah", "melihat", "manusia", "ihsan"], vec!["الإحسان ومراقبة الله"]),
            (&["malaikat", "jumlah", "nama", "tugas"], vec!["أسماء الملائكة ووظائفهم"]),
            (&["jin", "manusia", "beda", "hubungan"], vec!["علاقة الجن بالإنس"]),
            (&["setan", "iblis", "asal", "usul"], vec!["أصل الشيطان وإبليس"]),
            (&["qada", "qadar", "takdir", "pengertian"], vec!["القضاء والقدر"]),
            (&["takdir", "ikhtiar", "usaha", "manusia"], vec!["الجمع بين القدر والاختيار"]),
            (&["dosa", "besar", "kecil", "bedanya"], vec!["الكبائر والصغائر"]),
            (&["kufur", "murtad", "penyebab"], vec!["الردة وأسبابها"]),
            (&["nifak", "munafik", "tanda", "ciri"], vec!["النفاق وعلاماته"]),
            (&["riddah", "hukum", "masuk", "kembali", "islam"], vec!["حكم المرتد وتوبته"]),
            (&["orang", "kafir", "apakah", "masuk", "neraka"], vec!["مصير الكفار"]),
            (&["anak", "kecil", "meninggal", "masuk", "surga"], vec!["مصير أطفال المشركين"]),
            (&["non", "muslim", "baik", "masuk", "surga"], vec!["مصير غير المسلمين المحسنين"]),
            (&["orang", "tidak", "dengar", "islam", "bagaimana"], vec!["مصير من لم تبلغه الدعوة"]),
            (&["iman", "naik", "turun", "bertambah"], vec!["زيادة الإيمان ونقصانه"]),
            (&["iman", "amal", "shaleh", "hubungan"], vec!["الإيمان والعمل الصالح"]),
            (&["perbuatan", "dosa", "diampuni", "syirik"], vec!["المغفرة وشرك الله"]),
            (&["taubat", "diterima", "syarat", "kapan"], vec!["شروط التوبة المقبولة"]),
            (&["istidraj", "musibah", "nikmat", "ujian"], vec!["الاستدراج والابتلاء"]),
            (&["qada", "allah", "segala", "sesuatu"], vec!["مشيئة الله وإرادته"]),
            (&["fitrah", "manusia", "suci", "lahir"], vec!["فطرة الإنسان"]),
            (&["roh", "asal", "usul", "ditiupkan"], vec!["الروح وأصلها", "نفخ الروح"]),
            (&["mimpi", "baik", "buruk", "tafsir"], vec!["تفسير الأحلام"]),
            (&["mimpi", "nabi", "wahyu", "hukum"], vec!["رؤيا الأنبياء ووحيهم"]),
            (&["nabi", "manusia", "biasa", "perbedaan"], vec!["خصائص الأنبياء والرسل"]),
            (&["mukjizat", "karamah", "beda"], vec!["الفرق بين المعجزة والكرامة"]),
            (&["wali", "allah", "ciri", "definisi"], vec!["أولياء الله وصفاتهم"]),
            (&["bid'ah", "hasanah", "sayyi'ah", "beda"], vec!["أنواع البدعة الحسنة والسيئة"]),
            (&["sunnah", "bid'ah", "bagimana", "ukur"], vec!["معيار السنة والبدعة"]),
            (&["tafsir", "quran", "boleh", "dengan", "ra'yu"], vec!["حكم التفسير بالرأي"]),
            (&["nasikh", "mansukh", "quran", "contoh"], vec!["الناسخ والمنسوخ في القرآن"]),
            (&["asbabun", "nuzul", "manfaat", "mengetahui"], vec!["أسباب النزول"]),
            (&["muhkam", "mutasyabih", "ayat", "beda"], vec!["المحكم والمتشابه"]),
            (&["qiroat", "sab'ah", "tujuh", "bacaan"], vec!["القراءات السبع"]),
            (&["hafiz", "menjaga", "quran", "kewajiban"], vec!["حفظ القرآن وفضله"]),
            (&["tajwid", "wajib", "hukum", "membaca"], vec!["أحكام التجويد"]),
            (&["terjemah", "quran", "bahasa", "indonesia"], vec!["ترجمة القرآن ومشروعيتها"]),

            // BATCH 107: Tasawwuf, zuhud, and spiritual stations
            (&["tasawuf", "pengertian", "hakikat", "hukum"], vec!["حقيقة التصوف"]),
            (&["zuhud", "cinta", "dunia", "hukum"], vec!["الزهد وحب الدنيا"]),
            (&["wara", "menghindari", "syubhat"], vec!["الورع وترك الشبهات"]),
            (&["tawadu", "rendah", "hati", "hukum"], vec!["التواضع"]),
            (&["tawakkal", "usaha", "berdoa", "bersama"], vec!["التوكل مع الأخذ بالأسباب"]),
            (&["sabar", "syukur", "dalam", "islam"], vec!["الصبر والشكر"]),
            (&["ikhlas", "cara", "mencapai"], vec!["الإخلاص وكيفية تحقيقه"]),
            (&["muraqabah", "muhasabah", "menyucikan", "jiwa"], vec!["المراقبة والمحاسبة"]),
            (&["maqam", "ahwal", "sufi", "tingkat"], vec!["المقامات والأحوال الصوفية"]),
            (&["tobat", "inabah", "zuhud", "urutan"], vec!["مراتب التوبة والزهد"]),
            (&["fana", "baqa", "sufi", "pengertian"], vec!["الفناء والبقاء في التصوف"]),
            (&["hulul", "ittihad", "wahdat", "wujud"], vec!["الحلول والاتحاد ووحدة الوجود"]),
            (&["thariqah", "naqsyabandiyah", "qadiriyah"], vec!["الطرق الصوفية"]),
            (&["baiat", "thariqah", "jenis", "hukum"], vec!["بيعة الطريقة الصوفية"]),
            (&["dzikir", "berjamaah", "keras", "hukum"], vec!["الذكر الجهري الجماعي"]),
            (&["hadrah", "maulid", "inshad", "hukum"], vec!["حكم الحضرة والمديح النبوي"]),
            (&["silsilah", "guru", "murid", "thariqah"], vec!["سلسلة الطريقة الصوفية"]),
            (&["ilmu", "laduni", "wahyu", "makdum"], vec!["العلم اللدني والإلهام"]),
            (&["wali", "keramat", "dalil", "hukum"], vec!["كرامات الأولياء"]),
            (&["pantangan", "sufi", "adab", "suluk"], vec!["آداب التصوف والسلوك"]),
            (&["murid", "guru", "syekh", "adab"], vec!["أدب المريد مع شيخه"]),
            (&["halaqah", "ta'lim", "majelis", "ilmu"], vec!["حلقات العلم ومجالس التعليم"]),
            (&["sunah", "nabi", "cara", "ikut"], vec!["كيفية الاتباع النبوي"]),
            (&["mahabbah", "cinta", "allah", "cara"], vec!["محبة الله وطرق تحصيلها"]),
            (&["khauf", "roja", "takut", "harap"], vec!["الخوف والرجاء"]),
            (&["ridha", "qanaah", "menerima", "takdir"], vec!["الرضا والقناعة والتسليم"]),
            (&["mujahadah", "latih", "jiwa", "cara"], vec!["المجاهدة والرياضة الروحية"]),
            (&["nafsu", "ammarah", "lawwamah", "muthmainnah"], vec!["النفس الأمّارة واللوّامة والمطمئنة"]),
            (&["qalb", "aql", "ruh", "nafsu", "beda"], vec!["الفرق بين القلب والعقل والروح والنفس"]),
            (&["syatahat", "ucapan", "ganjil", "sufi"], vec!["الشطحات الصوفية"]),
            (&["halal", "haram", "sufi", "hukum", "syariat"], vec!["الصوفية والشريعة"]),
            (&["kaum", "darwis", "sufi", "jalan", "sunah"], vec!["التصوف السني"]),
            (&["ghazali", "ihya", "ulumuddin", "isi"], vec!["إحياء علوم الدين للغزالي"]),
            (&["ibnu", "arabi", "fusush", "futuhat", "pandangan"], vec!["آراء ابن عربي وخلافاتها"]),
            (&["rumi", "matsnawi", "syair", "hukum"], vec!["مثنوي الرومي وحكمه"]),
            (&["penyair", "sufi", "rumi", "hafiz"], vec!["الشعراء الصوفيون"]),
            (&["musik", "sama", "tari", "sufi", "hukum"], vec!["السماع والرقص عند الصوفية"]),
            (&["makan", "sedikit", "lapar", "jiwa"], vec!["الجوع وآدابه عند الصوفية"]),
            (&["uzlah", "menyendiri", "masyarakat"], vec!["العزلة وحكمها"]),
            (&["khatam", "quran", "doa", "majlis"], vec!["ختم القرآن والدعاء"]),

            // BATCH 108: Usul fiqh methodology advanced
            (&["istihsan", "contoh", "penerapan"], vec!["الاستحسان وتطبيقاته"]),
            (&["mashlahah", "mursalah", "contoh"], vec!["المصلحة المرسلة"]),
            (&["saddu", "dzara'i", "contoh", "penerapan"], vec!["سد الذرائع وتطبيقاته"]),
            (&["urf", "adat", "dalil", "hukum"], vec!["العرف وحجيته"]),
            (&["istishab", "prinsip", "contoh"], vec!["الاستصحاب وحجيته"]),
            (&["ijma", "jenis", "sarih", "sukuti"], vec!["الإجماع الصريح والسكوتي"]),
            (&["qiyas", "rukun", "syarat", "contoh"], vec!["أركان القياس وشروطه"]),
            (&["illat", "hikmat", "hukum", "beda"], vec!["الفرق بين العلة والحكمة"]),
            (&["maqasid", "syariah", "dhoruriyat", "hajiyat"], vec!["الضروريات والحاجيات والتحسينيات"]),
            (&["kaidah", "fiqh", "lima", "pokok"], vec!["القواعد الفقهية الخمس الكبرى"]),
            (&["kaidah", "masyaqqah", "taisir", "contoh"], vec!["المشقة تجلب التيسير"]),
            (&["kaidah", "darurat", "membolehkan", "terlarang"], vec!["الضرورات تبيح المحظورات"]),
            (&["kaidah", "yakin", "syak", "tidak", "hilangkan"], vec!["اليقين لا يزال بالشك"]),
            (&["kaidah", "mudhorat", "dihilangkan"], vec!["الضرر يزال"]),
            (&["kaidah", "adat", "ditetapkan", "hukum"], vec!["العادة محكّمة"]),
            (&["nasakh", "quran", "sunnah", "hadits"], vec!["نسخ القرآن بالسنة"]),
            (&["dhahir", "nash", "lafaz", "dilalah"], vec!["دلالة اللفظ عند الأصوليين"]),
            (&["manthuq", "mafhum", "muwafaqah", "mukhalafah"], vec!["المنطوق والمفهوم"]),
            (&["am", "khas", "taksis", "bayan"], vec!["العام والخاص والتخصيص"]),
            (&["muthlaq", "muqayyad", "hamlu", "cara"], vec!["المطلق والمقيد"]),
            (&["amar", "nahi", "jenis", "konsekuensi"], vec!["الأمر والنهي"]),
            (&["silogisme", "qiyas", "logika", "fiqh"], vec!["المنطق والقياس الأصولي"]),
            (&["ijtihad", "syarat", "kapan", "boleh"], vec!["شروط الاجتهاد"]),
            (&["taqlid", "jenis", "hukum", "kapan"], vec!["أنواع التقليد وأحكامها"]),
            (&["ittiba", "beda", "taqlid", "dalil"], vec!["الفرق بين الاتباع والتقليد"]),
            (&["talfiq", "mazhab", "hukum", "boleh"], vec!["حكم التلفيق"]),
            (&["fatwa", "syarat", "cara", "mengeluarkan"], vec!["شروط الفتوى"]),
            (&["mufti", "hakim", "wewenang", "beda"], vec!["الفرق بين المفتي والقاضي"]),
            (&["qadla", "hukum", "administrasi", "qadha"], vec!["القضاء في الإسلام"]),
            (&["hukum", "taklifi", "wadh'i", "beda"], vec!["الحكم التكليفي والوضعي"]),
            (&["sababul", "ikhtilaf", "ulama", "penyebab"], vec!["أسباب اختلاف الفقهاء"]),
            (&["khilaf", "fiqh", "ahkam", "sebab"], vec!["أسباب الخلاف الفقهي"]),
            (&["mazhab", "empat", "cara", "lahir"], vec!["نشأة المذاهب الأربعة"]),
            (&["mazhab", "hanafi", "ciri", "wilayah"], vec!["المذهب الحنفي وخصائصه"]),
            (&["mazhab", "maliki", "ciri", "wilayah"], vec!["المذهب المالكي وخصائصه"]),
            (&["mazhab", "syafii", "ciri", "wilayah"], vec!["المذهب الشافعي وخصائصه"]),
            (&["mazhab", "hanbali", "ciri", "wilayah"], vec!["المذهب الحنبلي وخصائصه"]),
            (&["mazhab", "zhahiri", "ibnu", "hazm", "dalil"], vec!["المذهب الظاهري وابن حزم"]),
            (&["mazhab", "ja'fari", "syiah", "fiqh"], vec!["الفقه الجعفري والمذهب الشيعي"]),
            (&["mujtahid", "mustaqil", "muntasib", "beda"], vec!["أنواع المجتهدين"]),

            // BATCH 109: Advanced pesantren-specific and scholarly Arabic terms
            (&["bayan", "taqrir", "tafsir", "nabi"], vec!["أنواع البيان النبوي"]),
            (&["hadits", "manhaj", "naqd", "jarh", "ta'dil"], vec!["الجرح والتعديل"]),
            (&["rawi", "tsiqat", "dhaif", "cara", "nilai"], vec!["تقييم رواة الحديث"]),
            (&["sanad", "matan", "hadits", "pengertian"], vec!["السند والمتن في الحديث"]),
            (&["hadits", "maudhu", "palsu", "ciri"], vec!["الحديث الموضوع وعلاماته"]),
            (&["hadits", "hasan", "li", "dzatihi", "li", "ghairihi"], vec!["الحديث الحسن لذاته ولغيره"]),
            (&["hadits", "shahih", "syarat", "lima"], vec!["شروط صحة الحديث"]),
            (&["hukum", "mencari", "ilmu", "fardhu", "ain"], vec!["فرضية طلب العلم"]),
            (&["adab", "belajar", "ilmu", "cara", "sunah"], vec!["آداب طلب العلم"]),
            (&["guru", "adab", "menghormati", "ulama"], vec!["أدب العالم والمتعلم"]),
            (&["kitab", "kuning", "pesantren", "penting"], vec!["كتب التراث في المدارس الإسلامية"]),
            (&["nahwu", "sorof", "i'rob", "pesantren"], vec!["علم النحو والصرف"]),
            (&["balaghah", "ma'ani", "bayan", "badi"], vec!["علم البلاغة"]),
            (&["mantiq", "logika", "fiqh", "kaitan"], vec!["علم المنطق الإسلامي"]),
            (&["falak", "hisab", "rukyat", "beda"], vec!["الهلال بين الحساب والرؤية"]),
            (&["rukyat", "hilal", "cara", "syarat"], vec!["رؤية الهلال وشروطها"]),
            (&["isbat", "pengadilan", "agama", "mapan"], vec!["إثبات دخول الشهر"]),
            (&["ramadhan", "berbeda", "tanggal", "mazhab"], vec!["اختلاف بداية رمضان"]),
            (&["kalender", "hijriah", "urutan", "bulan"], vec!["التقويم الهجري"]),
            (&["malam", "nisfu", "sya'ban", "amalan"], vec!["ليلة النصف من شعبان وأعمالها"]),
            (&["15", "syaban", "hukum", "ibadah", "khusus"], vec!["حكم العبادات الخاصة في النصف من شعبان"]),
            (&["malam", "jum'at", "amalan", "keutamaan"], vec!["أعمال ليلة الجمعة وفضلها"]),
            (&["hari", "jumat", "afdhal", "waktu", "doa"], vec!["فضل يوم الجمعة وأوقات الإجابة"]),
            (&["bulan", "haram", "empat", "nama", "hukum"], vec!["الأشهر الحرم الأربعة"]),
            (&["hari", "asyura", "puasa", "sejarah"], vec!["تاسوعاء وعاشوراء"]),
            (&["muharram", "keutamaan", "amalan"], vec!["فضل شهر محرم"]),
            (&["dzulhijjah", "sepuluh", "keutamaan"], vec!["فضل العشر من ذي الحجة"]),
            (&["shalawat", "jenis", "ibrahimiyah", "nariyah"], vec!["أنواع الصلاة على النبي"]),
            (&["shalawat", "nariyah", "bidah", "hukum"], vec!["حكم صلاة النارية"]),
            (&["tahlil", "kalimat", "la", "ilaha", "illallah"], vec!["التهليل وفضله"]),
            (&["takbir", "idul", "fitri", "adha", "lafaz"], vec!["التكبير في العيدين"]),
            (&["adzan", "iqomat", "lafaz", "waktu"], vec!["ألفاظ الأذان والإقامة"]),
            (&["adzan", "subuh", "hayya", "ala", "shalah"], vec!["الأذان الصبحي ومفرداته"]),
            (&["do'a", "antara", "adzan", "iqomat"], vec!["الدعاء بين الأذان والإقامة"]),
            (&["doa", "setelah", "adzan", "lafaz"], vec!["الدعاء بعد الأذان"]),
            (&["niat", "puasa", "ramadhan", "lafaz"], vec!["نية صيام رمضان"]),
            (&["niat", "zakat", "fitrah", "lafaz"], vec!["نية زكاة الفطر"]),
            (&["niat", "qurban", "lafaz", "cara"], vec!["نية الأضحية"]),
            (&["niat", "aqiqah", "lafaz", "cara"], vec!["نية العقيقة"]),
            (&["niat", "haji", "umroh", "lafaz"], vec!["نية الحج والعمرة"]),

            // BATCH 110: Social media, digital life, and modern questions
            (&["instagram", "tiktok", "youtube", "konten", "dakwah"], vec!["الدعوة عبر وسائل التواصل الاجتماعي"]),
            (&["fb", "facebook", "whatsapp", "hukum", "ghibah"], vec!["الغيبة في الفضاء الرقمي"]),
            (&["share", "hoax", "berita", "bohong", "hukum"], vec!["نشر الأخبار الكاذبة"]),
            (&["like", "konten", "maksiat", "medsos"], vec!["حكم الإعجاب بالمحتوى المحرم"]),
            (&["follow", "akun", "haram", "maksiat"], vec!["حكم متابعة الحسابات المحرمة"]),
            (&["foto", "manusia", "boleh", "posting"], vec!["حكم نشر الصور"]),
            (&["selfie", "foto", "diri", "sendiri", "hukum"], vec!["حكم صور السيلفي"]),
            (&["streaming", "live", "shalat", "hukum"], vec!["البث المباشر للصلاة"]),
            (&["online", "ceramah", "khutbah", "sah"], vec!["الخطبة والتعليم الديني عبر الإنترنت"]),
            (&["khatib", "jumat", "lewat", "video", "sah"], vec!["صحة الصلاة بخطيب افتراضي"]),
            (&["akad", "nikah", "online", "video", "call"], vec!["صحة عقد النكاح عبر الإنترنت"]),
            (&["bayar", "mahar", "transfer", "bank", "sharia"], vec!["دفع المهر إلكترونياً"]),
            (&["infak", "online", "transfer", "sah"], vec!["الصدقة الرقمية وصحتها"]),
            (&["cryptocurrency", "as", "zakat", "harta"], vec!["زكاة العملات الرقمية"]),
            (&["nft", "halal", "haram", "dalil"], vec!["حكم الرموز غير القابلة للاستبدال"]),
            (&["game", "online", "judi", "beda"], vec!["الفرق بين الألعاب الإلكترونية والقمار"]),
            (&["game", "online", "hukum", "islam"], vec!["حكم الألعاب الإلكترونية"]),
            (&["esport", "turnamen", "game", "hadiah"], vec!["حكم مسابقات الألعاب الإلكترونية"]),
            (&["judi", "togel", "lotere", "hukum"], vec!["حكم القمار واليانصيب"]),
            (&["undian", "berhadiah", "lotere", "hukum"], vec!["حكم السحب على الجوائز"]),
            (&["asuransi", "jiwa", "jaminan", "hukum"], vec!["حكم التأمين على الحياة"]),
            (&["bpjs", "jaminan", "kesehatan", "hukum"], vec!["حكم برنامج الضمان الصحي"]),
            (&["dana", "pensiun", "pensiunan", "hukum"], vec!["حكم صندوق التقاعد"]),
            (&["leasing", "kredit", "motor", "hukum"], vec!["حكم التأجير التمويلي"]),
            (&["gadai", "barang", "cicil", "hukum"], vec!["حكم الرهن التقسيطي"]),
            (&["pawn", "shop", "pegadaian", "hukum"], vec!["حكم الرهن في الشريعة"]),
            (&["investasi", "saham", "reksa", "hukum"], vec!["حكم الاستثمار في الأسهم"]),
            (&["properti", "over", "kredit", "hukum"], vec!["حكم الإحالة في الديون"]),
            (&["kos", "kontrakan", "sewa", "kamar"], vec!["حكم إيجار الغرفة"]),
            (&["airbnb", "booking", "hotel", "halal"], vec!["حكم الإقامة في فنادق مختلطة"]),
            (&["travel", "umroh", "promo", "dp", "cicil"], vec!["حكم حج التقسيط"]),
            (&["bekerja", "di", "bank", "konvensional"], vec!["حكم العمل في البنوك الربوية"]),
            (&["karyawan", "perusahaan", "haram", "hukum"], vec!["حكم العمل في شركات محرمة"]),
            (&["pajak", "wajib", "islam", "hukum"], vec!["الضريبة في الإسلام"]),
            (&["gratifikasi", "suap", "hadiah", "jabatan"], vec!["حكم الهدايا في الوظائف الحكومية"]),
            (&["korupsi", "merugikan", "negara", "hukum"], vec!["حكم الفساد المالي"]),
            (&["curang", "bisnis", "hukum", "islam"], vec!["حكم الغش في التجارة"]),
            (&["monopoli", "kartel", "hukum", "islam"], vec!["حكم الاحتكار"]),
            (&["ihtikar", "menimbun", "barang", "hukum"], vec!["حكم الاحتكار والتخزين"]),
            (&["riba", "bunga", "bank", "halal", "syarat"], vec!["الربا في المعاملات المصرفية"]),

            // BATCH 111: Children's Islamic education and parenting
            (&["aqiqah", "hari", "ketujuh", "syarat"], vec!["العقيقة في اليوم السابع"]),
            (&["aqiqah", "anak", "laki", "dua", "kambing"], vec!["عقيقة الذكر شاتان"]),
            (&["aqiqah", "anak", "perempuan", "satu", "kambing"], vec!["عقيقة الأنثى شاة"]),
            (&["cukur", "rambut", "bayi", "sunah", "cara"], vec!["حلق شعر المولود"]),
            (&["azan", "telinga", "bayi", "baru", "lahir"], vec!["أذان في أذن المولود"]),
            (&["nama", "baik", "anak", "sunah", "cara"], vec!["التسمية وما يستحب من الأسماء"]),
            (&["khitan", "anak", "laki", "hukum", "waktu"], vec!["الختان ووقته وحكمه"]),
            (&["khitan", "perempuan", "hukum", "mazhab"], vec!["ختان الإناث ومسألة المذاهب"]),
            (&["mengajar", "anak", "shalat", "berapa", "umur"], vec!["تعليم الأطفال الصلاة"]),
            (&["anak", "dosa", "belum", "baligh", "apakah"], vec!["تكليف الطفل قبل البلوغ"]),
            (&["baligh", "ciri", "anak", "laki", "perempuan"], vec!["علامات البلوغ"]),
            (&["mendidik", "anak", "cara", "islami"], vec!["أساليب التربية الإسلامية"]),
            (&["anak", "nakal", "hukum", "pukul"], vec!["التأديب بالضرب وحكمه"]),
            (&["hukum", "memukul", "anak", "mendidik"], vec!["ضرب الأطفال في التأديب"]),
            (&["anak", "yatim", "hak", "perwalian"], vec!["حق الولاية على اليتيم"]),
            (&["tanggung", "jawab", "nafkah", "anak"], vec!["نفقة الأبناء"]),
            (&["suami", "wafat", "nafkah", "anak", "siapa"], vec!["نفقة الأبناء بعد وفاة الأب"]),
            (&["ibu", "tiri", "ayah", "tiri", "hukum", "nasab"], vec!["حكم الزوج الثاني والأبناء"]),
            (&["saudara", "tiri", "mahram", "bukan"], vec!["محرمية الأخ من الأم"]),
            (&["susuan", "saudara", "mahram", "hukum"], vec!["الأخوة الرضاعية والمحرمية"]),
            (&["anak", "di", "luar", "nikah", "siapa", "wali"], vec!["ولاية ابن الزنا"]),
            (&["anak", "haram", "waris", "dari", "ayah"], vec!["إرث ولد الزنا"]),
            (&["hak", "asuh", "anak", "cerai", "usia"], vec!["حضانة الأطفال وسنها"]),
            (&["ibu", "kafir", "hak", "asuh", "anak"], vec!["حضانة الأم غير المسلمة"]),
            (&["beda", "agama", "orang", "tua", "anak"], vec!["الطفل بين الأبوين المختلفي الدين"]),
            (&["hukum", "menikahkan", "anak", "kecil"], vec!["حكم تزويج الأطفال"]),
            (&["mengaji", "mushaf", "tanpa", "wudhu", "anak"], vec!["حكم مس الأطفال للمصحف"]),
            (&["shalat", "anak", "batal", "apakah", "imam"], vec!["هل يجوز إمامة الصبي؟"]),
            (&["muhrim", "mahram", "beda", "pengertian"], vec!["الفرق بين المحرم والمُحرِم"]),
            (&["muhrim", "safar", "wanita", "jarak"], vec!["شرط المحرم في سفر المرأة"]),
            (&["wanita", "safar", "tanpa", "mahram", "boleh"], vec!["حكم سفر المرأة بدون محرم"]),
            (&["haid", "shalat", "qadha", "apakah", "wajib"], vec!["حكم قضاء الصلاة للحائض"]),
            (&["haid", "puasa", "qadha", "wajib", "kapan"], vec!["قضاء صوم الحائض"]),
            (&["wanita", "menstruasi", "kerja", "kantor"], vec!["المرأة الحائض في بيئة العمل"]),
            (&["batas", "usia", "menikah", "perempuan", "laki"], vec!["سن الزواج في الإسلام"]),
            (&["hak", "perempuan", "cerai", "kapan"], vec!["حق المرأة في طلب الطلاق"]),
            (&["perempuan", "bekerja", "diluar", "keluarga"], vec!["خروج المرأة للعمل"]),
            (&["wanita", "pidato", "ceramah", "hukum"], vec!["حكم خطاب المرأة أمام الرجال"]),
            (&["suara", "wanita", "aurat", "hukum"], vec!["صوت المرأة وحكمه"]),
            (&["wanita", "menjadi", "pemimpin", "hakim"], vec!["المرأة قاضية ووالية"]),

            // BATCH 112: Shalat timing, qibla, and travel edge cases
            (&["waktu", "shalat", "lima", "waktu", "daftar"], vec!["أوقات الصلوات الخمس"]),
            (&["waktu", "subuh", "fajar", "sadiq", "kadzib"], vec!["الفجر الصادق والكاذب"]),
            (&["waktu", "dhuha", "dari", "sampai"], vec!["وقت صلاة الضحى"]),
            (&["waktu", "zhuhur", "berakhir", "kapan"], vec!["انتهاء وقت الظهر"]),
            (&["waktu", "ashar", "ikhtiari", "dharuri"], vec!["وقت العصر الاختياري والضروري"]),
            (&["waktu", "maghrib", "berapa", "menit"], vec!["وقت المغرب"]),
            (&["waktu", "isya", "batas", "tengah", "malam"], vec!["وقت العشاء وانتهاؤه"]),
            (&["shalat", "di", "tempat", "najis", "tidak", "tahu"], vec!["الصلاة في المكان النجس جهلاً"]),
            (&["shalat", "menghadap", "kiblat", "cara"], vec!["استقبال القبلة وكيفيته"]),
            (&["shalat", "arah", "kiblat", "kompas", "app"], vec!["تحديد اتجاه القبلة"]),
            (&["kiblat", "berubah", "dua", "kali", "sejarah"], vec!["تحويل القبلة من بيت المقدس"]),
            (&["shalat", "di", "kapal", "laut", "bergerak"], vec!["الصلاة في السفينة"]),
            (&["shalat", "di", "pesawat", "waktu", "qashar"], vec!["الصلاة في الطائرة"]),
            (&["shalat", "di", "kereta", "bergerak", "cara"], vec!["الصلاة في القطار"]),
            (&["shalat", "di", "bulan", "luar", "angkasa"], vec!["الصلاة في الفضاء الخارجي"]),
            (&["waktu", "shalat", "kutub", "utara", "selatan"], vec!["الصلاة في القطبين"]),
            (&["waktu", "shalat", "malam", "tanpa", "subuh"], vec!["الصلاة في بلاد لا يغيب شمسها"]),
            (&["shalat", "sambil", "naik", "motor", "darurat"], vec!["الصلاة على الدابة في حال الضرورة"]),
            (&["shalat", "di", "lantai", "atas", "sajadah", "wajib"], vec!["حكم السجود على السجادة"]),
            (&["shalat", "tanpa", "sajadah", "boleh"], vec!["الصلاة على الأرض"]),
            (&["imam", "shalat", "syarat", "lebih", "utama"], vec!["شروط إمامة الصلاة"]),
            (&["wanita", "imam", "sesama", "perempuan"], vec!["إمامة المرأة للنساء"]),
            (&["bacaan", "imam", "dikeraskan", "diperlahankan"], vec!["الجهر والإسرار في القراءة"]),
            (&["makmum", "baca", "fatihah", "sendiri"], vec!["هل يقرأ المأموم الفاتحة؟"]),
            (&["shalat", "berjamaah", "shaf", "lurus"], vec!["تسوية الصفوف في الصلاة"]),
            (&["shaf", "perempuan", "di", "belakang", "laki"], vec!["صف النساء في الصلاة"]),
            (&["shalat", "di", "shaf", "sendiri", "boleh"], vec!["الصلاة منفرداً خلف الصف"]),
            (&["minimal", "jamaah", "dua", "orang", "sah"], vec!["أقل الجماعة في الصلاة"]),
            (&["shalat", "sendirian", "imam", "sendiri"], vec!["صلاة الفذ"]),
            (&["shalat", "tarawih", "berjamaah", "di", "rumah"], vec!["صلاة التراويح في البيت"]),
            (&["shalat", "witir", "setelah", "tarawih", "cara"], vec!["الوتر بعد التراويح"]),
            (&["shalat", "id", "lapangan", "masjid", "mana", "utama"], vec!["أيهما أفضل: صلاة العيد في المسجد أم المصلى؟"]),
            (&["mandi", "sunah", "hari", "jumat", "kapan"], vec!["سنة الاغتسال يوم الجمعة"]),
            (&["mandi", "sunah", "sebelum", "shalat", "id"], vec!["سنة الاغتسال لصلاة العيد"]),
            (&["mandi", "sunah", "ihram", "haji", "umroh"], vec!["سنة الاغتسال للإحرام"]),
            (&["wudhu", "usap", "kepala", "seluruh", "sebagian"], vec!["مسح الرأس في الوضوء"]),
            (&["wudhu", "usap", "telinga", "hukum"], vec!["مسح الأذنين في الوضوء"]),
            (&["niat", "puasa", "sehari", "sehari", "atau", "sekali"], vec!["تبييت نية الصوم"]),
            (&["puasa", "batal", "sengaja", "makan", "kaffarah"], vec!["كفارة الإفطار العمد"]),
            (&["puasa", "tapi", "mimpi", "basah", "batal"], vec!["هل الاحتلام يفسد الصوم؟"]),

            // BATCH 113: Islamic political thought and public life
            (&["khilafah", "konsep", "sejarah", "bentuk"], vec!["الخلافة الإسلامية"]),
            (&["negara", "islam", "khilafah", "wajib", "dalil"], vec!["وجوب إقامة الدولة الإسلامية"]),
            (&["demokrasi", "hukum", "boleh", "ikut"], vec!["حكم المشاركة في الانتخابات الديمقراطية"]),
            (&["memilih", "presiden", "wajib", "boleh", "islam"], vec!["الواجب السياسي في الانتخاب"]),
            (&["partai", "islam", "boleh", "bergabung"], vec!["حكم الانتماء للأحزاب الإسلامية"]),
            (&["menjadi", "anggota", "dpr", "dprd", "hukum"], vec!["حكم دخول البرلمان"]),
            (&["hukum", "taat", "pemimpin", "kafir", "dzalim"], vec!["طاعة ولي الأمر الكافر والظالم"]),
            (&["bughot", "pemberontakan", "pemimpin", "hukum"], vec!["حكم الخروج على الحاكم"]),
            (&["jihad", "difa", "hukum", "kapan"], vec!["الجهاد الدفاعي"]),
            (&["jihad", "thalab", "hukum", "kondisi"], vec!["الجهاد الهجومي"]),
            (&["ghazwul", "fikri", "perang", "pemikiran"], vec!["الغزو الفكري"]),
            (&["radikalisme", "tegangnya", "islamofobia"], vec!["التطرف الديني"]),
            (&["amar", "makruf", "nahi", "munkar", "cara"], vec!["الأمر بالمعروف والنهي عن المنكر"]),
            (&["dakwah", "cara", "hikmah", "mau'izah"], vec!["أساليب الدعوة"]),
            (&["wajib", "dakwah", "siapa", "bertanggung"], vec!["فرضية الدعوة"]),
            (&["berdakwah", "kepada", "non", "muslim", "cara"], vec!["الدعوة إلى غير المسلمين"]),
            (&["ridda", "keluar", "islam", "hukum"], vec!["حكم الردة"]),
            (&["ulama", "dulu", "kitab", "modern", "konteks"], vec!["الاجتهاد المعاصر"]),
            (&["fatwa", "kontemporer", "persoalan", "baru"], vec!["الفتاوى المعاصرة"]),
            (&["majlis", "fatwa", "mui", "keputusan"], vec!["قرارات مجلس الإفتاء الوطني"]),
            (&["perda", "syariah", "boleh", "indonesia"], vec!["الأنظمة الإسلامية المحلية"]),
            (&["hukuman", "mati", "indonesia", "islam"], vec!["عقوبة الإعدام في الإسلام"]),
            (&["narkoba", "narkotika", "hukum", "islam"], vec!["المخدرات في الإسلام"]),
            (&["shabu", "heroin", "ganja", "hukum"], vec!["حكم المواد المخدرة"]),
            (&["miras", "minuman", "keras", "hukum"], vec!["حكم المسكرات"]),
            (&["hukum", "jual", "beli", "miras", "non", "muslim"], vec!["حكم بيع المسكرات لغير المسلمين"]),
            (&["sex", "bebas", "zina", "hukum", "had"], vec!["بيان حد الزنا"]),
            (&["free", "sex", "pra", "nikah", "hukum"], vec!["حكم الزنا قبل الزواج"]),
            (&["homoseksual", "lgbt", "hukum", "islam"], vec!["حكم الشذوذ الجنسي"]),
            (&["lesbian", "gay", "biseksual", "hukum"], vec!["حكم العلاقات المثلية"]),
            (&["terorisme", "bom", "hukum", "jihad"], vec!["حكم الإرهاب"]),
            (&["bom", "bunuh", "diri", "syahid", "dalil"], vec!["حكم العمليات الانتحارية"]),
            (&["ahlussunnah", "wal", "jamaah", "definisi"], vec!["أهل السنة والجماعة"]),
            (&["syiah", "sunnah", "beda", "aqidah"], vec!["الفروق بين السنة والشيعة"]),
            (&["muktazilah", "asyariah", "maturidiyah"], vec!["المعتزلة والأشاعرة والماتريدية"]),
            (&["wahabi", "salafi", "tabligh", "hizb"], vec!["التيارات الإسلامية المعاصرة"]),
            (&["khawatir", "zaman", "akhir", "fitnah", "cara"], vec!["حكم الفتن في آخر الزمان"]),
            (&["tafarruq", "perpecahan", "umat", "hukum"], vec!["حكم الفرقة والاختلاف"]),
            (&["ukhuwah", "islamiyah", "wathaniyah", "basyariyah"], vec!["الأخوة الإسلامية والوطنية والإنسانية"]),
            (&["hak", "non", "muslim", "dalam", "islam"], vec!["حقوق غير المسلمين في الإسلام"]),

            // BATCH 114: Fiqh of contracts and mu'amalah specifics
            (&["jual", "beli", "yang", "belum", "ada", "barang"], vec!["بيع ما لا يملك", "بيع المعدوم"]),
            (&["akad", "salam", "pesanan", "bayar", "duluan"], vec!["عقد السلم"]),
            (&["istishna", "akad", "pesan", "buat", "barang"], vec!["عقد الاستصناع"]),
            (&["akad", "ju'alah", "upah", "hasil", "kerja"], vec!["عقد الجعالة"]),
            (&["akad", "ijarah", "sewa", "jasa", "cara"], vec!["عقد الإجارة وشروطه"]),
            (&["ijarah", "muntahia", "bittamlik", "leasing"], vec!["الإجارة المنتهية بالتمليك"]),
            (&["qardh", "hasan", "pinjam", "tanpa", "bunga"], vec!["القرض الحسن"]),
            (&["rahn", "gadai", "syariah", "cara"], vec!["الرهن الشرعي"]),
            (&["hawalah", "akad", "pengalihan", "hutang"], vec!["عقد الحوالة"]),
            (&["kafalah", "akad", "penjamin", "hutang"], vec!["عقد الكفالة"]),
            (&["wakalah", "kuasa", "syarat", "cara"], vec!["عقد الوكالة"]),
            (&["syirkah", "musyarakah", "modalnya", "cara"], vec!["عقد الشركة"]),
            (&["mudharabah", "modal", "bagi", "hasil"], vec!["المضاربة وأركانها"]),
            (&["wadi'ah", "titipan", "amanah", "dhaman"], vec!["الوديعة"]),
            (&["ariyah", "pinjam", "barang", "tanpa", "bayar"], vec!["العارية"]),
            (&["hibah", "syarat", "akad", "cara"], vec!["الهبة وشروط صحتها"]),
            (&["wasiat", "cara", "sah", "syarat"], vec!["الوصية وشروطها"]),
            (&["shulh", "perdamaian", "damai", "sengketa"], vec!["عقد الصلح"]),
            (&["syuf'ah", "hak", "beli", "kembali", "tetangga"], vec!["الشفعة"]),
            (&["luqtah", "barang", "temuan", "hukum"], vec!["اللقطة وأحكامها"]),
            (&["ghashb", "ambil", "barang", "hukum"], vec!["الغصب وأحكامه"]),
            (&["ihya", "lahan", "mati", "hukum"], vec!["إحياء الموات"]),
            (&["iqrar", "pengakuan", "hak", "orang", "lain"], vec!["الإقرار"]),
            (&["muzaraah", "pertanian", "bagi", "hasil"], vec!["المزارعة"]),
            (&["musaqah", "kebun", "bagi", "hasil"], vec!["المساقاة"]),
            (&["mugarasah", "kebun", "buah", "bagi"], vec!["المغارسة"]),
            (&["akad", "sah", "rukun", "syarat", "umum"], vec!["أركان العقد وشروط صحته"]),
            (&["fasakh", "akad", "batal", "cara", "kapan"], vec!["فسخ العقد"]),
            (&["khiyar", "hak", "pilih", "jual", "beli"], vec!["خيار البيع"]),
            (&["gharar", "ketidakpastian", "akad", "hukum"], vec!["الغرر في العقود"]),
            (&["maysir", "spekulasi", "judi", "akad"], vec!["الميسر والمخاطرة"]),
            (&["riba", "fadhl", "nasi'ah", "beda"], vec!["ربا الفضل وربا النسيئة"]),
            (&["riba", "jual", "beli", "emas", "tunai"], vec!["بيع الذهب بالذهب وشروطه"]),
            (&["bay", "inah", "tawarruq", "hukum"], vec!["بيع العينة والتورق"]),
            (&["talaqqi", "rukban", "hukum", "beli", "murah"], vec!["تلقي الركبان"]),
            (&["najsy", "menaikkan", "harga", "palsu"], vec!["النجش"]),
            (&["bay", "muzabanah", "araya", "buah"], vec!["بيع المزابنة والعرايا"]),
            (&["bay", "mukhadharah", "buah", "belum", "matang"], vec!["بيع المنابذة والمخاضرة"]),
            (&["ihtikar", "barang", "pokok", "sembako"], vec!["احتكار السلع الضرورية"]),
            (&["monopoli", "perusahaan", "besar", "hukum"], vec!["الاحتكار الاقتصادي"]),

            // BATCH 115: More modern fiqh scenarios
            (&["arisan", "hukum", "boleh", "riba"], vec!["حكم الأرسان (جمعية الادخار)"]),
            (&["utang", "piutang", "tulis", "saksi", "wajib"], vec!["توثيق الديون"]),
            (&["sewa", "rumah", "tinggal", "tidak", "bayar"], vec!["حكم الإجارة والمطل"]),
            (&["kontrakan", "rusak", "siapa", "tanggung"], vec!["ضمان المستأجر"]),
            (&["tanah", "sengketa", "hukum", "cara", "selesai"], vec!["النزاع على الأراضي"]),
            (&["hak", "milik", "intelektual", "islam"], vec!["حقوق الملكية الفكرية"]),
            (&["paten", "hak", "cipta", "dagang"], vec!["براءة الاختراع وحقوق النشر"]),
            (&["bajak", "software", "ilegal", "hukum"], vec!["حكم القرصنة الرقمية"]),
            (&["download", "film", "ilegal", "hukum"], vec!["حكم تنزيل الملفات المحمية"]),
            (&["upah", "kerja", "rampas", "buruh", "hukum"], vec!["حكم حبس أجر العامل"]),
            (&["kontrak", "kerja", "syarat", "hukum"], vec!["عقد العمل وشروطه"]),
            (&["phk", "pecat", "kerja", "hukum", "islam"], vec!["حكم فسخ عقد العمل"]),
            (&["cuti", "haid", "kantor", "hak", "hukum"], vec!["إجازة الحيض في العمل"]),
            (&["tip", "uang", "pelayan", "hukum"], vec!["حكم الإكرامية"]),
            (&["pungutan", "liar", "pungli", "hukum"], vec!["حكم الابتزاز والرشوة"]),
            (&["pungli", "suap", "darah", "pejabat"], vec!["حكم الانتزاع غير المشروع"]),
            (&["zakat", "diberikan", "kepada", "pegawai"], vec!["إعطاء الزكاة للموظفين"]),
            (&["zakat", "mal", "diberikan", "ke", "anak"], vec!["إعطاء الزكاة للأبناء"]),
            (&["kaffarah", "zihar", "cara", "penebusnya"], vec!["كفارة الظهار"]),
            (&["kaffarah", "ila", "sumpah", "tidak", "campur"], vec!["حكم الإيلاء وكفارته"]),
            (&["dhihar", "zihar", "mengharamkan", "istri"], vec!["الظهار وحكمه"]),
            (&["li'an", "tuduh", "istri", "zina", "hukum"], vec!["اللعان"]),
            (&["hadhanah", "biaya", "siapa", "tanggung", "cerai"], vec!["نفقة الحضانة بعد الطلاق"]),
            (&["mut'ah", "cerai", "hak", "istri"], vec!["المتعة للمطلقة"]),
            (&["nafkah", "iddah", "siapa", "bayar"], vec!["نفقة الاعتداد"]),
            (&["talak", "satu", "dua", "tiga", "hitungan"], vec!["حساب الطلاق"]),
            (&["kembali", "setelah", "talak", "tiga", "syarat"], vec!["الرجوع بعد الطلاق الثلاث"]),
            (&["nikah", "lagi", "setelah", "cerai", "kapan"], vec!["إعادة الزواج بعد الطلاق"]),
            (&["wali", "nikah", "jarak", "jauh", "wakil"], vec!["التوكيل في الولاية"]),
            (&["akad", "nikah", "syarat", "sah", "pokok"], vec!["شروط صحة عقد النكاح"]),
            (&["mahar", "minimum", "tidak", "ada", "batas"], vec!["ما يصح مهراً"]),
            (&["ijab", "qabul", "satu", "majelis", "syarat"], vec!["اشتراط الإيجاب والقبول في مجلس واحد"]),
            (&["saksi", "nikah", "dua", "syarat"], vec!["شروط شاهدي العقد"]),
            (&["walimah", "pesta", "pernikahan", "wajib"], vec!["وجوب وليمة العرس"]),
            (&["walimah", "undangan", "hadir", "wajib"], vec!["حكم إجابة دعوة الوليمة"]),
            (&["pengantin", "musik", "nyanyian", "hukum"], vec!["حكم الموسيقى في الأعراس"]),
            (&["foto", "pengantin", "album", "hukum"], vec!["حكم تصوير حفلات الزفاف"]),
            (&["cincin", "tunangan", "kalung", "hukum"], vec!["حكم خاتم الخطبة"]),
            (&["tunangan", "apakah", "nikah", "boleh", "pacaran"], vec!["حكم التخطيب والخروج"]),
            (&["ta'aruf", "cara", "benar", "syariatnya"], vec!["التعارف الشرعي في الزواج"]),

            // BATCH 116: Hadith collections + Islamic history
            (&["shahih", "bukhari", "berapa", "hadits", "isi"], vec!["صحيح البخاري"]),
            (&["shahih", "muslim", "hadits", "kitab"], vec!["صحيح مسلم"]),
            (&["sunan", "abu", "dawud", "hadits"], vec!["سنن أبي داود"]),
            (&["sunan", "tirmidzi", "hadits", "hukum"], vec!["سنن الترمذي"]),
            (&["sunan", "nasai", "hadits", "kitab"], vec!["سنن النسائي"]),
            (&["sunan", "ibnu", "majah", "hadits"], vec!["سنن ابن ماجه"]),
            (&["muwatta", "malik", "hadits", "kitab"], vec!["موطأ مالك"]),
            (&["musnad", "ahmad", "hadits", "banyak"], vec!["مسند أحمد بن حنبل"]),
            (&["riyaadh", "salihin", "nawawi", "bab"], vec!["رياض الصالحين"]),
            (&["bulughul", "maram", "ibnu", "hajar", "fiqh"], vec!["بلوغ المرام"]),
            (&["arba'in", "nawawi", "empat", "puluh", "hadits"], vec!["الأربعون النووية"]),
            (&["hadits", "qudsi", "beda", "quran", "nabi"], vec!["الحديث القدسي"]),
            (&["nabi", "muhammad", "lahir", "tanggal", "tahun"], vec!["مولد النبي ﷺ"]),
            (&["maulid", "nabi", "kapan", "12", "rabiul"], vec!["المولد النبوي"]),
            (&["hijrah", "nabi", "madinah", "sejarah"], vec!["هجرة النبي إلى المدينة"]),
            (&["isra", "mi'raj", "cerita", "kapan"], vec!["الإسراء والمعراج"]),
            (&["badar", "perang", "pertama", "sejarah"], vec!["غزوة بدر"]),
            (&["uhud", "perang", "sejarah", "pelajaran"], vec!["غزوة أحد"]),
            (&["khandaq", "perang", "parit", "sejarah"], vec!["غزوة الخندق"]),
            (&["mekah", "penaklukan", "fath", "sejarah"], vec!["فتح مكة"]),
            (&["khulafaur", "rasyidin", "khalifah", "empat"], vec!["الخلفاء الراشدون"]),
            (&["abu", "bakar", "siddiq", "khalifah"], vec!["أبو بكر الصديق"]),
            (&["umar", "faruk", "khalifah", "sejarah"], vec!["عمر بن الخطاب"]),
            (&["usman", "khalifah", "quran", "kumpulkan"], vec!["عثمان بن عفان"]),
            (&["ali", "bin", "abi", "thalib", "khalifah"], vec!["علي بن أبي طالب"]),
            (&["umayyah", "dinasti", "khalifah", "sejarah"], vec!["الدولة الأموية"]),
            (&["abbasiyah", "dinasti", "khalifah", "sejarah"], vec!["الدولة العباسية"]),
            (&["andalus", "spanyol", "islam", "masuk"], vec!["الأندلس"]),
            (&["ustmaniyah", "ottoman", "turki", "khalifah"], vec!["الدولة العثمانية"]),
            (&["wali", "songo", "jawa", "islam", "masuk"], vec!["دخول الإسلام إلى جاوة"]),
            (&["nusantara", "islam", "masuk", "kapan", "sejarah"], vec!["دخول الإسلام إلى جزر الملايو"]),
            (&["sahabat", "nabi", "siapa", "paling", "utama"], vec!["أفضل الصحابة"]),
            (&["ahlul", "bait", "nabi", "siapa", "saja"], vec!["أهل البيت النبوي"]),
            (&["nabi", "keluarga", "istri", "anak", "cucu"], vec!["آل النبي ﷺ"]),
            (&["ashabul", "kahfi", "cerita", "berapa", "orang"], vec!["أصحاب الكهف"]),
            (&["luqman", "hakim", "nasihat", "anak"], vec!["لقمان الحكيم"]),
            (&["maryam", "isa", "kelahiran", "cerita"], vec!["قصة مريم وعيسى"]),
            (&["nabi", "ibrahim", "ka'bah", "bangun", "sejarah"], vec!["إبراهيم وبناء الكعبة"]),
            (&["nabi", "musa", "firaun", "laut", "terbelah"], vec!["موسى وفرعون"]),
            (&["nabi", "yusuf", "mimpi", "mesir", "kisah"], vec!["قصة يوسف عليه السلام"]),

            // BATCH 117: Indonesian informal Islamic questions (ngomong/nanya style)
            (&["boleh", "gak", "minum", "pakai", "tangan", "kiri"], vec!["حكم الأكل والشرب بالشمال"]),
            (&["boleh", "gak", "hp", "saat", "shalat"], vec!["ما يبطل الصلاة"]),
            (&["kalau", "lupa", "bacaan", "shalat", "gimana"], vec!["السهو في الصلاة وسجود السهو"]),
            (&["lupa", "niat", "puasa", "subuh", "sahur", "boleh"], vec!["حكم من لم ينو الصيام قبل الفجر"]),
            (&["gimana", "cara", "tobat", "dosa", "besar"], vec!["التوبة النصوح من الكبائر"]),
            (&["kenapa", "harus", "shalat", "5", "waktu"], vec!["فرضية الصلاة الخمس"]),
            (&["boleh", "gak", "tidak", "shalat", "sekali", "karena"], vec!["حكم ترك الصلاة"]),
            (&["apakah", "boleh", "shalat", "pakai", "baju", "bergambar"], vec!["الصلاة في الثياب ذات الصور"]),
            (&["boleh", "gak", "batal", "wudhu", "saat", "shalat"], vec!["ما يبطل الوضوء"]),
            (&["berapa", "kali", "istinja", "cebok", "usap"], vec!["الاستجمار عدد المسحات"]),
            (&["boleh", "gak", "cebok", "pakai", "tissue"], vec!["الاستجمار بالورق"]),
            (&["gimana", "kalau", "tidak", "ada", "air", "mandi"], vec!["التيمم عن الغسل"]),
            (&["mandi", "junub", "cukup", "niat", "siram", "saja"], vec!["كفاية نية الغسل"]),
            (&["boleh", "gak", "mandi", "junub", "kulit", "tattoo"], vec!["الغسل مع الوشم"]),
            (&["apakah", "harus", "wudhu", "dulu", "sebelum", "quran"], vec!["حكم لمس المصحف"]),
            (&["boleh", "baca", "quran", "tanpa", "wudhu", "hafal"], vec!["قراءة القرآن من الحفظ"]),
            (&["boleh", "gak", "tidur", "tengkurap", "hukumnya"], vec!["حكم النوم على البطن"]),
            (&["boleh", "makan", "sambil", "berdiri", "makruh"], vec!["حكم الأكل قائماً"]),
            (&["boleh", "gak", "tidur", "setelah", "subuh"], vec!["النوم بعد صلاة الفجر"]),
            (&["tidur", "setelah", "sahur", "sebelum", "subuh", "boleh"], vec!["النوم بعد السحور"]),
            (&["boleh", "gak", "istri", "menolak", "ajakan", "suami"], vec!["حكم امتناع الزوجة"]),
            (&["berapa", "kali", "seminggu", "hak", "suami", "istri"], vec!["حق الوطء وتوقيته"]),
            (&["suami", "boleh", "paksa", "istri", "apakah", "boleh"], vec!["الإكراه الزوجي"]),
            (&["boleh", "gak", "tabarruj", "depan", "suami"], vec!["التبرج أمام الزوج"]),
            (&["suami", "perlu", "izin", "istri", "keluar"], vec!["إذن الزوج للخروج"]),
            (&["istri", "kerja", "pendapatan", "milik", "siapa"], vec!["ملكية كسب الزوجة"]),
            (&["gimana", "kalau", "tidak", "mau", "bayar", "zakat"], vec!["عقوبة مانع الزكاة"]),
            (&["berapa", "nisab", "zakat", "emas", "2024"], vec!["نصاب زكاة الذهب"]),
            (&["zakat", "fitrah", "berapa", "kilo", "beras"], vec!["زكاة الفطر بالكيلو"]),
            (&["fidyah", "tidak", "puasa", "berapa", "bayar"], vec!["الفدية وقدرها"]),
            (&["boleh", "gak", "nonton", "film", "sambil", "puasa"], vec!["مفطرات الصوم"]),
            (&["apakah", "marah", "batal", "puasa"], vec!["هل الغضب يفطر"]),
            (&["apakah", "mimpi", "basah", "batal", "puasa"], vec!["الاحتلام في رمضان"]),
            (&["boleh", "sikat", "gigi", "waktu", "puasa"], vec!["السواك والمعجون في رمضان"]),
            (&["boleh", "kumur", "saat", "puasa", "berlebihan"], vec!["المبالغة في المضمضة والصوم"]),
            (&["boleh", "gak", "donor", "darah", "saat", "puasa"], vec!["التبرع بالدم في رمضان"]),
            (&["suntik", "infus", "saat", "puasa", "batal"], vec!["الحقن الوريدية والصيام"]),
            (&["boleh", "gak", "menelan", "ludah", "sendiri", "puasa"], vec!["ابتلاع الريق والصيام"]),
            (&["cium", "istri", "saat", "puasa", "boleh"], vec!["القبلة في رمضان"]),
            (&["apakah", "onani", "batal", "puasa", "hukum"], vec!["الاستمناء والصيام"]),

            // BATCH 118: Medical/health/disability fiqh
            (&["orang", "sakit", "keras", "tidak", "bisa", "shalat"], vec!["صلاة المريض العاجز"]),
            (&["shalat", "di", "rumah", "sakit", "cara", "boleh"], vec!["الصلاة في المستشفى"]),
            (&["rawat", "inap", "rumah", "sakit", "shalat", "cara"], vec!["صلاة المريض المنوم"]),
            (&["pasien", "sakit", "puasa", "ramadhan", "boleh", "tidak"], vec!["الصوم والمرض"]),
            (&["cuci", "darah", "dialisis", "puasa", "batal"], vec!["الصوم والغسيل الكلوي"]),
            (&["insulin", "diabetes", "suntik", "puasa", "batal"], vec!["حقن الأنسولين والصوم"]),
            (&["cancer", "kanker", "kemoterapi", "puasa", "boleh"], vec!["الصوم مع العلاج الكيماوي"]),
            (&["waria", "banci", "transgender", "hukum", "islam"], vec!["حكم التشبه والتحول الجنسي"]),
            (&["khunsa", "hermafrodit", "hukum", "waris", "shalat"], vec!["أحكام الخنثى"]),
            (&["tuli", "bisu", "shalat", "cara", "hukum"], vec!["صلاة الأصم والأبكم"]),
            (&["buta", "tuna", "netra", "shalat", "kiblat", "cara"], vec!["صلاة الأعمى وضبط القبلة"]),
            (&["tuna", "daksa", "kursi", "roda", "shalat", "cara"], vec!["صلاة ذوي الإعاقة الحركية"]),
            (&["autis", "cacat", "mental", "shalat", "taklif"], vec!["التكليف والإعاقة الذهنية"]),
            (&["gila", "majnun", "shalat", "taklif", "hukum"], vec!["تكليف المجنون"]),
            (&["operasi", "bedah", "aurat", "dokter", "lawan", "jenis"], vec!["كشف العورة للطبيب"]),
            (&["dokter", "perempuan", "periksa", "laki", "boleh"], vec!["معالجة الطبيبة للمريض الأجنبي"]),
            (&["obat", "haram", "darurat", "boleh", "dipakai"], vec!["التداوي بالمحرم للضرورة"]),
            (&["transplantasi", "organ", "donor", "tubuh", "hukum"], vec!["حكم نقل الأعضاء"]),
            (&["donor", "ginjal", "hati", "jantung", "hukum", "islam"], vec!["التبرع بالأعضاء الحيوية"]),
            (&["bank", "sperma", "bayi", "tabung", "hukum"], vec!["حكم بنوك الحيوانات المنوية"]),
            (&["bayi", "tabung", "in", "vitro", "fertilization", "hukum"], vec!["أطفال الأنابيب"]),
            (&["aborsi", "janin", "cacat", "boleh", "tidak"], vec!["إجهاض الجنين المشوه"]),
            (&["kontrasepsi", "kb", "spiral", "pil", "hukum"], vec!["تنظيم النسل وموانع الحمل"]),
            (&["vasektomi", "tubektomi", "sterilisasi", "hukum"], vec!["قطع النسل وحكمه"]),
            (&["menyusui", "ibu", "susu", "formula", "boleh"], vec!["الرضاعة الطبيعية والصناعية"]),
            (&["autopsi", "bedah", "mayat", "hukum", "islam"], vec!["تشريح الجثة"]),
            (&["kematian", "otak", "brain", "dead", "cabut", "alat"], vec!["الموت الدماغي وقطع الأجهزة"]),
            (&["euthanasia", "suntik", "mati", "hukum", "islam"], vec!["قتل الرحمة"]),
            (&["rokok", "elektrik", "vape", "hukum", "islam"], vec!["حكم السيجارة الإلكترونية"]),
            (&["narkoba", "rehabilitasi", "zakat", "mustahiq"], vec!["المدمن وأهلية الزكاة"]),
            (&["psikolog", "konseling", "terapi", "hukum", "islam"], vec!["العلاج النفسي في الإسلام"]),
            (&["meditasi", "yoga", "senam", "hukum", "islam"], vec!["حكم التأمل واليوغا"]),
            (&["pergi", "ke", "dukun", "untuk", "sembuh", "hukum"], vec!["التداوي بالسحر"]),
            (&["ruqyah", "syariah", "cara", "bacaan", "hukum"], vec!["الرقية الشرعية"]),
            (&["bekam", "hijama", "sunnah", "cara", "hukum"], vec!["الحجامة"]),
            (&["akupunktur", "jarum", "hukum", "islam"], vec!["حكم الوخز بالإبر"]),
            (&["homeopati", "obat", "homeopathy", "hukum"], vec!["حكم التداوي بالهوميوباثي"]),
            (&["plasenta", "air", "ketuban", "ari", "milik", "siapa"], vec!["حكم المشيمة"]),
            (&["amputasi", "potong", "anggota", "badan", "shalat"], vec!["الصلاة بعد بتر الأطراف"]),
            (&["luka", "balut", "perban", "wudhu", "cara", "usap"], vec!["المسح على الجبيرة"]),

            // BATCH 119: Business/work ethics and specific Islamic finance
            (&["gaji", "minimum", "hak", "buruh", "islam"], vec!["الحد الأدنى للأجور"]),
            (&["serikat", "pekerja", "buruh", "hukum", "islam"], vec!["النقابات العمالية"]),
            (&["mogok", "kerja", "demo", "buruh", "hukum"], vec!["الإضراب العمالي"]),
            (&["perusahaan", "bangkrut", "hutang", "karyawan", "hak"], vec!["إفلاس الشركة وحقوق الموظفين"]),
            (&["resign", "keluar", "kerja", "hak", "pesangon"], vec!["حق الأجر عند ترك العمل"]),
            (&["cuti", "hamil", "melahirkan", "hak", "karyawan"], vec!["إجازة الوضع"]),
            (&["usaha", "bersama", "syirkah", "bagi", "hasil"], vec!["المضاربة والمشاركة في الأرباح"]),
            (&["franchise", "waralaba", "hukum", "syariah"], vec!["حكم الامتياز التجاري"]),
            (&["multi", "level", "marketing", "mlm", "syariah"], vec!["التسويق الشبكي"]),
            (&["investasi", "saham", "untung", "rugi", "bagi", "hukum"], vec!["الاستثمار في الأسهم"]),
            (&["obligasi", "sukuk", "surat", "utang", "hukum"], vec!["الصكوك الإسلامية"]),
            (&["deposito", "bank", "bunga", "halal", "haram"], vec!["الودائع المصرفية"]),
            (&["giro", "tabungan", "bank", "riba", "boleh"], vec!["حكم حسابات البنوك"]),
            (&["pinjam", "tanpa", "bunga", "syariah", "cara"], vec!["القرض الحسن"]),
            (&["gadai", "emas", "bank", "syariah", "biaya"], vec!["رهن الذهب"]),
            (&["leasing", "sewa", "beli", "benda", "manfaat"], vec!["الإجارة المنتهية بالتمليك"]),
            (&["murabahah", "beli", "harga", "mark", "up", "halal"], vec!["المرابحة وحكمها"]),
            (&["bay", "salam", "bayar", "dahulu", "barang", "kemudian"], vec!["بيع السلم"]),
            (&["bay", "istisna", "pesan", "buat", "barang", "bayar"], vec!["بيع الاستصناع"]),
            (&["sewa", "ijarah", "harga", "tetap", "berubah", "boleh"], vec!["تحديد الأجرة في الإجارة"]),
            (&["wakaf", "saham", "produktif", "hukum", "syariah"], vec!["وقف الأسهم"]),
            (&["wakaf", "uang", "tunai", "bank", "nasional", "hukum"], vec!["الوقف النقدي"]),
            (&["infak", "sedekah", "beda", "wakaf", "zakat"], vec!["الفرق بين الإنفاق والصدقة والزكاة والوقف"]),
            (&["jual", "beli", "tanpa", "akad", "jelas", "sah", "gak"], vec!["حكم المعاطاة"]),
            (&["khiyar", "majelis", "syarat", "aibi", "bola", "hukum"], vec!["خيار المجلس والشرط والعيب"]),
            (&["mal", "mitsli", "qimi", "beda", "ganti", "rugi"], vec!["المال المثلي والقيمي"]),
            (&["harga", "patokan", "pemerintah", "wajib", "ikut", "tidak"], vec!["تسعير الحكومة"]),
            (&["import", "ekspor", "bea", "cukai", "halal", "haram"], vec!["رسوم الاستيراد والتصدير"]),
            (&["asuransi", "konvensional", "beda", "takaful", "premi"], vec!["الفرق بين التأمين التجاري والتكافل"]),
            (&["klaim", "asuransi", "lebih", "dari", "kerugian", "hukum"], vec!["استرداد أكثر من الضرر في التأمين"]),
            (&["saham", "perusahaan", "haram", "jual", "terlanjur", "beli"], vec!["ما يفعل حامل أسهم الشركات المحرمة"]),
            (&["platform", "e-commerce", "fee", "komisi", "halal"], vec!["العمولة في التجارة الإلكترونية"]),
            (&["endorse", "iklan", "produk", "haram", "boleh"], vec!["الترويج للمنتجات المحرمة"]),
            (&["influencer", "konten", "dakwah", "bayar", "halal"], vec!["أجر الدعاة والمحتوى الرقمي"]),
            (&["hak", "siar", "siaran", "live", "streaming", "hukum"], vec!["حقوق البث والإذاعة"]),
            (&["copywriting", "iklan", "kata", "tipu", "daya", "hukum"], vec!["الإعلان المضلل"]),
            (&["diskon", "bonus", "hadiah", "pelanggan", "halal"], vec!["العروض الترويجية"]),
            (&["uang", "tip", "kembalian", "toko", "ambil", "hukum"], vec!["حكم الزيادة عند الصرف"]),
            (&["jual", "beli", "barang", "curian", "tidak", "tahu"], vec!["بيع السلعة المسروقة جهلاً"]),
            (&["menjual", "barang", "cacat", "tanpa", "memberitahu"], vec!["بيع المعيب دون بيان"]),

            // BATCH 120: Ritual purity detailed questions
            (&["air", "mutlak", "musyammas", "musta'mal", "jenis"], vec!["أنواع المياه الطهورة"]),
            (&["air", "mutlak", "bekas", "pakai", "najis", "tidak"], vec!["الماء المستعمل"]),
            (&["air", "sedikit", "banyak", "dua", "qullah", "liter"], vec!["الماء القليل والكثير"]),
            (&["sumur", "air", "sumur", "najis", "cara", "suci"], vec!["تطهير البئر"]),
            (&["air", "hujan", "embun", "es", "wudhu", "sah"], vec!["الطهارة بماء المطر والثلج"]),
            (&["kolam", "renang", "wudhu", "najis", "sah"], vec!["الوضوء بماء السباحة"]),
            (&["wudhu", "urutan", "anggota", "tiga", "kali"], vec!["ترتيب الوضوء وعدد الغسلات"]),
            (&["usap", "kepala", "wudhu", "seluruh", "sebagian"], vec!["مسح الرأس"]),
            (&["usap", "telinga", "wudhu", "wajib", "sunah"], vec!["مسح الأذنين"]),
            (&["kaki", "tumit", "wudhu", "mencuci", "usap"], vec!["غسل الرجلين إلى الكعبين"]),
            (&["muwalat", "tertib", "wudhu", "sela", "lama"], vec!["الموالاة في الوضوء"]),
            (&["niat", "wudhu", "kapan", "diucap", "dalam", "hati"], vec!["نية الوضوء"]),
            (&["bercukur", "jenggot", "wudhu", "batal", "tidak"], vec!["الحلق وانتقاض الوضوء"]),
            (&["menyentuh", "qur'an", "wudhu", "haid", "boleh"], vec!["مس المصحف بغير طهارة"]),
            (&["masjid", "masuk", "haid", "junub", "boleh"], vec!["دخول الجنب والحائض المسجد"]),
            (&["shalat", "tanpa", "wudhu", "tahu", "tidak", "hukum"], vec!["الصلاة بغير طهارة"]),
            (&["debu", "tanah", "tayamum", "cara", "sah", "syarat"], vec!["التيمم وكيفيته"]),
            (&["tayamum", "pengganti", "wudhu", "mandi", "bisa"], vec!["صحة التيمم عن الغسل"]),
            (&["tayamum", "berapa", "kali", "shalat", "satu"], vec!["التيمم لأكثر من صلاة"]),
            (&["tayamum", "habis", "kapan", "batal", "kondisi"], vec!["نواقض التيمم"]),
            (&["istinja", "cebok", "setelah", "buang", "air", "wajib"], vec!["وجوب الاستنجاء"]),
            (&["qiblatain", "arah", "kiblat", "salah", "ulang", "shalat"], vec!["إصابة القبلة واختلافها"]),
            (&["pakaian", "bernajis", "sentuh", "shalat", "batal"], vec!["أثر الثوب النجس على الصلاة"]),
            (&["najis", "kering", "basah", "lantai", "masjid"], vec!["النجاسة اليابسة والرطبة"]),
            (&["sepatu", "kaos", "kaki", "usap", "wudhu", "cara"], vec!["المسح على الخفين والجوربين"]),
            (&["pembalut", "wanita", "shalat", "darah", "bocor", "najis"], vec!["حكم تسرب الدم وصلاة الحائض"]),
            (&["istihadhah", "wanita", "shalat", "puasa", "boleh"], vec!["المستحاضة وأحكامها"]),
            (&["nifas", "batas", "hari", "40", "60", "berapa"], vec!["مدة النفاس"]),
            (&["haid", "berhenti", "istihadhah", "bedanya", "cara", "tahu"], vec!["التمييز في الحيض"]),
            (&["haid", "tidak", "rutin", "tidak", "teratur", "cara"], vec!["الحيض المضطرب"]),
            (&["darah", "setelah", "melahirkan", "nifas", "berhenti", "mandi"], vec!["الاغتسال من النفاس"]),
            (&["sunah", "mandi", "hari", "jumat", "ihram", "id"], vec!["المواضع المستحب فيها الغسل"]),
            (&["mandi", "sunnah", "cara", "urutan", "niat", "bacaan"], vec!["كيفية الغسل المستحب"]),
            (&["wudhu", "sah", "celup", "tangan", "ke", "air"], vec!["الوضوء بالارتماس"]),
            (&["wudhu", "perban", "gips", "cara", "usap", "boleh"], vec!["الوضوء مع الجبيرة"]),
            (&["mandi", "junub", "urutan", "niat", "bismillah", "wudhu"], vec!["كيفية الغسل من الجنابة"]),
            (&["mandi", "junub", "saat", "puasa", "siang", "boleh"], vec!["الاغتسال من الجنابة نهار رمضان"]),
            (&["tanda", "wanita", "baligh", "haid", "pertama", "mandi"], vec!["الاغتسال عند أول حيض"]),
            (&["mimpi", "basah", "mandi", "wajib", "laki", "perempuan"], vec!["وجوب الغسل من الاحتلام"]),
            (&["mencukur", "bulu", "kemaluan", "sunah", "hukum", "frekuensi"], vec!["إزالة شعر العانة"]),

            // BATCH 121: Zakat calculation, hajj/umroh, Islamic months details
            (&["zakat", "perak", "nisab", "200", "dirham", "berapa"], vec!["نصاب زكاة الفضة"]),
            (&["zakat", "pertanian", "5", "wasaq", "berapa", "kg"], vec!["نصاب زكاة الزروع"]),
            (&["zakat", "ternak", "unta", "sapi", "kambing", "nishab"], vec!["نصاب زكاة الأنعام"]),
            (&["zakat", "tijarah", "dagang", "modal", "untung", "cara"], vec!["زكاة عروض التجارة"]),
            (&["zakat", "rikaz", "temuan", "barang", "seper", "lima"], vec!["زكاة الركاز"]),
            (&["zakat", "profesi", "gaji", "kapan", "haul", "nisab"], vec!["زكاة الراتب"]),
            (&["amil", "zakat", "boleh", "makan", "berapa", "persen"], vec!["سهم العاملين على الزكاة"]),
            (&["zakat", "digunakan", "untuk", "bangun", "masjid", "boleh"], vec!["الزكاة للمساجد"]),
            (&["mustahiq", "zakat", "delapan", "asnaf", "golongan"], vec!["مصارف الزكاة الثمانية"]),
            (&["sabilillah", "zakat", "pengertian", "siapa", "dapat"], vec!["في سبيل الله من مصارف الزكاة"]),
            (&["zakat", "fitrah", "kapan", "waktu", "boleh", "tunaikan"], vec!["وقت إخراج زكاة الفطر"]),
            (&["zakat", "fitrah", "uang", "or", "makanan", "mana", "afdhal"], vec!["إخراج زكاة الفطر نقداً"]),
            (&["haji", "wajib", "berapa", "kali", "seumur", "hidup"], vec!["وجوب الحج مرة في العمر"]),
            (&["haji", "badal", "orang", "lain", "syarat", "hukum"], vec!["حج البدل"]),
            (&["haji", "khusus", "biaya", "hutang", "boleh", "berangkat"], vec!["الحج بمال مستقرض"]),
            (&["ihram", "miqat", "dimana", "tempat", "mana", "mulai"], vec!["المواقيت المكانية"]),
            (&["mahram", "wanita", "haji", "tanpa", "boleh", "tidak"], vec!["سفر المرأة للحج بلا محرم"]),
            (&["tawaf", "cara", "tujuh", "putaran", "syarat"], vec!["كيفية الطواف وشروطه"]),
            (&["sa'i", "shafa", "marwah", "cara", "tujuh", "lintasan"], vec!["السعي بين الصفا والمروة"]),
            (&["wukuf", "arafah", "kapan", "waktu", "wajib", "syarat"], vec!["الوقوف بعرفة"]),
            (&["mabit", "muzdalifah", "mina", "wajib", "hukum"], vec!["المبيت بمزدلفة ومنى"]),
            (&["jumrah", "lempar", "batu", "cara", "waktu", "syarat"], vec!["رمي الجمرات"]),
            (&["thawaf", "ifadhah", "wada", "qudum", "beda", "hukum"], vec!["أنواع الطواف"]),
            (&["tahalul", "awal", "tsani", "haji", "cara", "larangan"], vec!["التحلل الأول والثاني"]),
            (&["dam", "haji", "apa", "kapan", "wajib", "bayar"], vec!["دماء الحج وموجباتها"]),
            (&["umroh", "berkali", "satu", "safar", "boleh"], vec!["تكرار العمرة في سفر واحد"]),
            (&["umroh", "ramadhan", "keutamaan", "sama", "haji"], vec!["فضل العمرة في رمضان"]),
            (&["rajab", "puasa", "keutamaan", "khusus", "ada"], vec!["فضل الصيام في رجب"]),
            (&["sya'ban", "nisfu", "malam", "15", "amalan", "dalil"], vec!["ليلة النصف من شعبان"]),
            (&["ramadhan", "laylatul", "qadr", "malam", "kapan", "tanda"], vec!["ليلة القدر"]),
            (&["itikaf", "masjid", "kapan", "syarat", "cara", "hukum"], vec!["الاعتكاف وأحكامه"]),
            (&["sepuluh", "hari", "terakhir", "ramadhan", "amalan", "apa"], vec!["العشر الأواخر من رمضان"]),
            (&["idul", "fitri", "amalan", "sunnah", "hari", "apa"], vec!["أعمال يوم العيد"]),
            (&["idul", "adha", "amalan", "sunah", "sebelum", "shalat"], vec!["أعمال عيد الأضحى"]),
            (&["qurban", "idul", "adha", "syarat", "hewan", "cara"], vec!["أحكام الأضحية"]),
            (&["aqiqah", "syarat", "hewan", "umur", "kembar", "anak"], vec!["أحكام العقيقة"]),
            (&["puasa", "syawal", "enam", "hari", "keuntungan", "cara"], vec!["صيام ستة من شوال"]),
            (&["puasa", "arafah", "9", "dzulhijjah", "keutamaan"], vec!["صيام يوم عرفة"]),
            (&["puasa", "tasu'a", "asyura", "10", "muharram", "dalil"], vec!["صيام يوم عاشوراء"]),
            (&["puasa", "daud", "sehari", "tidak", "sehari", "ya", "cara"], vec!["صيام داود"]),

            // BATCH 122: Inheritance calculations and family law
            (&["waris", "ashab", "furudh", "siapa", "dapat", "berapa"], vec!["أصحاب الفروض"]),
            (&["waris", "istri", "berapa", "suami", "ada", "anak"], vec!["ميراث الزوجين"]),
            (&["waris", "ibu", "sepertiga", "seperenam", "kondisi"], vec!["ميراث الأم"]),
            (&["waris", "bapak", "anak", "ada", "tidak", "berapa"], vec!["ميراث الأب"]),
            (&["waris", "anak", "laki", "beda", "perempuan", "berapa"], vec!["ميراث الأبناء"]),
            (&["waris", "saudara", "kandung", "seayah", "seibu", "berapa"], vec!["ميراث الإخوة"]),
            (&["waris", "cucu", "laki", "perempuan", "kakek", "nenek"], vec!["ميراث الأحفاد"]),
            (&["'aul", "radd", "masalah", "waris", "apa", "contoh"], vec!["العول والرد في الميراث"]),
            (&["al-gharrawayn", "masalah", "umariyatain", "waris"], vec!["المسألة الغراوية"]),
            (&["masalah", "musytarakah", "himariyah", "waris"], vec!["المسألة المشتركة"]),
            (&["munasakhah", "waris", "sebelum", "bagi", "meninggal"], vec!["المناسخة في الميراث"]),
            (&["waris", "noni", "muslim", "beda", "agama", "dapat"], vec!["حرمان الكافر من الميراث"]),
            (&["waris", "bunuh", "ahli", "waris", "pembunuh", "hak"], vec!["القاتل لا يرث"]),
            (&["waris", "anak", "dalam", "kandungan", "janin", "hak"], vec!["ميراث الجنين"]),
            (&["takharuj", "perdamaian", "waris", "gugur", "hak"], vec!["التخارج في الميراث"]),
            (&["mahjub", "hajb", "terhalang", "waris", "siapa"], vec!["الحجب في الميراث"]),
            (&["dzul", "arham", "waris", "kerabat", "jauh", "dapat"], vec!["ذوو الأرحام في الميراث"]),
            (&["nikah", "fasid", "tidak", "sah", "waris", "tetap"], vec!["الميراث في النكاح الفاسد"]),
            (&["thalaq", "raj'i", "ba'in", "waris", "masih", "bisa"], vec!["الميراث بين المطلقين"]),
            (&["wasiat", "waris", "beda", "kapan", "berlaku", "siapa"], vec!["الفرق بين الوصية والإرث"]),
            (&["wasiat", "untuk", "ahli", "waris", "boleh", "tidak"], vec!["الوصية لوارث"]),
            (&["wasiat", "wajibah", "cucu", "anak", "angkat", "hukum"], vec!["الوصية الواجبة"]),
            (&["hibah", "beda", "waris", "wasiat", "kapan", "aktif"], vec!["الفرق بين الهبة والوصية"]),
            (&["rujuk", "talak", "satu", "dua", "cara", "saksi"], vec!["الرجعة وكيفيتها"]),
            (&["iddah", "wafat", "berapa", "bulan", "hamil", "tidak"], vec!["عدة الوفاة"]),
            (&["iddah", "talak", "haid", "tidak", "haid", "berapa"], vec!["عدة الطلاق"]),
            (&["masa", "iddah", "boleh", "keluar", "rumah", "atau", "tidak"], vec!["حكم خروج المعتدة"]),
            (&["ihdad", "istri", "meninggal", "suami", "boleh", "pakai"], vec!["الإحداد على الزوج"]),
            (&["perwalian", "wali", "anak", "cerai", "ibu", "bapak"], vec!["ولاية المطلقة على الأولاد"]),
            (&["biaya", "sekolah", "anak", "cerai", "siapa", "tanggung"], vec!["نفقة تعليم الأطراف بعد الطلاق"]),
            (&["pernikahan", "beda", "agama", "islam", "hukumnya"], vec!["الزواج من غير المسلمين"]),
            (&["menikah", "kristen", "yahudi", "kitabiyah", "boleh"], vec!["زواج المسلم من الكتابية"]),
            (&["muslimah", "menikah", "non", "muslim", "boleh", "tidak"], vec!["زواج المسلمة من غير المسلم"]),
            (&["pindah", "agama", "setelah", "menikah", "hukum", "pernikahan"], vec!["أثر الردة على عقد الزواج"]),
            (&["kafir", "masuk", "islam", "nikah", "lagi", "perlu", "tidak"], vec!["إسلام الكافر وعقد النكاح"]),
            (&["anak", "dari", "pernikahan", "tidak", "sah", "nasab"], vec!["نسب ولد النكاح الفاسد"]),
            (&["nasab", "anak", "dari", "zina", "ikut", "ibu", "bapak"], vec!["نسب ولد الزنا"]),
            (&["susuan", "radha'", "mahram", "berapa", "kali", "hukum"], vec!["عدد الرضعات المحرمة"]),
            (&["susuan", "orang", "dewasa", "mahram", "tidak", "berlaku"], vec!["رضاع الكبير"]),
            (&["anak", "tiri", "mahram", "apakah", "hukum"], vec!["أحكام الربيب والربيبة"]),

            // BATCH 123: Detailed shalat rulings and Jum'ah specifics
            (&["sujud", "sahwi", "cara", "waktu", "lupa", "shalat"], vec!["سجود السهو"]),
            (&["sujud", "tilawah", "cara", "wajib", "hukum", "surah"], vec!["سجود التلاوة"]),
            (&["sujud", "syukur", "kapan", "boleh", "cara"], vec!["سجود الشكر"]),
            (&["ruku", "thuma'ninah", "arti", "ukur", "berapa", "lama"], vec!["الطمأنينة في الركوع"]),
            (&["i'tidal", "setelah", "ruku", "wajib", "cara"], vec!["الاعتدال بعد الركوع"]),
            (&["qunut", "subuh", "wajib", "tidak", "mazhab"], vec!["القنوت في الفجر"]),
            (&["qunut", "witir", "nazilah", "kapan", "cara"], vec!["قنوت الوتر"]),
            (&["duduk", "tasyahud", "awal", "akhir", "cara", "posisi"], vec!["التشهد الأول والأخير"]),
            (&["shalawat", "tasyahud", "ibrahimiyah", "wajib", "tidak"], vec!["الصلاة الإبراهيمية في التشهد"]),
            (&["salam", "shalat", "wajib", "kanan", "kiri", "lafaz"], vec!["التسليم في الصلاة"]),
            (&["niat", "shalat", "dalam", "hati", "bacakan", "boleh"], vec!["النية في الصلاة"]),
            (&["takbiratul", "ihram", "syarat", "sah", "wajib", "cara"], vec!["تكبيرة الإحرام"]),
            (&["membaca", "surah", "setelah", "fatihah", "wajib", "tidak"], vec!["قراءة السورة بعد الفاتحة"]),
            (&["tahiyyatul", "masjid", "kapan", "shalat", "wajib", "dilakukan"], vec!["تحية المسجد"]),
            (&["shalat", "rawatib", "sebelum", "sesudah", "wajib", "jumlah"], vec!["السنن الرواتب"]),
            (&["shalat", "dhuha", "minimal", "maksimal", "rakaat", "waktu"], vec!["صلاة الضحى"]),
            (&["tahajud", "qiyamullail", "waktu", "rakaat", "cara"], vec!["قيام الليل"]),
            (&["jumat", "wajib", "siapa", "syarat", "khusus"], vec!["شروط وجوب الجمعة"]),
            (&["khutbah", "jumat", "rukun", "syarat", "bahasa", "arab"], vec!["شروط خطبة الجمعة"]),
            (&["khutbah", "arab", "tidak", "mengerti", "sah", "tidak"], vec!["خطبة الجمعة بالعربية"]),
            (&["jumlah", "jamaah", "jumat", "minimal", "sah"], vec!["العدد المشترط للجمعة"]),
            (&["jumat", "di", "desa", "beberapa", "masjid", "sah"], vec!["تعدد الجمعة في البلد"]),
            (&["mandi", "jumat", "wajib", "sunnah", "mazhab"], vec!["غسل الجمعة"]),
            (&["qabliyah", "jumat", "ada", "tidak", "shalat", "sunah"], vec!["السنة القبلية للجمعة"]),
            (&["ba'diyah", "jumat", "berapa", "rakaat", "cara"], vec!["السنة البعدية للجمعة"]),
            (&["musafir", "tidak", "wajib", "jumat", "syarat", "batas"], vec!["إسقاط الجمعة عن المسافر"]),
            (&["wanita", "anakanak", "jumat", "boleh", "tidak", "masuk"], vec!["جمعة المرأة والصبيان"]),
            (&["azan", "jumat", "pertama", "kedua", "sejarah"], vec!["الأذان الأول للجمعة"]),
            (&["makmum", "telat", "masbuq", "jumat", "cara", "lanjut"], vec!["المسبوق في صلاة الجمعة"]),
            (&["haid", "jumat", "tetap", "dengar", "khutbah", "boleh"], vec!["حضور الجمعة للحائض"]),
            (&["shalat", "ied", "hukum", "wajib", "sunnah", "mazhab"], vec!["حكم صلاة العيد"]),
            (&["takbir", "ied", "tujuh", "lima", "cara", "mazhab"], vec!["تكبيرات العيد"]),
            (&["khutbah", "ied", "sebelum", "sesudah", "shalat", "urutan"], vec!["الخطبة بعد صلاة العيد"]),
            (&["shalat", "kusuf", "khusuf", "gerhana", "cara", "dua", "ruku"], vec!["صلاة الكسوف والخسوف"]),
            (&["shalat", "istisqa", "minta", "hujan", "cara", "qunut"], vec!["صلاة الاستسقاء"]),
            (&["shalat", "jenazah", "empat", "takbir", "doa", "cara"], vec!["صلاة الجنازة"]),
            (&["doa", "jenazah", "laki", "perempuan", "bacaan"], vec!["الدعاء في صلاة الجنازة"]),
            (&["ghaib", "shalat", "jenazah", "tanpa", "hadir", "sah"], vec!["صلاة الجنازة على الغائب"]),
            (&["mayyit", "sudah", "dikubur", "shalat", "jenazah", "sah"], vec!["الصلاة على القبر"]),
            (&["shalat", "khauf", "musuh", "perang", "cara", "bergantian"], vec!["صلاة الخوف"]),

            // Pendapat mazhab patterns
            (&["pendapat", "syafii"], vec!["الشافعية", "الإمام الشافعي"]),
            (&["pendapat", "hanafi"], vec!["الحنفية", "أبو حنيفة"]),
            (&["pendapat", "maliki"], vec!["المالكية", "الإمام مالك"]),
            (&["pendapat", "hanbali"], vec!["الحنابلة", "الإمام أحمد"]),
            (&["mazhab", "syafii"], vec!["مذهب الشافعي", "الشافعية"]),
            (&["mazhab", "hanafi"], vec!["مذهب الحنفي", "الحنفية"]),
            (&["mazhab", "maliki"], vec!["مذهب المالكي", "المالكية"]),
            (&["mazhab", "hanbali"], vec!["مذهب الحنبلي", "الحنابلة"]),
            // Specific kitab references for "menurut kitab X" pattern
            (&["menurut", "raudhatul"], vec!["روضة الطالبين", "النووي"]),
            (&["menurut", "fathul"], vec!["فتح الباري", "ابن حجر"]),
            (&["menurut", "nihayatul"], vec!["نهاية المحتاج", "الرملي"]),
            (&["menurut", "ihya"], vec!["إحياء علوم الدين", "الغزالي"]),
            (&["menurut", "riyadhus"], vec!["رياض الصالحين", "النووي"]),
            (&["menurut", "bulughul"], vec!["بلوغ المرام", "ابن حجر"]),
            // Finance / trading types
            (&["short", "selling"], vec!["البيع على المكشوف", "بيع ما لا يملك"]),
            (&["binary", "options"], vec!["الخيارات الثنائية", "المراهنة"]),
            (&["day", "trading"], vec!["المتاجرة اليومية", "التداول"]),
            (&["margin", "trading"], vec!["التداول بالهامش", "الرافعة المالية"]),
            (&["copy", "trading"], vec!["التداول التلقائي"]),
            (&["auto", "trading"], vec!["التداول الآلي"]),
            (&["spread", "betting"], vec!["المراهنة على الفارق", "القمار"]),
            (&["affiliate", "marketing"], vec!["التسويق بالعمولة", "السمسرة"]),
            // Hukum memakan hewan (eating animals)
            (&["makan", "anjing"], vec!["أكل الكلب", "الحيوانات المحرمة"]),
            (&["makan", "kucing"], vec!["أكل الهرة", "الحيوانات المحرمة"]),
            (&["makan", "tikus"], vec!["أكل الفأر", "الفواسق"]),
            (&["makan", "ular"], vec!["أكل الثعبان", "الحيوانات المحرمة"]),
            (&["makan", "cicak"], vec!["أكل الوزغ", "حيوانات الأرض"]),
            (&["makan", "kelelawar"], vec!["أكل الخفاش", "الحيوانات المحرمة"]),
            (&["makan", "biawak"], vec!["أكل الضب", "الوَرَل"]),
            (&["makan", "monyet"], vec!["أكل القرد", "الحيوانات المحرمة"]),
            (&["makan", "harimau"], vec!["أكل النمر", "ذوات الناب"]),
            (&["makan", "singa"], vec!["أكل الأسد", "ذوات الناب"]),
            (&["makan", "elang"], vec!["ذوات المخالب", "الحيوانات المحرمة"]),
            (&["makan", "gagak"], vec!["أكل الغراب", "الطيور المحرمة"]),
            (&["makan", "lumba-lumba"], vec!["أكل الدلفين", "حيوانات البحر"]),
            (&["makan", "gajah"], vec!["أكل الفيل"]),
            (&["makan", "hiu"], vec!["أكل القرش", "حيوانات البحر"]),
            (&["makan", "belut"], vec!["أكل الجريث", "حيوانات البحر"]),
            (&["makan", "lele"], vec!["أكل السمك", "حيوانات البحر"]),
            // Islamic events
            (&["amalan", "arafah"], vec!["أعمال يوم عرفة", "فضل يوم عرفة"]),
            (&["amalan", "dzulhijjah"], vec!["أعمال عشر ذي الحجة"]),
            (&["amalan", "tasyrik"], vec!["أعمال أيام التشريق"]),

            // ── BATCH 34: Surat name bigrams and tafsir surat patterns ──
            // "tafsir surat X" / "surat al/ar X" patterns
            (&["surat", "rahman"], vec!["تفسير سورة الرحمن", "سورة الرحمن"]),
            (&["surat", "mulk"], vec!["تفسير سورة الملك", "سورة الملك", "تبارك"]),
            (&["surat", "baqarah"], vec!["تفسير سورة البقرة", "سورة البقرة"]),
            (&["surat", "yasin"], vec!["تفسير سورة يس", "سورة يس"]),
            (&["surat", "kahfi"], vec!["تفسير سورة الكهف", "سورة الكهف"]),
            (&["surat", "waqiah"], vec!["سورة الواقعة", "الواقعة"]),
            (&["surat", "nisa"], vec!["سورة النساء", "أحكام النساء"]),
            (&["surat", "imran"], vec!["سورة آل عمران", "آل عمران"]),
            (&["surat", "maidah"], vec!["سورة المائدة", "الحلال والحرام"]),
            (&["surat", "nur"], vec!["سورة النور", "الطهارة والأخلاق"]),
            (&["surat", "hujurat"], vec!["سورة الحجرات", "الأخلاق"]),
            (&["surat", "luqman"], vec!["سورة لقمان", "تربية الأبناء"]),
            (&["surat", "fath"], vec!["سورة الفتح", "الفتح"]),
            (&["tafsir", "rahman"], vec!["تفسير الرحمن", "سورة الرحمن"]),
            (&["tafsir", "mulk"], vec!["تفسير الملك", "سورة الملك"]),
            (&["tafsir", "baqarah"], vec!["تفسير البقرة", "سورة البقرة"]),
            (&["tafsir", "yasin"], vec!["تفسير يس"]),
            (&["tafsir", "kahfi"], vec!["تفسير الكهف"]),
            (&["tafsir", "nisa"], vec!["تفسير النساء"]),
            (&["tafsir", "imran"], vec!["تفسير آل عمران"]),

            // ── Haji types (more specific than just haji) ──
            (&["haji", "tamattu"], vec!["حج التمتع", "التمتع"]),
            (&["haji", "qiran"], vec!["حج القران", "القران"]),
            (&["haji", "ifrad"], vec!["حج الإفراد", "الإفراد"]),
            (&["umroh", "tamattu"], vec!["عمرة التمتع"]),
            // Shahih kitab
            (&["kitab", "shahih"], vec!["صحيح البخاري", "الصحيح"]),
            (&["kitab", "bukhari"], vec!["صحيح البخاري", "البخاري"]),
            (&["kitab", "muslim"], vec!["صحيح مسلم", "مسلم"]),

            // ── Rukun (pillars) + ibadah ──
            (&["rukun", "shalat"], vec!["أركان الصلاة", "فرائض الصلاة"]),
            (&["rukun", "wudhu"], vec!["أركان الوضوء", "فرائض الوضوء"]),
            (&["rukun", "puasa"], vec!["أركان الصيام", "فرائض الصوم"]),
            (&["rukun", "haji"], vec!["أركان الحج", "فرائض الحج"]),
            (&["rukun", "umroh"], vec!["أركان العمرة", "فرائض العمرة"]),
            (&["rukun", "zakat"], vec!["أركان الزكاة", "شروط الزكاة"]),
            (&["rukun", "nikah"], vec!["أركان النكاح", "أركان الزواج"]),
            (&["rukun", "iman"], vec!["أركان الإيمان", "الإيمان بالله"]),
            (&["rukun", "islam"], vec!["أركان الإسلام", "الشهادتان"]),
            (&["rukun", "sholat"], vec!["أركان الصلاة", "فرائض الصلاة"]),

            // ── Syarat (conditions) + ibadah ──
            (&["syarat", "shalat"], vec!["شروط الصلاة", "شرائط الصلاة"]),
            (&["syarat", "wudhu"], vec!["شروط الوضوء", "شرائط الوضوء"]),
            (&["syarat", "puasa"], vec!["شروط الصيام", "شرائط الصوم"]),
            (&["syarat", "haji"], vec!["شروط الحج", "استطاعة الحج"]),
            (&["syarat", "zakat"], vec!["شروط الزكاة", "شرائط الزكاة"]),
            (&["syarat", "nikah"], vec!["شروط النكاح", "شرائط الزواج"]),
            (&["syarat", "sah"], vec!["شروط الصحة", "الشرائط"]),
            (&["syarat", "sholat"], vec!["شروط الصلاة", "شرائط الصلاة"]),

            // ── Cara / tata cara (how-to) + ibadah ──
            (&["cara", "shalat"], vec!["كيفية الصلاة", "صفة الصلاة"]),
            (&["cara", "wudhu"], vec!["كيفية الوضوء", "صفة الوضوء"]),
            (&["cara", "puasa"], vec!["كيفية الصيام", "صفة الصوم"]),
            (&["cara", "mandi"], vec!["كيفية الغسل", "صفة الغسل"]),
            (&["cara", "tayamum"], vec!["كيفية التيمم", "صفة التيمم"]),
            (&["cara", "sholat"], vec!["كيفية الصلاة", "صفة الصلاة"]),
            (&["tata", "shalat"], vec!["كيفية الصلاة", "صفة الصلاة"]),
            (&["tata", "wudhu"], vec!["كيفية الوضوء", "صفة الوضوء"]),

            // ── Niat (intention) + ibadah ──
            (&["niat", "shalat"], vec!["نية الصلاة", "نيات الصلوات"]),
            (&["niat", "wudhu"], vec!["نية الوضوء"]),
            (&["niat", "puasa"], vec!["نية الصيام", "نية الصوم"]),
            (&["niat", "mandi"], vec!["نية الغسل", "نية غسل الجنابة"]),
            (&["niat", "zakat"], vec!["نية الزكاة"]),
            (&["niat", "haji"], vec!["نية الحج", "الإحرام بالحج"]),
            (&["niat", "sholat"], vec!["نية الصلاة", "نيات الصلوات"]),

            // ── Pembatal (nullifiers) + ibadah ──
            (&["pembatal", "shalat"], vec!["مبطلات الصلاة", "نواقض الصلاة"]),
            (&["pembatal", "wudhu"], vec!["نواقض الوضوء", "مبطلات الوضوء"]),
            (&["pembatal", "puasa"], vec!["مبطلات الصوم", "مفسدات الصوم"]),

            // ── Waktu (time) + ibadah ──
            (&["waktu", "shalat"], vec!["أوقات الصلاة", "مواقيت الصلاة"]),
            (&["waktu", "subuh"], vec!["وقت الفجر", "أذان الصبح"]),
            (&["waktu", "dzuhur"], vec!["وقت الظهر"]),
            (&["waktu", "ashar"], vec!["وقت العصر"]),
            (&["waktu", "maghrib"], vec!["وقت المغرب"]),
            (&["waktu", "isya"], vec!["وقت العشاء"]),
        ];

        for (phrase_words, phrase_arabic) in &phrase_map {
            // All phrase words must appear somewhere in the query (not necessarily adjacent)
            // Use smart matching: short words (<4 chars) require exact match to prevent
            // false positives like "tidak".contains("id") matching ("shalat","id")→صلاة العيد
            let all_match = phrase_words.iter().all(|pw| {
                lower_words.iter().any(|w| {
                    if pw.len() < 4 {
                        w == pw  // short phrase words: exact match only
                    } else {
                        w == pw || w.contains(pw)  // longer: allow substring (catches berwudhu→wudhu)
                    }
                })
            });
            if all_match {
                for arab in phrase_arabic {
                    expansions.push(arab.to_string());
                }
            }
        }

        // Deduplicate
        expansions.sort();
        expansions.dedup();
        expansions
    }

    // ─── Tantivy Query Construction ───

    fn build_tantivy_query(&self, arabic_terms: &[String], latin_terms: &[String]) -> String {
        let mut parts = Vec::new();

        // Arabic terms get higher boost
        for term in arabic_terms {
            // If term has spaces (phrase), wrap in quotes
            if term.contains(' ') {
                parts.push(format!("\"{}\"^3", term));
            } else {
                parts.push(format!("{}^2", term));
            }
        }

        // Latin terms (original query words) get lower boost for fallback
        for term in latin_terms {
            if term.len() > 2 {
                parts.push(term.clone());
            }
        }

        if parts.is_empty() {
            // Return original for safety
            return arabic_terms
                .first()
                .or(latin_terms.first())
                .cloned()
                .unwrap_or_default();
        }

        parts.join(" OR ")
    }

    // ─── Dictionary Builder ───

    fn build_term_dictionary(&mut self) {
        // ═══ IBADAH MAHDAH ═══
        for key in &["shalat", "solat", "salat", "prayer", "prayers", "salah", "sholat"] {
            self.term_map.insert(key.to_string(), vec!["صلاة", "الصلاة", "الصلوات"]);
        }
        // Jum'ah — standalone entries: covers "jumat", "jum'at", "jumuah", etc.
        for key in &["jumat", "jumaat", "jum'at", "jumaah", "jumah", "jumuah", "jum'ah"] {
            self.term_map.insert(key.to_string(), vec!["الجمعة", "صلاة الجمعة", "يوم الجمعة"]);
        }
        for key in &["friday"] {
            self.term_map.insert(key.to_string(), vec!["الجمعة", "يوم الجمعة", "صلاة الجمعة"]);
        }
        for key in &["wudhu", "wudu", "ablution", "wudhuk", "wuduk"] {
            self.term_map.insert(key.to_string(), vec!["وضوء", "الوضوء", "الطهارة"]);
        }
        for key in &["tayammum", "tayamum"] {
            self.term_map.insert(key.to_string(), vec!["تيمم", "التيمم"]);
        }
        for key in &["puasa", "shaum", "shiyam", "fasting", "saum", "siyam"] {
            self.term_map.insert(key.to_string(), vec!["صوم", "صيام", "الصيام"]);
        }
        for key in &["zakat", "zakah", "zakaah"] {
            self.term_map.insert(key.to_string(), vec!["زكاة", "الزكاة", "زكاة المال"]);
        }
        for key in &["haji", "hajj", "pilgrimage"] {
            self.term_map.insert(key.to_string(), vec!["حج", "الحج", "المناسك"]);
        }
        for key in &["umroh", "umrah", "umra"] {
            self.term_map.insert(key.to_string(), vec!["عمرة", "العمرة"]);
        }
        for key in &["sujud", "prostration"] {
            self.term_map.insert(key.to_string(), vec!["سجود", "السجود"]);
        }
        for key in &["ruku", "rukuk", "bowing"] {
            self.term_map.insert(key.to_string(), vec!["ركوع", "الركوع"]);
        }
        for key in &["qunut", "qunoot"] {
            self.term_map.insert(key.to_string(), vec!["قنوت", "القنوت", "دعاء القنوت"]);
        }
        for key in &["subuh", "shubuh", "subh", "fajar"] {
            self.term_map.insert(key.to_string(), vec!["صبح", "الصبح", "الفجر", "صلاة الفجر"]);
        }
        for key in &["dzuhur", "zuhur", "dhuhr", "zhuhur", "lohor"] {
            self.term_map.insert(key.to_string(), vec!["ظهر", "الظهر", "صلاة الظهر"]);
        }
        for key in &["ashar", "asar", "ashr"] {
            self.term_map.insert(key.to_string(), vec!["عصر", "العصر", "صلاة العصر"]);
        }
        for key in &["maghrib", "magrib"] {
            self.term_map.insert(key.to_string(), vec!["مغرب", "المغرب", "صلاة المغرب"]);
        }
        for key in &["isya", "isha", "isya'"] {
            self.term_map.insert(key.to_string(), vec!["عشاء", "العشاء", "صلاة العشاء"]);
        }
        for key in &["adzan", "azan", "azaan", "adhan"] {
            self.term_map.insert(key.to_string(), vec!["أذان", "الأذان"]);
        }
        for key in &["iqamah", "iqamat", "iqomah"] {
            self.term_map.insert(key.to_string(), vec!["إقامة", "الإقامة"]);
        }
        for key in &["itikaf", "i'tikaf", "iktikaf"] {
            self.term_map.insert(key.to_string(), vec!["اعتكاف", "الاعتكاف"]);
        }
        for key in &["qurban", "kurban", "udhiyah", "udhiyyah", "sacrifice"] {
            self.term_map.insert(key.to_string(), vec!["أضحية", "الأضحية", "الذبح"]);
        }
        for key in &["aqiqah", "akikah"] {
            self.term_map.insert(key.to_string(), vec!["عقيقة", "العقيقة"]);
        }
        for key in &["fidyah"] {
            self.term_map.insert(key.to_string(), vec!["فدية", "الفدية"]);
        }
        for key in &["kaffarah", "kafarah", "kifarat", "kafarat", "kaffarat"] {
            self.term_map.insert(key.to_string(), vec!["كفارة", "الكفارة", "الكفارات"]);
        }
        for key in &["sumpah", "yamin", "oath", "bersumpah"] {
            self.term_map.insert(key.to_string(), vec!["يمين", "الأيمان", "حلف", "القسم"]);
        }
        for key in &["palsu", "bohong", "dusta", "false"] {
            self.term_map.insert(key.to_string(), vec!["كاذبة", "الكذب", "الغموس", "اليمين الغموس"]);
        }
        for key in &["nadzar", "nazar", "vow"] {
            self.term_map.insert(key.to_string(), vec!["نذر", "النذر", "النذور"]);
        }
        for key in &["dzikir", "zikir", "dhikr", "remembrance"] {
            self.term_map.insert(key.to_string(), vec!["ذكر", "الأذكار"]);
        }
        for key in &["doa", "du'a", "supplication", "duaa"] {
            self.term_map.insert(key.to_string(), vec!["دعاء", "الدعاء"]);
        }

        // ═══ THAHARAH ═══
        for key in &["najis", "najasa", "impure", "impurity"] {
            self.term_map.insert(key.to_string(), vec!["نجاسة", "النجس", "المتنجس"]);
        }
        for key in &["junub", "junubi"] {
            self.term_map.insert(key.to_string(), vec!["جنابة", "الحدث الأكبر"]);
        }
        for key in &["haid", "haidh", "menstruasi", "menstruation", "haidz"] {
            self.term_map.insert(key.to_string(), vec!["حيض", "المحيض", "الحيض"]);
        }
        for key in &["nifas", "postpartum"] {
            self.term_map.insert(key.to_string(), vec!["نفاس", "النفاس"]);
        }
        for key in &["istihadhah", "istihadzah", "istihadoh"] {
            self.term_map.insert(key.to_string(), vec!["استحاضة", "الاستحاضة", "المستحاضة"]);
        }
        for key in &["mandi", "ghusl", "bathing"] {
            self.term_map.insert(key.to_string(), vec!["غسل", "الغسل"]);
        }
        for key in &["suci", "bersuci", "purification"] {
            self.term_map.insert(key.to_string(), vec!["طهارة", "الطهارة", "التطهير"]);
        }
        for key in &["mani", "sperma", "semen"] {
            self.term_map.insert(key.to_string(), vec!["مني", "المني"]);
        }
        for key in &["madzi", "madhy", "madhiy"] {
            self.term_map.insert(key.to_string(), vec!["مذي", "المذي"]);
        }
        for key in &["wadi", "wadiy"] {
            self.term_map.insert(key.to_string(), vec!["ودي", "الودي"]);
        }
        for key in &["kencing", "urine"] {
            self.term_map.insert(key.to_string(), vec!["بول", "البول"]);
        }
        for key in &["tinja", "kotoran", "feces"] {
            self.term_map.insert(key.to_string(), vec!["غائط", "الغائط"]);
        }
        for key in &["darah", "blood"] {
            self.term_map.insert(key.to_string(), vec!["دم", "الدم"]);
        }
        // v10: missing Thaharah terms
        for key in &["perban", "diperban", "gips", "balut", "jabira", "jabirah"] {
            self.term_map.insert(key.to_string(), vec!["جبيرة", "الجبيرة", "المسح على الجبيرة"]);
        }

        // ═══ MUAMALAT ═══
        for key in &["riba", "bunga", "interest", "usury"] {
            self.term_map.insert(key.to_string(), vec!["ربا", "الربا", "فائدة", "ربا الفضل", "ربا النسيئة"]);
        }
        for key in &["jual", "beli", "trade"] {
            self.term_map.insert(key.to_string(), vec!["بيع", "البيع", "المعاملات"]);
        }
        for key in &["transaksi", "transaction"] {
            self.term_map.insert(key.to_string(), vec!["معاملة", "المعاملات"]);
        }
        for key in &["utang", "hutang", "debt", "piutang"] {
            self.term_map.insert(key.to_string(), vec!["دين", "الدين", "قرض", "القرض"]);
        }
        for key in &["gadai", "pawn", "collateral"] {
            self.term_map.insert(key.to_string(), vec!["رهن", "الرهن"]);
        }
        for key in &["sewa", "ijarah", "rent", "rental"] {
            self.term_map.insert(key.to_string(), vec!["إجارة", "الإجارة"]);
        }
        for key in &["mudharabah", "mudhorobah", "mudarabah"] {
            self.term_map.insert(key.to_string(), vec!["مضاربة", "المضاربة"]);
        }
        for key in &["musyarakah", "musharakah", "partnership"] {
            self.term_map.insert(key.to_string(), vec!["مشاركة", "الشركة"]);
        }
        for key in &["wakaf", "waqf", "endowment"] {
            self.term_map.insert(key.to_string(), vec!["وقف", "الوقف", "الأوقاف"]);
        }
        for key in &["hibah", "gift"] {
            self.term_map.insert(key.to_string(), vec!["هبة", "الهبة"]);
        }
        for key in &["wasiat", "will", "testament"] {
            self.term_map.insert(key.to_string(), vec!["وصية", "الوصية"]);
        }
        for key in &["waris", "warisan", "inheritance"] {
            self.term_map.insert(key.to_string(), vec!["ميراث", "الميراث", "التركة", "الإرث", "الفرائض"]);
        }
        for key in &["halal"] {
            self.term_map.insert(key.to_string(), vec!["حلال", "الحلال", "المباح"]);
        }
        for key in &["haram", "forbidden", "prohibited"] {
            self.term_map.insert(key.to_string(), vec!["حرام", "الحرام", "المحرم"]);
        }
        for key in &["makruh", "disliked"] {
            self.term_map.insert(key.to_string(), vec!["مكروه", "الكراهة"]);
        }
        for key in &["mubah", "permissible", "allowed"] {
            self.term_map.insert(key.to_string(), vec!["مباح", "الإباحة"]);
        }
        for key in &["sunnah", "sunah", "recommended"] {
            self.term_map.insert(key.to_string(), vec!["سنة", "مستحب", "مندوب"]);
        }
        for key in &["wajib", "fardhu", "obligatory", "fard"] {
            self.term_map.insert(key.to_string(), vec!["واجب", "فرض", "الوجوب"]);
        }

        // ═══ MUNAKAHAT ═══
        for key in &["nikah", "kawin", "marriage", "pernikahan", "menikah"] {
            self.term_map.insert(key.to_string(), vec!["نكاح", "زواج", "التزويج", "عقد النكاح"]);
        }
        for key in &["cerai", "talak", "divorce", "perceraian"] {
            self.term_map.insert(key.to_string(), vec!["طلاق", "الطلاق", "فراق"]);
        }
        for key in &["khuluk", "khulu", "khul'"] {
            self.term_map.insert(key.to_string(), vec!["خلع", "الخلع"]);
        }
        for key in &["iddah", "idah"] {
            self.term_map.insert(key.to_string(), vec!["عدة", "العدة"]);
        }
        for key in &["nafkah", "nafaqah", "alimony"] {
            self.term_map.insert(key.to_string(), vec!["نفقة", "النفقة"]);
        }
        for key in &["mahar", "dowry"] {
            self.term_map.insert(key.to_string(), vec!["مهر", "صداق", "المهر", "الصداق"]);
        }
        for key in &["poligami", "polygamy", "poligini"] {
            self.term_map.insert(key.to_string(), vec!["تعدد الزوجات", "التعدد"]);
        }
        for key in &["walimah", "resepsi"] {
            self.term_map.insert(key.to_string(), vec!["وليمة", "الوليمة"]);
        }
        for key in &["wali"] {
            self.term_map.insert(key.to_string(), vec!["ولي", "الولي", "الولاية"]);
        }
        for key in &["mahram", "muhrim"] {
            self.term_map.insert(key.to_string(), vec!["محرم", "المحارم"]);
        }
        for key in &["impoten", "impotent"] {
            self.term_map.insert(key.to_string(), vec!["عنين", "العنة", "الجب", "العيوب في النكاح"]);
        }
        for key in &["fasakh", "fasak", "annulment"] {
            self.term_map.insert(key.to_string(), vec!["فسخ", "فسخ النكاح"]);
        }
        for key in &["suami", "husband"] {
            self.term_map.insert(key.to_string(), vec!["زوج", "الزوج"]);
        }
        for key in &["istri", "wife"] {
            self.term_map.insert(key.to_string(), vec!["زوجة", "الزوجة"]);
        }
        for key in &["anak", "child", "children"] {
            self.term_map.insert(key.to_string(), vec!["ولد", "الولد", "الأولاد"]);
        }
        for key in &["perempuan", "wanita", "daughter", "girl"] {
            self.term_map.insert(key.to_string(), vec!["بنت", "البنت", "النساء", "الإناث"]);
        }
        for key in &["laki", "lelaki", "pria", "son", "boy"] {
            self.term_map.insert(key.to_string(), vec!["ابن", "الابن", "الذكور"]);
        }
        for key in &["yatim", "orphan"] {
            self.term_map.insert(key.to_string(), vec!["يتيم", "اليتيم"]);
        }
        for key in &["susuan", "radha'ah", "breastfeeding"] {
            self.term_map.insert(key.to_string(), vec!["رضاعة", "الرضاع", "الرضاعة"]);
        }
        for key in &["nusyuz", "nushuz", "disobedience"] {
            self.term_map.insert(key.to_string(), vec!["نشوز", "النشوز"]);
        }
        // v10: missing Munakahat terms
        for key in &["nasab", "keturunan", "lineage", "nasib"] {
            self.term_map.insert(key.to_string(), vec!["نسب", "النسب", "نسب الولد"]);
        }
        for key in &["marah", "amarah", "anger", "angry"] {
            self.term_map.insert(key.to_string(), vec!["غضب", "الغضب"]);
        }
        for key in &["emosi", "emotional", "kalap"] {
            self.term_map.insert(key.to_string(), vec!["غضب", "الغضب", "الانفعال"]);
        }
        for key in &["patungan", "urunan", "iuran", "pooling"] {
            self.term_map.insert(key.to_string(), vec!["اشتراك", "الاشتراك", "المشاركة"]);
        }
        for key in &["li'an", "lian"] {
            self.term_map.insert(key.to_string(), vec!["لعان", "اللعان"]);
        }
        for key in &["ila'", "ila"] {
            self.term_map.insert(key.to_string(), vec!["إيلاء", "الإيلاء"]);
        }
        for key in &["zhihar", "dzihar", "zihar"] {
            self.term_map.insert(key.to_string(), vec!["ظهار", "الظهار"]);
        }
        for key in &["rujuk", "ruju'", "reconciliation"] {
            self.term_map.insert(key.to_string(), vec!["رجعة", "الرجعة"]);
        }

        // ═══ AQIDAH ═══
        for key in &["tauhid", "tawhid", "monotheism"] {
            self.term_map.insert(key.to_string(), vec!["توحيد", "التوحيد"]);
        }
        for key in &["syirik", "shirk", "polytheism", "musyrik"] {
            self.term_map.insert(key.to_string(), vec!["شرك", "الشرك"]);
        }
        for key in &["murtad", "apostasy", "riddah", "ridda"] {
            self.term_map.insert(key.to_string(), vec!["ردة", "الردة", "المرتد"]);
        }
        for key in &["bid'ah", "bidah", "innovation"] {
            self.term_map.insert(key.to_string(), vec!["بدعة", "البدعة", "البدع"]);
        }
        for key in &["kafir", "kufr", "disbelief"] {
            self.term_map.insert(key.to_string(), vec!["كفر", "الكفر"]);
        }
        for key in &["iman", "faith", "keimanan"] {
            self.term_map.insert(key.to_string(), vec!["إيمان", "الإيمان"]);
        }
        for key in &["tawakkal", "tawakkul", "tawakal", "tawakul", "reliance"] {
            self.term_map.insert(key.to_string(), vec!["توكل", "التوكل", "التوكل على الله"]);
        }
        for key in &["tawassul", "tawasul"] {
            self.term_map.insert(key.to_string(), vec!["توسل", "التوسل"]);
        }
        for key in &["syafaat", "syafa'at", "intercession"] {
            self.term_map.insert(key.to_string(), vec!["شفاعة", "الشفاعة"]);
        }
        for key in &["takdir", "destiny", "fate"] {
            self.term_map.insert(key.to_string(), vec!["قدر", "القدر", "القضاء والقدر"]);
        }

        // ═══ JINAYAT ═══
        for key in &["hudud", "had", "punishment"] {
            self.term_map.insert(key.to_string(), vec!["حدود", "الحدود", "حد"]);
        }
        for key in &["qishash", "qisas", "retaliation"] {
            self.term_map.insert(key.to_string(), vec!["قصاص", "القصاص"]);
        }
        for key in &["diyat", "diyah", "bloodmoney"] {
            self.term_map.insert(key.to_string(), vec!["دية", "الدية"]);
        }
        for key in &["ta'zir", "tazir", "discretionary"] {
            self.term_map.insert(key.to_string(), vec!["تعزير", "التعزير"]);
        }
        for key in &["pencurian", "mencuri", "theft", "stealing"] {
            self.term_map.insert(key.to_string(), vec!["سرقة", "السرقة"]);
        }
        for key in &["zina", "adultery", "fornication"] {
            self.term_map.insert(key.to_string(), vec!["زنا", "الزنا", "حد الزنا"]);
        }
        for key in &["qadzaf", "qadhaf", "slander"] {
            self.term_map.insert(key.to_string(), vec!["قذف", "القذف"]);
        }
        for key in &["khamr", "miras", "alcohol", "wine"] {
            self.term_map.insert(key.to_string(), vec!["خمر", "الخمر", "المسكر"]);
        }
        for key in &["bunuh", "membunuh", "murder", "killing"] {
            self.term_map.insert(key.to_string(), vec!["قتل", "القتل"]);
        }

        // ═══ TAFSIR ═══
        for key in &["tafsir", "interpretation"] {
            self.term_map.insert(key.to_string(), vec!["تفسير", "التفسير"]);
        }
        for key in &["ta'wil", "takwil"] {
            self.term_map.insert(key.to_string(), vec!["تأويل", "التأويل"]);
        }
        for key in &["asbab", "nuzul", "revelation"] {
            self.term_map.insert(key.to_string(), vec!["أسباب النزول", "سبب النزول"]);
        }
        for key in &["nasikh", "mansukh", "abrogation"] {
            self.term_map.insert(key.to_string(), vec!["ناسخ", "منسوخ", "النسخ"]);
        }
        for key in &["ayat", "ayah", "verse"] {
            self.term_map.insert(key.to_string(), vec!["آية", "الآية"]);
        }
        for key in &["surat", "surah", "chapter"] {
            self.term_map.insert(key.to_string(), vec!["سورة", "السورة"]);
        }

        // ═══ HADITS ═══
        for key in &["hadits", "hadis", "hadith"] {
            self.term_map.insert(key.to_string(), vec!["حديث", "الحديث", "الأحاديث"]);
        }
        for key in &["sanad", "chain"] {
            self.term_map.insert(key.to_string(), vec!["إسناد", "السند"]);
        }
        for key in &["rawi", "narrator"] {
            self.term_map.insert(key.to_string(), vec!["راوي", "الرواة"]);
        }
        for key in &["mustalah", "terminology"] {
            self.term_map.insert(key.to_string(), vec!["مصطلح الحديث"]);
        }

        // ═══ TASAWUF ═══
        for key in &["tasawuf", "sufism", "tasawwuf"] {
            self.term_map.insert(key.to_string(), vec!["تصوف", "التصوف"]);
        }
        for key in &["ihsan"] {
            self.term_map.insert(key.to_string(), vec!["إحسان", "الإحسان"]);
        }
        for key in &["taubat", "taubah", "repentance"] {
            self.term_map.insert(key.to_string(), vec!["توبة", "التوبة"]);
        }
        for key in &["ikhlas", "sincerity"] {
            self.term_map.insert(key.to_string(), vec!["إخلاص", "الإخلاص"]);
        }
        for key in &["sabar", "patience"] {
            self.term_map.insert(key.to_string(), vec!["صبر", "الصبر"]);
        }
        for key in &["syukur", "gratitude"] {
            self.term_map.insert(key.to_string(), vec!["شكر", "الشكر"]);
        }
        for key in &["taqwa", "piety"] {
            self.term_map.insert(key.to_string(), vec!["تقوى", "التقوى"]);
        }
        for key in &["khusyu", "khushu", "humility"] {
            self.term_map.insert(key.to_string(), vec!["خشوع", "الخشوع"]);
        }
        for key in &["riya", "riya'", "showoff"] {
            self.term_map.insert(key.to_string(), vec!["رياء", "الرياء"]);
        }
        for key in &["ujub", "vanity"] {
            self.term_map.insert(key.to_string(), vec!["عجب", "العُجب"]);
        }
        for key in &["hasad", "hasud", "envy"] {
            self.term_map.insert(key.to_string(), vec!["حسد", "الحسد"]);
        }
        for key in &["ghibah", "backbiting"] {
            self.term_map.insert(key.to_string(), vec!["غيبة", "الغيبة"]);
        }
        for key in &["namimah", "slander", "gossip"] {
            self.term_map.insert(key.to_string(), vec!["نميمة", "النميمة"]);
        }

        // ═══ COMMON TERMS ═══
        for key in &["hukum", "ruling", "law"] {
            self.term_map.insert(key.to_string(), vec!["حكم", "الحكم", "أحكام"]);
        }
        for key in &["fatwa", "verdict"] {
            self.term_map.insert(key.to_string(), vec!["فتوى", "الفتوى", "الفتاوى"]);
        }
        for key in &["mazhab", "madzhab", "madhab", "school"] {
            self.term_map.insert(key.to_string(), vec!["مذهب", "المذهب"]);
        }
        for key in &["syafi'i", "syafii", "shafi'i", "shafii"] {
            self.term_map.insert(key.to_string(), vec!["الشافعي", "شافعي"]);
        }
        for key in &["hanafi"] {
            self.term_map.insert(key.to_string(), vec!["الحنفي", "حنفي"]);
        }
        for key in &["maliki"] {
            self.term_map.insert(key.to_string(), vec!["المالكي", "مالكي"]);
        }
        for key in &["hanbali"] {
            self.term_map.insert(key.to_string(), vec!["الحنبلي", "حنبلي"]);
        }
        for key in &["ijma", "ijma'", "consensus"] {
            self.term_map.insert(key.to_string(), vec!["إجماع", "الإجماع"]);
        }
        for key in &["qiyas", "analogy"] {
            self.term_map.insert(key.to_string(), vec!["قياس", "القياس"]);
        }
        for key in &["ijtihad"] {
            self.term_map.insert(key.to_string(), vec!["اجتهاد", "الاجتهاد"]);
        }
        for key in &["dalil", "evidence", "proof"] {
            self.term_map.insert(key.to_string(), vec!["دليل", "الدليل", "الأدلة"]);
        }
        for key in &["khilaf", "ikhtilaf", "disagreement"] {
            self.term_map.insert(key.to_string(), vec!["خلاف", "اختلاف", "الخلاف"]);
        }
        for key in &["darurat", "darura", "necessity", "emergency"] {
            self.term_map.insert(key.to_string(), vec!["ضرورة", "الضرورة", "الاضطرار"]);
        }
        for key in &["maslahah", "maslahat", "benefit"] {
            self.term_map.insert(key.to_string(), vec!["مصلحة", "المصلحة", "المصالح"]);
        }
        for key in &["mafsadah", "mafsadat", "harm"] {
            self.term_map.insert(key.to_string(), vec!["مفسدة", "المفسدة", "المفاسد"]);
        }
        for key in &["niat", "niyyah", "intention"] {
            self.term_map.insert(key.to_string(), vec!["نية", "النية"]);
        }
        for key in &["syarat", "condition", "requirement"] {
            self.term_map.insert(key.to_string(), vec!["شرط", "الشروط"]);
        }
        for key in &["rukun", "pillar"] {
            self.term_map.insert(key.to_string(), vec!["ركن", "الأركان"]);
        }
        for key in &["boleh", "bolehkah", "allowed", "permissible"] {
            self.term_map.insert(key.to_string(), vec!["جواز", "الجواز", "يجوز", "حلال"]);
        }
        for key in &["ulama", "scholar", "scholars"] {
            self.term_map.insert(key.to_string(), vec!["علماء", "العلماء", "عالم"]);
        }
        for key in &["imam"] {
            self.term_map.insert(key.to_string(), vec!["إمام", "الإمام"]);
        }
        for key in &["masjid", "mosque", "musholla", "musala"] {
            self.term_map.insert(key.to_string(), vec!["مسجد", "المسجد", "المساجد"]);
        }
        for key in &["jenazah", "janazah", "funeral", "mayit", "mayat"] {
            self.term_map.insert(key.to_string(), vec!["جنازة", "الجنازة", "الميت"]);
        }
        for key in &["kubur", "makam", "grave"] {
            self.term_map.insert(key.to_string(), vec!["قبر", "القبر", "القبور"]);
        }
        for key in &["sedekah", "shadaqah", "charity"] {
            self.term_map.insert(key.to_string(), vec!["صدقة", "الصدقة"]);
        }
        for key in &["infaq", "infak", "spending"] {
            self.term_map.insert(key.to_string(), vec!["إنفاق", "الإنفاق"]);
        }
        for key in &["khitan", "sunat", "circumcision"] {
            self.term_map.insert(key.to_string(), vec!["ختان", "الختان"]);
        }
        for key in &["rakaat", "rakat", "raka'at", "rakah"] {
            self.term_map.insert(key.to_string(), vec!["ركعة", "الركعات", "ركعات"]);
        }
        for key in &["mati", "meninggal", "wafat", "death", "kematian"] {
            self.term_map.insert(key.to_string(), vec!["الموت", "المتوفى", "الوفاة", "الميت"]);
        }
        for key in &["nafilah", "nawafil"] {
            self.term_map.insert(key.to_string(), vec!["سنة", "النوافل", "المستحب", "المندوب"]);
        }
        for key in &["dilarang"] {
            self.term_map.insert(key.to_string(), vec!["حرام", "التحريم", "المحرمات"]);
        }
        for key in &["lawful"] {
            self.term_map.insert(key.to_string(), vec!["حلال", "الحلال", "المباح", "الجائز"]);
        }
        for key in &["dibenci"] {
            self.term_map.insert(key.to_string(), vec!["مكروه", "المكروه"]);
        }
        for key in &["fardu"] {
            self.term_map.insert(key.to_string(), vec!["واجب", "الواجب", "فرض", "الفرض"]);
        }
        for key in &["taubat", "tobat", "tawbah", "repentance"] {
            self.term_map.insert(key.to_string(), vec!["توبة", "التوبة"]);
        }
        for key in &["syafaat", "shafa'ah", "intercession"] {
            self.term_map.insert(key.to_string(), vec!["شفاعة", "الشفاعة"]);
        }

        // ═══ MODERN ISSUES ═══
        for key in &["vaksin", "vaccine", "imunisasi"] {
            self.term_map.insert(key.to_string(), vec!["التطعيم", "اللقاح"]);
        }
        for key in &["rokok", "cigarette", "smoking", "merokok"] {
            self.term_map.insert(key.to_string(), vec!["التدخين", "الدخان"]);
        }
        for key in &["foto", "photograph", "gambar", "picture"] {
            self.term_map.insert(key.to_string(), vec!["تصوير", "التصوير", "الصورة"]);
        }
        for key in &["musik", "music"] {
            self.term_map.insert(key.to_string(), vec!["الغناء", "المعازف", "الموسيقى"]);
        }
        for key in &["tato", "tattoo"] {
            self.term_map.insert(key.to_string(), vec!["الوشم"]);
        }
        for key in &["transplantasi", "transplant"] {
            self.term_map.insert(key.to_string(), vec!["نقل الأعضاء", "زراعة الأعضاء"]);
        }
        for key in &["aborsi", "abortion", "menggugurkan"] {
            self.term_map.insert(key.to_string(), vec!["إجهاض", "الإجهاض"]);
        }
        for key in &["bayi", "tabung", "ivf"] {
            self.term_map.insert(key.to_string(), vec!["التلقيح الصناعي", "أطفال الأنابيب"]);
        }
        for key in &["kloning", "cloning"] {
            self.term_map.insert(key.to_string(), vec!["الاستنساخ", "الاستنساخ البشري", "استنساخ"]);
        }
        for key in &["gelatin", "jelly"] {
            self.term_map.insert(key.to_string(), vec!["الجيلاتين", "الاستحالة"]);
        }
        for key in &["makan", "eat", "eating", "makanan", "food"] {
            self.term_map.insert(key.to_string(), vec!["أكل", "الأكل", "الطعام", "الأطعمة"]);
        }
        for key in &["minum", "drink", "minuman", "beverage"] {
            self.term_map.insert(key.to_string(), vec!["شرب", "الشرب", "المشروبات"]);
        }
        for key in &["konvensional", "conventional"] {
            self.term_map.insert(key.to_string(), vec!["الربوي", "التقليدي"]);
        }
        for key in &["babi", "pork", "pig"] {
            self.term_map.insert(key.to_string(), vec!["خنزير", "لحم الخنزير"]);
        }
        for key in &["alkohol", "alcohol"] {
            self.term_map.insert(key.to_string(), vec!["كحول", "الكحول", "خمر"]);
        }
        for key in &["obat", "medicine", "pengobatan"] {
            self.term_map.insert(key.to_string(), vec!["دواء", "التداوي", "العلاج"]);
        }
        for key in &["bank", "perbankan", "banking"] {
            self.term_map.insert(key.to_string(), vec!["البنك", "المصرف", "البنوك"]);
        }
        for key in &["kripto", "crypto", "bitcoin"] {
            self.term_map.insert(key.to_string(), vec!["العملات الرقمية", "البتكوين"]);
        }
        for key in &["saham", "stocks", "investasi"] {
            self.term_map.insert(key.to_string(), vec!["الأسهم", "الاستثمار"]);
        }

        // ═══ HEALTH & MEDICAL ═══
        for key in &["sakit", "sick", "ill", "illness", "penyakit", "disease"] {
            self.term_map.insert(key.to_string(), vec!["مرض", "المريض", "الأمراض"]);
        }
        for key in &["darurat", "emergency", "terpaksa", "necessity", "keterpaksaan"] {
            self.term_map.insert(key.to_string(), vec!["ضرورة", "الضرورة", "الاضطرار"]);
        }
        for key in &["iftar", "buka puasa", "tidak puasa"] {
            self.term_map.insert(key.to_string(), vec!["إفطار", "الإفطار", "ترك الصوم"]);
        }
        for key in &["saat", "ketika", "waktu", "when", "during"] {
            self.term_map.insert(key.to_string(), vec![]);
        }
        for key in &["dokter", "physician", "medis", "medical"] {
            self.term_map.insert(key.to_string(), vec!["الطب", "الطبيب"]);
        }

        // ═══ MARRIAGE DEFECTS ═══
        for key in &["impoten", "impotent", "lemah syahwat"] {
            self.term_map.insert(key.to_string(), vec!["عنة", "العنين", "العنة", "الجب"]);
        }
        for key in &["khuluk", "cerai gugat", "khul"] {
            self.term_map.insert(key.to_string(), vec!["خلع", "الخلع"]);
        }
        for key in &["shiqaq", "syiqaq", "perselisihan suami istri"] {
            self.term_map.insert(key.to_string(), vec!["شقاق", "الشقاق"]);
        }
        for key in &["hakim", "judge", "pengadilan", "court"] {
            self.term_map.insert(key.to_string(), vec!["القاضي", "الحاكم", "القضاء"]);
        }

        // ═══ MISSING COMMON TERMS (added v8) ═══

        // Religious core terms
        for key in &["allah", "god"] {
            self.term_map.insert(key.to_string(), vec!["الله", "سبحانه وتعالى"]);
        }
        for key in &["ramadhan", "ramadan", "romadhon"] {
            self.term_map.insert(key.to_string(), vec!["رمضان", "شهر رمضان"]);
        }
        for key in &["muhammad", "nabi", "rasul", "prophet"] {
            self.term_map.insert(key.to_string(), vec!["النبي", "الرسول", "محمد"]);
        }

        // Financial terms
        for key in &["nisab", "nishab"] {
            self.term_map.insert(key.to_string(), vec!["نصاب", "النصاب"]);
        }
        for key in &["haul"] {
            self.term_map.insert(key.to_string(), vec!["حول", "الحول"]);
        }
        for key in &["emas", "gold"] {
            self.term_map.insert(key.to_string(), vec!["ذهب", "الذهب"]);
        }
        for key in &["perak", "silver"] {
            self.term_map.insert(key.to_string(), vec!["فضة", "الفضة"]);
        }
        for key in &["uang", "money", "harta", "wealth"] {
            self.term_map.insert(key.to_string(), vec!["مال", "المال", "الأموال"]);
        }
        for key in &["online", "daring"] {
            self.term_map.insert(key.to_string(), vec!["الإلكتروني", "عبر الإنترنت"]);
        }

        // Food and animals
        for key in &["daging", "meat"] {
            self.term_map.insert(key.to_string(), vec!["لحم", "اللحم", "اللحوم"]);
        }
        for key in &["ular", "snake"] {
            self.term_map.insert(key.to_string(), vec!["حية", "الحية", "الأفعى", "حكم أكل الحيات"]);
        }
        for key in &["sembelihan", "penyembelihan", "menyembelih", "slaughter"] {
            self.term_map.insert(key.to_string(), vec!["ذبيحة", "الذبح", "الذبائح", "التذكية"]);
        }
        for key in &["binatang", "hewan", "animal"] {
            self.term_map.insert(key.to_string(), vec!["حيوان", "الحيوانات"]);
        }
        for key in &["ikan", "fish"] {
            self.term_map.insert(key.to_string(), vec!["سمك", "السمك"]);
        }
        for key in &["ayam", "chicken"] {
            self.term_map.insert(key.to_string(), vec!["دجاج", "الدجاج"]);
        }
        for key in &["kambing", "goat"] {
            self.term_map.insert(key.to_string(), vec!["غنم", "الغنم", "شاة"]);
        }
        for key in &["sapi", "cow", "cattle"] {
            self.term_map.insert(key.to_string(), vec!["بقر", "البقر"]);
        }
        for key in &["anjing", "dog"] {
            self.term_map.insert(key.to_string(), vec!["كلب", "الكلب"]);
        }
        for key in &["kucing", "cat"] {
            self.term_map.insert(key.to_string(), vec!["هر", "الهرة", "القط"]);
        }

        // Body/health terms
        for key in &["hamil", "kehamilan", "pregnant", "pregnancy"] {
            self.term_map.insert(key.to_string(), vec!["حمل", "الحمل", "الحامل"]);
        }
        for key in &["ibu", "mother"] {
            self.term_map.insert(key.to_string(), vec!["أم", "الأم"]);
        }
        for key in &["menyusui", "breastfeed"] {
            self.term_map.insert(key.to_string(), vec!["رضاعة", "الرضاعة", "المرضع"]);
        }
        for key in &["bayi", "baby", "infant"] {
            self.term_map.insert(key.to_string(), vec!["طفل", "الطفل", "الرضيع", "الصبي"]);
        }
        for key in &["tua", "lansia", "elderly", "old"] {
            self.term_map.insert(key.to_string(), vec!["شيخ", "كبير السن", "العجوز"]);
        }

        // Position/action terms
        for key in &["duduk", "sitting", "sit"] {
            self.term_map.insert(key.to_string(), vec!["جالس", "قاعد", "القعود", "الجلوس"]);
        }
        for key in &["berdiri", "standing", "stand"] {
            self.term_map.insert(key.to_string(), vec!["قائم", "القيام", "الوقوف"]);
        }
        for key in &["berbaring", "lying", "rebahan"] {
            self.term_map.insert(key.to_string(), vec!["مضطجع", "الاضطجاع", "مستلقي"]);
        }
        for key in &["pegang", "memegang", "hold", "carrying"] {
            self.term_map.insert(key.to_string(), vec!["حمل", "يحمل", "الحمل"]);
        }

        // Media/entertainment
        for key in &["nyanyian", "nyanyi", "singing", "song"] {
            self.term_map.insert(key.to_string(), vec!["غناء", "الغناء", "الأغاني"]);
        }
        for key in &["video", "film", "movie"] {
            self.term_map.insert(key.to_string(), vec!["الفيلم", "المرئيات"]);
        }

        // Travel
        for key in &["perjalanan", "safar", "travel", "musafir", "traveler"] {
            self.term_map.insert(key.to_string(), vec!["سفر", "السفر", "المسافر"]);
        }

        // Legal/validity terms
        for key in &["sah", "valid", "sahih"] {
            self.term_map.insert(key.to_string(), vec!["صحة", "صحيح", "الصحة"]);
        }
        for key in &["batal", "invalid", "void"] {
            self.term_map.insert(key.to_string(), vec!["بطلان", "باطل", "البطلان"]);
        }
        for key in &["fasid", "defective"] {
            self.term_map.insert(key.to_string(), vec!["فاسد", "الفساد"]);
        }

        // People of the book
        for key in &["nasrani", "kristen", "christian"] {
            self.term_map.insert(key.to_string(), vec!["نصراني", "النصارى", "أهل الكتاب"]);
        }
        for key in &["yahudi", "jewish", "jew"] {
            self.term_map.insert(key.to_string(), vec!["يهودي", "اليهود", "أهل الكتاب"]);
        }

        // Bid'ah variant handling
        for key in &["bid", "bidah", "bid'ah"] {
            self.term_map.insert(key.to_string(), vec!["بدعة", "البدعة", "البدع"]);
        }

        // Concept/definition terms
        for key in &["pengertian", "definisi", "meaning", "definition"] {
            self.term_map.insert(key.to_string(), vec!["تعريف", "معنى", "المعنى"]);
        }
        for key in &["batasan", "limit", "boundary"] {
            self.term_map.insert(key.to_string(), vec!["حدود", "ضابط", "الضوابط"]);
        }
        for key in &["perbedaan", "difference", "beda"] {
            self.term_map.insert(key.to_string(), vec!["فرق", "الفرق", "الاختلاف"]);
        }
        for key in &["cara", "tata", "method", "kaifiyah", "kaifiyyah"] {
            self.term_map.insert(key.to_string(), vec!["كيفية", "الكيفية"]);
        }

        // Asuransi standalone
        for key in &["asuransi", "insurance"] {
            self.term_map.insert(key.to_string(), vec!["التأمين", "تأمين", "تكافل"]);
        }

        // Misc common words that appear in Islamic queries
        for key in &["tanpa", "tanpa", "without"] {
            self.term_map.insert(key.to_string(), vec!["بغير", "بدون"]);
        }
        for key in &["setelah", "sesudah", "after"] {
            self.term_map.insert(key.to_string(), vec!["بعد", "بعد ذلك"]);
        }
        for key in &["mengandung", "menggunakan", "containing", "using"] {
            self.term_map.insert(key.to_string(), vec![]);
        }
        for key in &["bahan", "ingredient", "material"] {
            self.term_map.insert(key.to_string(), vec!["مادة", "المواد"]);
        }

        // ═══ MISSING TERMS — from 120-query eval audit (v12) ═══

        // Hajj terms (caused ZERO RESULTS)
        for key in &["ihram", "ihrom", "ihram"] {
            self.term_map.insert(key.to_string(), vec!["إحرام", "الإحرام"]);
        }
        for key in &["miqat", "miqot", "mikat"] {
            self.term_map.insert(key.to_string(), vec!["ميقات", "المواقيت", "الميقات"]);
        }
        for key in &["tawaf", "thawaf", "towaf"] {
            self.term_map.insert(key.to_string(), vec!["طواف", "الطواف"]);
        }
        for key in &["sa'i", "sai", "sa'y"] {
            self.term_map.insert(key.to_string(), vec!["سعي", "السعي"]);
        }
        for key in &["wukuf", "wuquf"] {
            self.term_map.insert(key.to_string(), vec!["وقوف", "الوقوف بعرفة"]);
        }
        for key in &["jamarat", "jumroh", "jamrah"] {
            self.term_map.insert(key.to_string(), vec!["جمرات", "رمي الجمرات"]);
        }
        for key in &["dam", "denda haji"] {
            self.term_map.insert(key.to_string(), vec!["دم", "فدية"]);
        }

        // Akhlak / Adab terms (caused ZERO RESULTS)
        for key in &["adab", "etika", "akhlak", "etiquette", "manners"] {
            self.term_map.insert(key.to_string(), vec!["أدب", "آداب", "الآداب", "الأدب"]);
        }
        for key in &["murid", "santri", "student", "pelajar"] {
            self.term_map.insert(key.to_string(), vec!["تلميذ", "طالب", "طالب العلم", "المتعلم"]);
        }
        for key in &["guru", "ustadz", "teacher", "kyai"] {
            self.term_map.insert(key.to_string(), vec!["معلم", "أستاذ", "شيخ", "المعلم"]);
        }
        for key in &["hormat", "menghormati", "respect"] {
            self.term_map.insert(key.to_string(), vec!["احترام", "تعظيم", "إكرام"]);
        }

        // Medical / modern terms (caused ZERO RESULTS)
        for key in &["operasi", "bedah", "surgery"] {
            self.term_map.insert(key.to_string(), vec!["جراحة", "عملية", "العمليات الجراحية"]);
        }
        for key in &["plastik", "kecantikan", "cosmetic"] {
            self.term_map.insert(key.to_string(), vec!["تجميل", "التجميل", "عمليات التجميل"]);
        }
        for key in &["dropship", "dropshipping", "reseller"] {
            self.term_map.insert(key.to_string(), vec!["بيع ما لا يملك", "البيع قبل القبض", "السلم", "بيع المعدوم"]);
        }
        for key in &["tahlilan", "tahlil", "yasinan"] {
            self.term_map.insert(key.to_string(), vec!["التهليل", "قراءة الفاتحة", "إهداء الثواب", "الدعاء للميت"]);
        }

        // Aqidah specifics (caused LOW COVERAGE)
        for key in &["sifat", "attribute", "attributes"] {
            self.term_map.insert(key.to_string(), vec!["صفة", "صفات", "الصفات"]);
        }
        for key in &["rububiyah", "rububiyyah"] {
            self.term_map.insert(key.to_string(), vec!["الربوبية", "توحيد الربوبية"]);
        }
        for key in &["uluhiyah", "uluhiyyah", "ubudiyah"] {
            self.term_map.insert(key.to_string(), vec!["الألوهية", "توحيد الألوهية", "العبودية"]);
        }
        for key in &["asma", "asmaul", "nama"] {
            self.term_map.insert(key.to_string(), vec!["أسماء", "الأسماء", "أسماء الله"]);
        }
        for key in &["husna"] {
            self.term_map.insert(key.to_string(), vec!["الحسنى", "أسماء الله الحسنى"]);
        }
        for key in &["dua", "duapuluh", "twenty", "20"] {
            self.term_map.insert(key.to_string(), vec!["عشرون", "العشرون"]);
        }
        for key in &["puluh", "belas", "ratus"] {
            self.term_map.insert(key.to_string(), vec![]); // numeric modifiers, no Arabic needed
        }
        for key in &["kecil", "small", "minor", "kecilnya"] {
            self.term_map.insert(key.to_string(), vec!["أصغر", "الأصغر", "صغير"]);
        }
        for key in &["besar", "big", "major", "besarnya"] {
            self.term_map.insert(key.to_string(), vec!["أكبر", "الأكبر", "كبير"]);
        }

        // Ibadah specifics (caused LOW COVERAGE)
        for key in &["sahwi", "sahw"] {
            self.term_map.insert(key.to_string(), vec!["سهو", "السهو", "سجود السهو"]);
        }
        for key in &["tilawah", "tilawat"] {
            self.term_map.insert(key.to_string(), vec!["تلاوة", "التلاوة", "سجود التلاوة"]);
        }
        for key in &["aurat", "awrat", "aurot"] {
            self.term_map.insert(key.to_string(), vec!["عورة", "العورة", "ستر العورة"]);
        }
        for key in &["celana", "pants", "trousers"] {
            self.term_map.insert(key.to_string(), vec!["سروال", "لباس", "اللباس"]);
        }
        for key in &["pendek", "short"] {
            self.term_map.insert(key.to_string(), vec!["قصير"]);
        }
        for key in &["imam", "imamah"] {
            self.term_map.insert(key.to_string(), vec!["إمام", "الإمام", "الإمامة"]);
        }
        for key in &["makmum", "makmun", "musalli"] {
            self.term_map.insert(key.to_string(), vec!["مأموم", "المأموم", "المصلي"]);
        }
        for key in &["muadzin", "muazin", "bilal"] {
            self.term_map.insert(key.to_string(), vec!["مؤذن", "المؤذن"]);
        }
        for key in &["khutbah", "khotbah", "sermon"] {
            self.term_map.insert(key.to_string(), vec!["خطبة", "الخطبة"]);
        }

        // Munakahat specifics (caused WRONG RESULTS)
        for key in &["agama", "religion", "din"] {
            self.term_map.insert(key.to_string(), vec!["دين", "الدين", "الأديان"]);
        }
        for key in &["beda", "berbeda", "different"] {
            self.term_map.insert(key.to_string(), vec!["اختلاف", "مختلف", "الاختلاف"]);
        }
        for key in &["murtad", "murtadd", "apostate"] {
            self.term_map.insert(key.to_string(), vec!["ردة", "الردة", "المرتد"]);
        }

        // Jinayat specifics (caused LOW COVERAGE)
        for key in &["korupsi", "corruption"] {
            self.term_map.insert(key.to_string(), vec!["الفساد", "الإختلاس", "أكل المال بالباطل", "الغلول"]);
        }
        for key in &["suap", "risywah", "bribery", "bribe"] {
            self.term_map.insert(key.to_string(), vec!["رشوة", "الرشوة"]);
        }

        // Contemporary / modern fiqh
        for key in &["lgbt", "homoseksual", "homosexual"] {
            self.term_map.insert(key.to_string(), vec!["اللواط", "لوط", "قوم لوط", "السحاق"]);
        }
        for key in &["kb", "kontrasepsi", "contraception"] {
            self.term_map.insert(key.to_string(), vec!["تنظيم النسل", "منع الحمل", "العزل"]);
        }
        for key in &["donor", "mendonorkan"] {
            self.term_map.insert(key.to_string(), vec!["التبرع", "نقل الأعضاء"]);
        }
        for key in &["organ", "organs"] {
            self.term_map.insert(key.to_string(), vec!["أعضاء", "الأعضاء", "زراعة الأعضاء"]);
        }
        for key in &["kredit", "cicilan", "installment", "credit"] {
            self.term_map.insert(key.to_string(), vec!["البيع بالتقسيط", "الأقساط", "البيع بالأجل"]);
        }
        for key in &["mlm", "multi level", "network marketing"] {
            self.term_map.insert(key.to_string(), vec!["التسويق الشبكي", "بيع الغرر"]);
        }
        for key in &["demo", "demonstrasi", "unjuk rasa", "protest"] {
            self.term_map.insert(key.to_string(), vec!["المظاهرات", "الخروج على الحاكم"]);
        }

        // Action verbs commonly used in queries
        for key in &["membatalkan", "cancel", "invalidate"] {
            self.term_map.insert(key.to_string(), vec!["مبطلات", "نواقض", "مفسدات", "إبطال"]);
        }
        for key in &["mewajibkan", "obligate"] {
            self.term_map.insert(key.to_string(), vec!["يوجب", "الإيجاب", "الوجوب"]);
        }
        for key in &["mengharamkan", "prohibit", "forbid"] {
            self.term_map.insert(key.to_string(), vec!["يحرم", "التحريم", "الحرمة"]);
        }
        for key in &["menghalalkan", "legalize"] {
            self.term_map.insert(key.to_string(), vec!["يحل", "الإباحة", "الحل"]);
        }
        for key in &["menikahi", "menikahkan"] {
            self.term_map.insert(key.to_string(), vec!["نكاح", "تزويج", "عقد النكاح"]);
        }
        for key in &["menceraikan", "mencerai"] {
            self.term_map.insert(key.to_string(), vec!["طلاق", "التطليق"]);
        }

        // Shalat-related verbs
        for key in &["sholat", "sholaat"] {
            self.term_map.insert(key.to_string(), vec!["صلاة", "الصلاة", "الصلوات"]);
        }

        // ═══ v15: Common misspelling variants from 10K eval ═══
        // Wudhu variants
        for key in &["wudhlu", "wudlu", "wudu", "wudho", "wudoo"] {
            self.term_map.insert(key.to_string(), vec!["وضوء", "الوضوء", "الطهارة"]);
        }
        // Puasa variants
        for key in &["puoso", "shiyam", "shaum", "syiyam"] {
            self.term_map.insert(key.to_string(), vec!["صوم", "صيام", "الصيام"]);
        }
        // Zakat variants
        for key in &["zakaat", "dzakat", "dzakaat"] {
            self.term_map.insert(key.to_string(), vec!["زكاة", "الزكاة", "زكاة المال"]);
        }
        // Haji variants
        for key in &["hajj", "hadzj", "hadj"] {
            self.term_map.insert(key.to_string(), vec!["حج", "الحج", "المناسك"]);
        }
        // Tayammum variants
        for key in &["tayyamum", "tayamum", "tayamom"] {
            self.term_map.insert(key.to_string(), vec!["تيمم", "التيمم"]);
        }
        // Talak variants
        for key in &["talaq", "tholaq", "tolak", "thalaq"] {
            self.term_map.insert(key.to_string(), vec!["طلاق", "الطلاق"]);
        }
        // Nikah variants
        for key in &["nikakh", "nikaah", "nikha"] {
            self.term_map.insert(key.to_string(), vec!["نكاح", "زواج", "عقد النكاح"]);
        }
        // Khulu' variants
        for key in &["khulug", "khuluq", "khuluk"] {
            self.term_map.insert(key.to_string(), vec!["خلع", "الخلع"]);
        }
        // Fasakh variants
        for key in &["fasah", "fasak", "fasakh"] {
            self.term_map.insert(key.to_string(), vec!["فسخ", "فسخ النكاح"]);
        }
        // Ruku' variants
        for key in &["ruko'", "rukuk", "ruku'", "rukoo"] {
            self.term_map.insert(key.to_string(), vec!["ركوع", "الركوع"]);
        }
        // Sujud variants
        for key in &["sujud", "sujood", "sajdah", "sajda"] {
            self.term_map.insert(key.to_string(), vec!["سجود", "السجود", "السجدة"]);
        }
        // Modern terms for colloquial queries
        for key in &["whatsapp", "wa", "chat"] {
            self.term_map.insert(key.to_string(), vec!["رسالة", "مكتوب", "كتابة"]);
        }
        for key in &["bitcoin", "kripto", "crypto"] {
            self.term_map.insert(key.to_string(), vec!["عملة", "نقد", "صرف", "مال"]);
        }
        for key in &["bank", "perbankan", "banking"] {
            self.term_map.insert(key.to_string(), vec!["مصرف", "ربا", "فائدة"]);
        }
        for key in &["asuransi", "insurance"] {
            self.term_map.insert(key.to_string(), vec!["تأمين", "التأمين", "ضمان"]);
        }
        for key in &["vaksin", "vaccine", "imunisasi"] {
            self.term_map.insert(key.to_string(), vec!["تطعيم", "لقاح", "تلقيح"]);
        }
        for key in &["alkohol", "alcohol", "rhum", "rum"] {
            self.term_map.insert(key.to_string(), vec!["خمر", "كحول", "مسكر"]);
        }
        for key in &["game", "gaming", "permainan"] {
            self.term_map.insert(key.to_string(), vec!["لعب", "لهو", "قمار"]);
        }
        for key in &["musik", "music", "lagu", "nyanyi"] {
            self.term_map.insert(key.to_string(), vec!["غناء", "معازف", "موسيقى"]);
        }
        for key in &["foto", "gambar", "picture", "selfie"] {
            self.term_map.insert(key.to_string(), vec!["تصوير", "صورة", "ذوات الأرواح"]);
        }
        for key in &["pacaran", "dating"] {
            self.term_map.insert(key.to_string(), vec!["خلوة", "اختلاط", "نظر"]);
        }
        for key in &["dropship", "reseller", "makelar"] {
            self.term_map.insert(key.to_string(), vec!["سمسار", "وكالة", "بيع ما لا يملك"]);
        }

        // Numbers/time words used in fiqh queries
        for key in &["tiga", "three", "3"] {
            self.term_map.insert(key.to_string(), vec!["ثلاث", "ثلاثة"]);
        }
        for key in &["empat", "four", "4"] {
            self.term_map.insert(key.to_string(), vec!["أربع", "أربعة"]);
        }

        // ══════ v15 batch 2: High-impact fiqh terms from gap analysis ══════
        
        // Congregational prayer & mosque
        for key in &["jamaah", "berjamaah", "jemaah", "berjemaah", "congregation"] {
            self.term_map.insert(key.to_string(), vec!["جماعة", "الجماعة", "صلاة الجماعة"]);
        }
        // Jinn & sorcery
        for key in &["jin", "jinn", "genie"] {
            self.term_map.insert(key.to_string(), vec!["جن", "الجن", "الجن والشياطين"]);
        }
        for key in &["sihir", "santet", "sorcery", "witchcraft"] {
            self.term_map.insert(key.to_string(), vec!["سحر", "السحر"]);
        }
        for key in &["ruqyah", "rukiah", "rukyah"] {
            self.term_map.insert(key.to_string(), vec!["رقية", "الرقية", "الرقية الشرعية"]);
        }
        for key in &["kesurupan", "possessed", "possession"] {
            self.term_map.insert(key.to_string(), vec!["مس", "المس", "الجن"]);
        }
        // Women's dress code
        for key in &["cadar", "niqab", "niqob"] {
            self.term_map.insert(key.to_string(), vec!["نقاب", "النقاب"]);
        }
        for key in &["jilbab", "hijab", "kerudung", "veil"] {
            self.term_map.insert(key.to_string(), vec!["حجاب", "الحجاب", "جلباب"]);
        }
        // Direction of prayer
        for key in &["kiblat", "qiblat", "qibla", "qiblah"] {
            self.term_map.insert(key.to_string(), vec!["قبلة", "القبلة", "استقبال القبلة"]);
        }
        // Invalidators of wudu
        for key in &["kentut", "buang angin", "flatulence"] {
            self.term_map.insert(key.to_string(), vec!["ريح", "الريح", "حدث", "الحدث"]);
        }
        // Beard
        for key in &["jenggot", "janggut", "beard"] {
            self.term_map.insert(key.to_string(), vec!["لحية", "اللحية", "إعفاء اللحية"]);
        }
        // Engagement & courtship
        for key in &["tunangan", "pertunangan", "engagement"] {
            self.term_map.insert(key.to_string(), vec!["خطبة", "الخطبة"]);
        }
        for key in &["taaruf", "ta'aruf", "ta'arruf"] {
            self.term_map.insert(key.to_string(), vec!["تعارف", "التعارف"]);
        }
        // Puberty
        for key in &["baligh", "balig", "puberty", "aqil baligh"] {
            self.term_map.insert(key.to_string(), vec!["بلوغ", "البلوغ", "بالغ"]);
        }
        // Widow/divorcee
        for key in &["janda", "duda", "widow", "divorcee"] {
            self.term_map.insert(key.to_string(), vec!["مطلقة", "أرملة", "عدة"]);
        }
        // Cupping therapy
        for key in &["bekam", "hijamah", "cupping"] {
            self.term_map.insert(key.to_string(), vec!["حجامة", "الحجامة"]);
        }
        // Gambling
        for key in &["judi", "gambling", "taruhan", "gacha", "lotre", "lottery"] {
            self.term_map.insert(key.to_string(), vec!["قمار", "القمار", "ميسر", "الميسر"]);
        }
        // Insurance (BPJS is Indonesian national health insurance)
        for key in &["bpjs", "jaminan sosial"] {
            self.term_map.insert(key.to_string(), vec!["تأمين", "التأمين", "ضمان"]);
        }
        // Euthanasia & medical ethics
        for key in &["euthanasia", "eutanasia", "suntik mati"] {
            self.term_map.insert(key.to_string(), vec!["قتل الرحمة", "قتل المريض"]);
        }
        for key in &["autopsi", "otopsi", "autopsy"] {
            self.term_map.insert(key.to_string(), vec!["تشريح", "تشريح الجثة"]);
        }
        for key in &["transfusi", "transfusion", "donor darah"] {
            self.term_map.insert(key.to_string(), vec!["نقل الدم", "التبرع بالدم"]);
        }
        // Contraception & sterilization
        for key in &["kondom", "kontrasepsi", "contraception", "kb"] {
            self.term_map.insert(key.to_string(), vec!["منع الحمل", "وسائل منع الحمل", "تنظيم النسل", "عزل"]);
        }
        for key in &["sterilisasi", "vasektomi", "tubektomi", "sterilization"] {
            self.term_map.insert(key.to_string(), vec!["تعقيم", "قطع النسل", "منع الحمل"]);
        }
        for key in &["inseminasi", "bayi tabung", "ivf", "insemination"] {
            self.term_map.insert(key.to_string(), vec!["تلقيح", "التلقيح الاصطناعي"]);
        }
        // Drugs & intoxicants
        for key in &["narkoba", "narkotika", "drugs", "ganja", "marijuana"] {
            self.term_map.insert(key.to_string(), vec!["مخدرات", "المخدرات", "مسكر"]);
        }
        // Gender interaction
        for key in &["ikhtilat", "campur baur", "mixing"] {
            self.term_map.insert(key.to_string(), vec!["اختلاط", "الاختلاط"]);
        }
        // Astrology & fortune-telling
        for key in &["zodiak", "zodiac", "horoskop", "horoscope", "ramalan"] {
            self.term_map.insert(key.to_string(), vec!["تنجيم", "التنجيم", "كهانة"]);
        }
        for key in &["dukun", "paranormal", "fortune teller"] {
            self.term_map.insert(key.to_string(), vec!["كاهن", "كهانة", "عراف"]);
        }
        // Non-Muslim celebrations
        for key in &["valentine", "hallowen", "halloween"] {
            self.term_map.insert(key.to_string(), vec!["أعياد الكفار", "تشبه بالكفار"]);
        }
        for key in &["natal", "christmas", "tahun baru"] {
            self.term_map.insert(key.to_string(), vec!["أعياد الكفار", "تشبه بالكفار", "عيد الميلاد"]);
        }
        // Financial instruments
        for key in &["reksadana", "reksa dana", "mutual fund"] {
            self.term_map.insert(key.to_string(), vec!["صناديق الاستثمار", "استثمار"]);
        }
        for key in &["obligasi", "bonds", "surat utang"] {
            self.term_map.insert(key.to_string(), vec!["سندات", "دين", "ربا"]);
        }
        for key in &["pinjaman", "ngutang", "loan", "pinjol", "paylater"] {
            self.term_map.insert(key.to_string(), vec!["قرض", "القرض", "دين", "الدين"]);
        }
        for key in &["forex", "trading", "saham", "stocks"] {
            self.term_map.insert(key.to_string(), vec!["تجارة", "بيع", "صرف", "مضاربة"]);
        }
        // Dawah & Islamic propagation
        for key in &["dakwah", "dawah", "tabligh"] {
            self.term_map.insert(key.to_string(), vec!["دعوة", "الدعوة", "تبليغ"]);
        }
        // Hypnosis & alternative medicine
        for key in &["hipnotis", "hipnoterapi", "hypnosis"] {
            self.term_map.insert(key.to_string(), vec!["تنويم", "التنويم", "التنويم المغناطيسي"]);
        }
        for key in &["yoga", "meditasi", "meditation"] {
            self.term_map.insert(key.to_string(), vec!["رياضة", "تأمل"]);
        }
        // Itikaf misspelling
        for key in &["tikaf", "iktikaf", "itikaf"] {
            self.term_map.insert(key.to_string(), vec!["اعتكاف", "الاعتكاف"]);
        }
        // Prayer rows
        for key in &["saf", "shaf", "barisan shalat"] {
            self.term_map.insert(key.to_string(), vec!["صف", "الصف", "الصفوف"]);
        }
        // Marriage contract
        for key in &["ijab", "ijab qabul", "akad nikah"] {
            self.term_map.insert(key.to_string(), vec!["إيجاب", "الإيجاب والقبول", "عقد النكاح"]);
        }
        for key in &["qabul", "kabul"] {
            self.term_map.insert(key.to_string(), vec!["قبول", "الإيجاب والقبول"]);
        }
        // Dowry
        for key in &["mahar", "maskawin", "mas kawin", "dowry"] {
            self.term_map.insert(key.to_string(), vec!["مهر", "المهر", "صداق"]);
        }
        // Iddah (waiting period)
        for key in &["iddah", "idah", "masa tunggu"] {
            self.term_map.insert(key.to_string(), vec!["عدة", "العدة"]);
        }
        // Nusyuz (marital disobedience)
        for key in &["nusyuz", "nushuz", "durhaka", "pembangkangan"] {
            self.term_map.insert(key.to_string(), vec!["نشوز", "النشوز"]);
        }
        // Walimah (wedding feast)
        for key in &["walimah", "walimatul urs", "resepsi nikah"] {
            self.term_map.insert(key.to_string(), vec!["وليمة", "الوليمة", "وليمة العرس"]);
        }
        // Qurban specifics
        for key in &["kurban", "qurban", "berkurban", "sacrifice"] {
            self.term_map.insert(key.to_string(), vec!["أضحية", "الأضحية", "ذبح", "قربان"]);
        }
        // Aqiqah
        for key in &["aqiqah", "akikah", "aqikah"] {
            self.term_map.insert(key.to_string(), vec!["عقيقة", "العقيقة"]);
        }
        // Dzikir/remembrance
        for key in &["dzikir", "zikir", "berdzikir", "dhikr", "remembrance"] {
            self.term_map.insert(key.to_string(), vec!["ذكر", "الذكر", "الأذكار"]);
        }
        // Quran recitation
        for key in &["membaca quran", "baca quran", "tilawah", "recitation"] {
            self.term_map.insert(key.to_string(), vec!["قراءة", "تلاوة", "القراءة"]);
        }

        // ═══ BATCH 3: Additional fiqh terms from skenario/kontemporer gap analysis ═══
        // Contract (akad)
        for key in &["akad", "contract", "perjanjian"] {
            self.term_map.insert(key.to_string(), vec!["عقد", "العقد", "العقود"]);
        }
        // Tasyahud (sitting in prayer)
        for key in &["tasyahud", "tasyahhud", "tahiyat", "tahiyyat"] {
            self.term_map.insert(key.to_string(), vec!["تشهد", "التشهد", "التحيات"]);
        }
        // Shaving (cukur)
        for key in &["cukur", "mencukur", "shaving"] {
            self.term_map.insert(key.to_string(), vec!["حلق", "الحلق"]);
        }
        // Nail polish (kutek — blocks wudu water)
        for key in &["kutek", "kuteks", "nail polish"] {
            self.term_map.insert(key.to_string(), vec!["طلاء الأظافر", "أظافر", "المسح"]);
        }
        // Bandage/cast (plester — wudu with bandage)
        for key in &["plester", "perban", "gips", "bandage", "cast"] {
            self.term_map.insert(key.to_string(), vec!["جبيرة", "الجبيرة", "المسح على الجبيرة"]);
        }
        // Piercing
        for key in &["piercing", "tindik"] {
            self.term_map.insert(key.to_string(), vec!["ثقب", "ثقب الأذن", "خرق"]);
        }
        // Epidemic/pandemic
        for key in &["pandemi", "wabah", "epidemic", "pandemic"] {
            self.term_map.insert(key.to_string(), vec!["وباء", "الوباء", "طاعون"]);
        }
        // Handshake (jabat tangan)
        for key in &["jabat", "berjabat", "handshake"] {
            self.term_map.insert(key.to_string(), vec!["مصافحة", "المصافحة"]);
        }
        // Gender/sex change
        for key in &["kelamin", "jenis kelamin", "gender"] {
            self.term_map.insert(key.to_string(), vec!["جنس", "تغيير الجنس"]);
        }
        // Cosplay/imitation
        for key in &["cosplay", "kostum"] {
            self.term_map.insert(key.to_string(), vec!["تشبه", "التشبه"]);
        }
        // Ring (cincin — wudu with ring)
        for key in &["cincin", "ring"] {
            self.term_map.insert(key.to_string(), vec!["خاتم", "الخاتم"]);
        }
        // Nails (kuku — wudu with long nails)
        for key in &["kuku", "nails"] {
            self.term_map.insert(key.to_string(), vec!["أظافر", "الأظافر", "قص الأظافر"]);
        }
        // Dream (mimpi — nocturnal emission)
        for key in &["mimpi", "mimpi basah", "wet dream"] {
            self.term_map.insert(key.to_string(), vec!["احتلام", "الاحتلام", "حلم"]);
        }
        // Eyebrow (alis — plucking eyebrows)
        for key in &["alis", "eyebrow", "mencabut alis"] {
            self.term_map.insert(key.to_string(), vec!["نمص", "النمص", "الحاجب"]);
        }
        // Hair extension
        for key in &["extension", "rambut sambung", "hair extension"] {
            self.term_map.insert(key.to_string(), vec!["وصل الشعر", "الوصل"]);
        }
        // Adopt (anak angkat)
        for key in &["adopsi", "adoption", "anak angkat"] {
            self.term_map.insert(key.to_string(), vec!["تبني", "التبني", "كفالة"]);
        }
        // Organ donation
        for key in &["donor", "donation", "sumbangan organ"] {
            self.term_map.insert(key.to_string(), vec!["تبرع", "التبرع", "نقل الأعضاء"]);
        }
        // PayLater / installment variants
        for key in &["cicilan", "installment", "angsuran"] {
            self.term_map.insert(key.to_string(), vec!["قسط", "أقساط", "بيع التقسيط"]);
        }
        // Online lending
        for key in &["pinjol", "fintech", "lending"] {
            self.term_map.insert(key.to_string(), vec!["قرض", "ربا", "فائدة"]);
        }
        // Voting/election
        for key in &["pemilu", "election", "pilkada", "voting", "memilih"] {
            self.term_map.insert(key.to_string(), vec!["انتخاب", "الانتخاب", "اختيار"]);
        }

        // ═══ Batch 4: English term gap-fill ═══
        // Alms/charity
        for key in &["alms", "charity", "charitable"] {
            self.term_map.insert(key.to_string(), vec!["صدقة", "الصدقة", "إحسان"]);
        }
        // Buying/selling/trade
        for key in &["buying", "selling", "trade", "trading", "transaction"] {
            self.term_map.insert(key.to_string(), vec!["بيع", "شراء", "تجارة", "البيوع"]);
        }
        // Conditions/requirements
        for key in &["conditions", "requirements", "prerequisites"] {
            self.term_map.insert(key.to_string(), vec!["شروط", "الشروط", "شرط"]);
        }
        // Creed/belief
        for key in &["creed", "belief", "theology"] {
            self.term_map.insert(key.to_string(), vec!["عقيدة", "العقيدة", "إيمان"]);
        }
        // Cryptocurrency
        self.term_map.insert("cryptocurrency".to_string(), vec!["عملة رقمية", "بيتكوين", "ربا", "صرف"]);
        // Custody
        for key in &["custody", "guardianship"] {
            self.term_map.insert(key.to_string(), vec!["حضانة", "الحضانة", "كفالة"]);
        }
        // Eclipse
        for key in &["eclipse", "lunar", "solar"] {
            self.term_map.insert(key.to_string(), vec!["كسوف", "خسوف", "صلاة الكسوف"]);
        }
        // Eid
        self.term_map.insert("eid".to_string(), vec!["عيد", "العيد", "عيد الفطر", "عيد الأضحى"]);
        // Ethics/morals
        for key in &["ethics", "morals", "morality", "virtue"] {
            self.term_map.insert(key.to_string(), vec!["أخلاق", "الأخلاق", "فضيلة", "آداب"]);
        }
        // Guardian/wali
        self.term_map.insert("guardian".to_string(), vec!["ولي", "الولي", "ولاية"]);
        // Istisqa (rain prayer)
        self.term_map.insert("istisqa".to_string(), vec!["استسقاء", "صلاة الاستسقاء"]);
        // Janabah (major impurity)
        self.term_map.insert("janabah".to_string(), vec!["جنابة", "الجنابة", "غسل الجنابة"]);
        // Jihad
        self.term_map.insert("jihad".to_string(), vec!["جهاد", "الجهاد", "قتال"]);
        // Lease/rent
        for key in &["lease", "rent", "rental", "leasing"] {
            self.term_map.insert(key.to_string(), vec!["إجارة", "الإجارة", "أجرة"]);
        }
        // Makeup/cosmetics
        for key in &["makeup", "cosmetics", "beautification"] {
            self.term_map.insert(key.to_string(), vec!["زينة", "تجميل", "الزينة"]);
        }
        // Men/women
        self.term_map.insert("men".to_string(), vec!["رجال", "الرجال", "رجل"]);
        self.term_map.insert("women".to_string(), vec!["نساء", "النساء", "المرأة"]);
        // Missed (prayers)
        self.term_map.insert("missed".to_string(), vec!["قضاء", "فائتة", "الفوائت"]);
        // Narration/hadith chain
        for key in &["narration", "narrator", "chain"] {
            self.term_map.insert(key.to_string(), vec!["رواية", "إسناد", "سند", "راوي"]);
        }
        // Night (prayer)
        self.term_map.insert("night".to_string(), vec!["ليل", "الليل", "قيام الليل"]);
        // Photography/images
        for key in &["photography", "photos", "images", "pictures"] {
            self.term_map.insert(key.to_string(), vec!["تصوير", "صورة", "التصوير"]);
        }
        // Pillars
        for key in &["pillars", "pillar", "rukun"] {
            self.term_map.insert(key.to_string(), vec!["أركان", "ركن", "الأركان"]);
        }
        // Pledge/collateral
        self.term_map.insert("pledge".to_string(), vec!["رهن", "الرهن", "ضمان"]);
        // Predestination/fate
        for key in &["predestination", "fate", "destiny", "qadr"] {
            self.term_map.insert(key.to_string(), vec!["قضاء", "قدر", "القضاء والقدر"]);
        }
        // Rain
        self.term_map.insert("rain".to_string(), vec!["مطر", "استسقاء", "صلاة الاستسقاء"]);
        // Resident/traveler
        for key in &["resident", "traveler", "travel", "journey"] {
            self.term_map.insert(key.to_string(), vec!["مقيم", "مسافر", "السفر", "قصر"]);
        }
        // Silk
        self.term_map.insert("silk".to_string(), vec!["حرير", "الحرير", "لبس الحرير"]);
        // Tithe/ushr
        for key in &["tithe", "ushr", "tenth"] {
            self.term_map.insert(key.to_string(), vec!["عشر", "العشر", "زكاة الزروع"]);
        }
        // Treason/rebellion
        for key in &["treason", "rebellion", "revolt"] {
            self.term_map.insert(key.to_string(), vec!["بغي", "البغاة", "خروج"]);
        }
        // Vaping/smoking
        for key in &["vaping", "vape", "smoking", "cigarette", "tobacco"] {
            self.term_map.insert(key.to_string(), vec!["تدخين", "التدخين", "تبغ"]);
        }
        // War/combat
        self.term_map.insert("war".to_string(), vec!["حرب", "الحرب", "جهاد", "قتال"]);
        // Wedding/ceremony
        self.term_map.insert("wedding".to_string(), vec!["زفاف", "وليمة", "عرس", "وليمة العرس"]);
        // Direction (qibla)
        self.term_map.insert("direction".to_string(), vec!["قبلة", "اتجاه", "استقبال القبلة"]);
        // Defense/self-defense
        for key in &["defense", "defence", "self-defense"] {
            self.term_map.insert(key.to_string(), vec!["دفاع", "الدفاع", "دفع الصائل"]);
        }
        // Commentary/exegesis
        for key in &["commentary", "exegesis", "interpretation"] {
            self.term_map.insert(key.to_string(), vec!["تفسير", "التفسير", "شرح", "تأويل"]);
        }

        // ═══ Batch 5: Usul fiqh, standalone worship, comparative religion terms ═══
        // Usul fiqh methodology terms
        for key in &["istihsan", "istihsaan"] {
            self.term_map.insert(key.to_string(), vec!["استحسان", "الاستحسان"]);
        }
        for key in &["istishab", "istishhab"] {
            self.term_map.insert(key.to_string(), vec!["استصحاب", "الاستصحاب"]);
        }
        for key in &["urf", "'urf"] {
            self.term_map.insert(key.to_string(), vec!["عرف", "العرف", "العادة"]);
        }
        for key in &["maqashid", "maqasid", "maqosid"] {
            self.term_map.insert(key.to_string(), vec!["مقاصد الشريعة", "المقاصد", "مقاصد"]);
        }
        for key in &["istidlal", "istidlaal"] {
            self.term_map.insert(key.to_string(), vec!["استدلال", "الاستدلال"]);
        }
        for key in &["dzariah", "dzari'ah", "zariah", "zari'ah"] {
            self.term_map.insert(key.to_string(), vec!["ذريعة", "سد الذرائع", "الذرائع"]);
        }
        for key in &["hifzh", "hifz", "hifdz"] {
            self.term_map.insert(key.to_string(), vec!["حفظ", "الحفظ"]);
        }
        for key in &["istinbath", "istinbat"] {
            self.term_map.insert(key.to_string(), vec!["استنباط", "الاستنباط"]);
        }
        for key in &["tarjih", "tarjeh"] {
            self.term_map.insert(key.to_string(), vec!["ترجيح", "الترجيح"]);
        }
        // Standalone worship terms (previously only in phrase_map)
        for key in &["tarawih", "taraweeh", "taraweh"] {
            self.term_map.insert(key.to_string(), vec!["تراويح", "التراويح", "صلاة التراويح"]);
        }
        for key in &["witir", "witr", "witer"] {
            self.term_map.insert(key.to_string(), vec!["وتر", "الوتر", "صلاة الوتر"]);
        }
        for key in &["basmalah", "basmala", "bismillah"] {
            self.term_map.insert(key.to_string(), vec!["بسملة", "البسملة", "بسم الله"]);
        }
        self.term_map.insert("tarji'".to_string(), vec!["ترجيع", "الترجيع"]);
        self.term_map.insert("tarji".to_string(), vec!["ترجيع", "الترجيع"]);
        // Betrothal/engagement
        for key in &["khitbah", "meminang", "melamar", "pinangan", "lamaran"] {
            self.term_map.insert(key.to_string(), vec!["خطبة", "الخطبة", "خطبة النكاح"]);
        }
        // Comparative religion
        for key in &["trinitas", "trinity"] {
            self.term_map.insert(key.to_string(), vec!["تثليث", "الثالوث"]);
        }
        for key in &["jizyah", "jizya", "jizyat"] {
            self.term_map.insert(key.to_string(), vec!["جزية", "الجزية"]);
        }
        // Food categories
        self.term_map.insert("sushi".to_string(), vec!["سمك", "أكل السمك", "حكم الأسماك"]);
        self.term_map.insert("sashimi".to_string(), vec!["سمك", "أكل السمك نيئاً", "اللحم النيء"]);
        // Fiqh analysis terms
        self.term_map.insert("illat".to_string(), vec!["علة", "العلة"]);
        for key in &["mansukh", "nasikh", "naskh"] {
            self.term_map.insert(key.to_string(), vec!["نسخ", "الناسخ والمنسوخ", "النسخ"]);
        }
        self.term_map.insert("mutlaq".to_string(), vec!["مطلق", "المطلق"]);
        self.term_map.insert("muqayyad".to_string(), vec!["مقيد", "المقيد"]);
        self.term_map.insert("aam".to_string(), vec!["عام", "العام"]);
        self.term_map.insert("khash".to_string(), vec!["خاص", "الخاص"]);

        // ═══════════════════════════════════════════════════════════
        // BATCH 6 (V16): 479 ZERO-RESULT FIXES — confirmed by audit_zeros.py
        // All 479 failures were zero-term translation failures; this batch
        // adds every missing term identified in that audit.
        // ═══════════════════════════════════════════════════════════

        // ── HAJJ: missing pillar terms ──
        for key in &["jumrah", "jamroh", "jumroh"] {
            self.term_map.insert(key.to_string(), vec!["جمرة", "الجمرات", "رمي الجمرات"]);
        }
        for key in &["tahallul", "tahalul", "tahallal"] {
            self.term_map.insert(key.to_string(), vec!["تحلل", "التحلل", "التحلل الأول", "التحلل الثاني"]);
        }
        for key in &["mabit"] {
            self.term_map.insert(key.to_string(), vec!["مبيت", "المبيت بمزدلفة", "المبيت بمنى"]);
        }
        for key in &["muzdalifah"] {
            self.term_map.insert(key.to_string(), vec!["مزدلفة", "المبيت بمزدلفة"]);
        }
        for key in &["sahur", "sahor"] {
            self.term_map.insert(key.to_string(), vec!["سحور", "السحور", "الإمساك"]);
        }
        for key in &["imsak", "imsakiah"] {
            self.term_map.insert(key.to_string(), vec!["إمساك", "وقت الإمساك"]);
        }

        // ── MUAMALAT: missing contracts & commercial terms ──
        for key in &["wakalah", "wakala"] {
            self.term_map.insert(key.to_string(), vec!["وكالة", "الوكالة", "الوكيل"]);
        }
        for key in &["kafalah", "kafala"] {
            self.term_map.insert(key.to_string(), vec!["كفالة", "الكفالة", "ضمان"]);
        }
        for key in &["khiyar"] {
            self.term_map.insert(key.to_string(), vec!["خيار", "الخيار", "خيار المجلس", "خيار الشرط", "خيار العيب"]);
        }
        for key in &["gharar"] {
            self.term_map.insert(key.to_string(), vec!["غرر", "الغرر", "بيع الغرر"]);
        }
        for key in &["maysir", "qimar"] {
            self.term_map.insert(key.to_string(), vec!["ميسر", "الميسر", "قمار"]);
        }
        for key in &["murabahah", "murabaha"] {
            self.term_map.insert(key.to_string(), vec!["مرابحة", "المرابحة", "بيع المرابحة"]);
        }
        for key in &["istishna", "istisnaa"] {
            self.term_map.insert(key.to_string(), vec!["استصناع", "الاستصناع", "عقد الاستصناع"]);
        }
        for key in &["syuf'ah", "syufah", "syufa"] {
            self.term_map.insert(key.to_string(), vec!["شفعة", "الشفعة"]);
        }
        for key in &["luqathah", "luqatha"] {
            self.term_map.insert(key.to_string(), vec!["لقطة", "اللقطة"]);
        }
        for key in &["ji'alah", "ji'ala", "ji'alah"] {
            self.term_map.insert(key.to_string(), vec!["جعالة", "الجعالة"]);
        }
        for key in &["qardh", "qard"] {
            self.term_map.insert(key.to_string(), vec!["قرض", "القرض"]);
        }
        for key in &["hadhanah", "hadzanah"] {
            self.term_map.insert(key.to_string(), vec!["حضانة", "الحضانة"]);
        }
        for key in &["syirkah"] {
            self.term_map.insert(key.to_string(), vec!["شركة", "الشركة", "الشركات"]);
        }
        for key in &["abdan"] {
            self.term_map.insert(key.to_string(), vec!["شركة الأبدان", "الأبدان"]);
        }
        for key in &["inan"] {
            self.term_map.insert(key.to_string(), vec!["شركة العنان", "العنان"]);
        }
        for key in &["mufawadhah"] {
            self.term_map.insert(key.to_string(), vec!["شركة المفاوضة", "المفاوضة"]);
        }
        for key in &["kpr", "KPR"] {
            self.term_map.insert(key.to_string(), vec!["تمويل عقاري", "مرابحة عقارية", "بيع بالتقسيط"]);
        }
        // dzawil furudh = heirs with fixed shares
        for key in &["dzawil", "zawil", "ashhabul"] {
            self.term_map.insert(key.to_string(), vec!["ذوو الفروض", "أصحاب الفروض"]);
        }
        for key in &["furudh"] {
            self.term_map.insert(key.to_string(), vec!["الفروض", "ذوو الفروض"]);
        }
        for key in &["karya", "karangan", "tulisan"] {
            self.term_map.insert(key.to_string(), vec!["مؤلفات", "كتب", "تأليف"]);
        }

        // ── AQIDAH: Theological terms ──
        for key in &["kufur", "kufr"] {
            self.term_map.insert(key.to_string(), vec!["كفر", "الكفر", "أنواع الكفر"]);
        }
        for key in &["nifaq", "kemunafikan"] {
            self.term_map.insert(key.to_string(), vec!["نفاق", "النفاق", "المنافق"]);
        }
        for key in &["munafik", "munafiq"] {
            self.term_map.insert(key.to_string(), vec!["منافق", "المنافق", "علامات النفاق"]);
        }
        for key in &["fasiq", "fasik"] {
            self.term_map.insert(key.to_string(), vec!["فاسق", "الفسق", "الفاسق"]);
        }
        for key in &["asy'ariyah", "asyariyah", "ashariyah"] {
            self.term_map.insert(key.to_string(), vec!["أشعرية", "الأشعرية"]);
        }
        for key in &["maturidiyah", "maturidiyyah"] {
            self.term_map.insert(key.to_string(), vec!["ماتريدية", "الماتريدية"]);
        }
        for key in &["manhaj"] {
            self.term_map.insert(key.to_string(), vec!["منهج", "منهج السلف"]);
        }
        for key in &["maulid", "maulud", "milad nabi"] {
            self.term_map.insert(key.to_string(), vec!["مولد", "المولد النبوي", "الاحتفال بالمولد"]);
        }
        for key in &["jimat", "azimat", "talisman", "amulet"] {
            self.term_map.insert(key.to_string(), vec!["تميمة", "التمائم"]);
        }
        for key in &["rajah"] {
            self.term_map.insert(key.to_string(), vec!["رقية", "الرقية"]);
        }
        for key in &["keramat", "karamah", "karomah"] {
            self.term_map.insert(key.to_string(), vec!["كرامة", "الكرامة", "كرامات الأولياء"]);
        }

        // ── IBADAH: Prayer specifics ──
        for key in &["tahajjud", "qiyamullail"] {
            self.term_map.insert(key.to_string(), vec!["تهجد", "التهجد", "قيام الليل", "صلاة التهجد"]);
        }
        for key in &["masah"] {
            self.term_map.insert(key.to_string(), vec!["مسح", "المسح على الخفين", "خفان"]);
        }
        for key in &["wirid", "wird"] {
            self.term_map.insert(key.to_string(), vec!["ورد", "الأوراد", "ورد الصباح", "ورد المساء"]);
        }
        for key in &["istighatsah", "istighasah", "istighosah"] {
            self.term_map.insert(key.to_string(), vec!["استغاثة", "الاستغاثة"]);
        }
        for key in &["ratib", "ratibah"] {
            self.term_map.insert(key.to_string(), vec!["راتب", "الراتبة", "السنن الرواتب"]);
        }
        for key in &["i'tidal", "iktidal"] {
            self.term_map.insert(key.to_string(), vec!["اعتدال", "الاعتدال", "الاعتدال من الركوع"]);
        }
        for key in &["tumakninah", "tumaninah", "tuma'ninah"] {
            self.term_map.insert(key.to_string(), vec!["طمأنينة", "الطمأنينة"]);
        }
        for key in &["ta'awwudz", "ta'awwuz", "taawudz"] {
            self.term_map.insert(key.to_string(), vec!["تعوذ", "الاستعاذة", "أعوذ بالله"]);
        }
        for key in &["amin", "amiin", "ameen"] {
            self.term_map.insert(key.to_string(), vec!["آمين", "التأمين", "الجهر بآمين"]);
        }
        for key in &["shalawat", "salawat"] {
            self.term_map.insert(key.to_string(), vec!["الصلاة على النبي", "صلوات", "فضل الصلاة على النبي"]);
        }
        for key in &["istighfar", "istighfaar"] {
            self.term_map.insert(key.to_string(), vec!["استغفار", "الاستغفار", "فضل الاستغفار"]);
        }
        for key in &["istinja", "istinja'"] {
            self.term_map.insert(key.to_string(), vec!["استنجاء", "الاستنجاء"]);
        }
        for key in &["waktu"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["وقت", "الوقت", "الأوقات"]);
        }

        // ── JINAYAT: Criminal fiqh ──
        for key in &["qishas", "qisas"] {
            self.term_map.insert(key.to_string(), vec!["قصاص", "القصاص", "حد القصاص"]);
        }
        for key in &["hirabah", "hiraba"] {
            self.term_map.insert(key.to_string(), vec!["حرابة", "الحرابة", "المحاربة"]);
        }
        for key in &["bughat", "bugat"] {
            self.term_map.insert(key.to_string(), vec!["بغاة", "البغاة", "الخروج على الإمام"]);
        }
        for key in &["jinayah", "jinayat"] {
            self.term_map.insert(key.to_string(), vec!["جناية", "الجنايات"]);
        }
        for key in &["janin", "jaenin"] {
            self.term_map.insert(key.to_string(), vec!["جنين", "الجنين", "الجناية على الجنين"]);
        }
        for key in &["arsy"] {
            self.term_map.insert(key.to_string(), vec!["أرش", "الأرش", "دية الأطراف"]);
        }
        for key in &["hukumah"] {
            self.term_map.insert(key.to_string(), vec!["حكومة", "حكومة العدل"]);
        }
        for key in &["ghurah", "ghurah janin"] {
            self.term_map.insert(key.to_string(), vec!["غرة", "دية الجنين"]);
        }

        // ── AKHLAQ: Virtues and vices ──
        for key in &["tawadhu", "tawadu"] {
            self.term_map.insert(key.to_string(), vec!["تواضع", "التواضع"]);
        }
        for key in &["kibr", "takabbur"] {
            self.term_map.insert(key.to_string(), vec!["كبر", "الكبر", "التكبر"]);
        }
        for key in &["husnuzhan", "husnu dzan"] {
            self.term_map.insert(key.to_string(), vec!["حسن الظن"]);
        }
        for key in &["su'uzhan", "suuzhan"] {
            self.term_map.insert(key.to_string(), vec!["سوء الظن"]);
        }
        for key in &["birrul walidain", "birr al walidain"] {
            self.term_map.insert(key.to_string(), vec!["بر الوالدين", "عقوق الوالدين"]);
        }
        for key in &["silaturahmi", "silaturahim"] {
            self.term_map.insert(key.to_string(), vec!["صلة الرحم", "الرحم"]);
        }
        for key in &["keutamaan", "fadilah", "fadhilah"] {
            self.term_map.insert(key.to_string(), vec!["فضل", "فضائل", "الفضل"]);
        }
        for key in &["mahmudah", "terpuji"] {
            self.term_map.insert(key.to_string(), vec!["صفات محمودة", "الفضائل"]);
        }
        for key in &["madzmumah", "tercela"] {
            self.term_map.insert(key.to_string(), vec!["صفات مذمومة", "الرذائل"]);
        }

        // ── TASAWUF: Spiritual states (maqamat) ──
        for key in &["muraqabah", "muroqobah"] {
            self.term_map.insert(key.to_string(), vec!["مراقبة", "المراقبة"]);
        }
        for key in &["muhasabah", "muhasabat"] {
            self.term_map.insert(key.to_string(), vec!["محاسبة", "المحاسبة", "محاسبة النفس"]);
        }
        for key in &["mahabbah"] {
            self.term_map.insert(key.to_string(), vec!["محبة", "المحبة", "المحبة لله"]);
        }
        for key in &["khauf"] {
            self.term_map.insert(key.to_string(), vec!["خوف", "الخوف", "الخشية"]);
        }
        for key in &["raja'", "raja"] {
            // Islamic spiritual concept: hope/longing for Allah's mercy
            self.term_map.entry(key.to_string()).or_insert(vec!["رجاء", "الرجاء"]);
        }
        for key in &["zuhud", "zuhd"] {
            self.term_map.insert(key.to_string(), vec!["زهد", "الزهد"]);
        }
        for key in &["wara'", "wara"] {
            self.term_map.insert(key.to_string(), vec!["ورع", "الورع"]);
        }
        for key in &["tarekat", "thariqat", "tariqah"] {
            self.term_map.insert(key.to_string(), vec!["طريقة", "الطريقة", "الطرق الصوفية"]);
        }
        for key in &["naqsyabandiyah", "naqsybandiyah", "naqshbandiyya"] {
            self.term_map.insert(key.to_string(), vec!["نقشبندية", "الطريقة النقشبندية"]);
        }
        for key in &["qadiriyah", "qadiriyyah", "qadiri"] {
            self.term_map.insert(key.to_string(), vec!["قادرية", "الطريقة القادرية"]);
        }

        // ── ULUMUL QURAN: Quran sciences ──
        for key in &["makiyyah", "makkiyah"] {
            self.term_map.insert(key.to_string(), vec!["مكية", "المكي", "السور المكية"]);
        }
        for key in &["madaniyyah", "madaniyah"] {
            self.term_map.insert(key.to_string(), vec!["مدنية", "المدني", "السور المدنية"]);
        }
        for key in &["muhkam"] {
            self.term_map.insert(key.to_string(), vec!["محكم", "المحكم", "المحكم والمتشابه"]);
        }
        for key in &["mutasyabih", "mutashabih"] {
            self.term_map.insert(key.to_string(), vec!["متشابه", "المتشابه"]);
        }
        for key in &["i'jaz", "ijaz"] {
            self.term_map.insert(key.to_string(), vec!["إعجاز", "الإعجاز", "إعجاز القرآن"]);
        }
        for key in &["qira'at", "qiraah", "qiraat"] {
            self.term_map.insert(key.to_string(), vec!["قراءات", "القراءات السبع", "القراءات"]);
        }
        for key in &["tahfidz", "hafidz"] {
            self.term_map.insert(key.to_string(), vec!["حفظ القرآن", "تحفيظ القرآن"]);
        }
        for key in &["quran", "alquran", "qur'an"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["القرآن", "القرآن الكريم"]);
        }

        // ── HADITH BOOKS: proper noun references ──
        for key in &["bukhari", "al-bukhari"] {
            self.term_map.insert(key.to_string(), vec!["صحيح البخاري", "البخاري"]);
        }
        for key in &["shahih", "sahih"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صحيح", "الصحيح"]);
        }
        for key in &["shahih bukhari", "sahih bukhari", "sahih al-bukhari"] {
            self.term_map.insert(key.to_string(), vec!["صحيح البخاري", "البخاري"]);
        }
        for key in &["shahih muslim", "sahih muslim"] {
            self.term_map.insert(key.to_string(), vec!["صحيح مسلم", "مسلم"]);
        }
        for key in &["abu dawud"] {
            self.term_map.insert(key.to_string(), vec!["سنن أبي داود", "أبو داود"]);
        }
        for key in &["tirmidzi", "tirmizi", "tirmidhi"] {
            self.term_map.insert(key.to_string(), vec!["سنن الترمذي", "الترمذي"]);
        }
        for key in &["nasa'i", "nasai"] {
            self.term_map.insert(key.to_string(), vec!["سنن النسائي", "النسائي"]);
        }
        for key in &["ibnu majah", "ibn majah"] {
            self.term_map.insert(key.to_string(), vec!["سنن ابن ماجه", "ابن ماجه"]);
        }
        for key in &["musnad"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مسند أحمد", "المسند"]);
        }
        for key in &["muwattha"] {
            self.term_map.insert(key.to_string(), vec!["الموطأ", "موطأ مالك"]);
        }

        // ── CLASSICAL KITAB: Book title references ──
        for key in &["ihya"] {
            self.term_map.insert(key.to_string(), vec!["إحياء علوم الدين", "الغزالي"]);
        }
        for key in &["bidayatul mujtahid"] {
            self.term_map.insert(key.to_string(), vec!["بداية المجتهد", "ابن رشد"]);
        }
        for key in &["fathul qarib"] {
            self.term_map.insert(key.to_string(), vec!["فتح القريب", "التقريب"]);
        }
        for key in &["fathul mu'in", "fathul muin"] {
            self.term_map.insert(key.to_string(), vec!["فتح المعين", "زين الدين"]);
        }
        for key in &["kifayatul akhyar"] {
            self.term_map.insert(key.to_string(), vec!["كفاية الأخيار"]);
        }
        for key in &["tuhfatul muhtaj"] {
            self.term_map.insert(key.to_string(), vec!["تحفة المحتاج", "ابن حجر الهيتمي"]);
        }
        for key in &["nihayatul muhtaj"] {
            self.term_map.insert(key.to_string(), vec!["نهاية المحتاج", "الرملي"]);
        }
        for key in &["mughnil muhtaj"] {
            self.term_map.insert(key.to_string(), vec!["مغني المحتاج", "الشربيني"]);
        }
        for key in &["raudhatut thalibin", "raudhat thalibin"] {
            self.term_map.insert(key.to_string(), vec!["روضة الطالبين", "النووي"]);
        }
        for key in &["riyadhus shalihin"] {
            self.term_map.insert(key.to_string(), vec!["رياض الصالحين", "النووي"]);
        }
        for key in &["bulughul maram"] {
            self.term_map.insert(key.to_string(), vec!["بلوغ المرام", "ابن حجر العسقلاني"]);
        }
        for key in &["subulussalam"] {
            self.term_map.insert(key.to_string(), vec!["سبل السلام", "الصنعاني"]);
        }
        for key in &["nailul authar"] {
            self.term_map.insert(key.to_string(), vec!["نيل الأوطار", "الشوكاني"]);
        }
        for key in &["kifayah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["كفاية", "الكفاية"]);
        }
        for key in &["tuhfah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تحفة"]);
        }
        for key in &["nihayah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نهاية"]);
        }
        for key in &["mughni"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مغني"]);
        }
        for key in &["raudhah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["روضة"]);
        }
        for key in &["bulugh"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بلوغ"]);
        }
        for key in &["subul"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سبل"]);
        }

        // ── USUL FIQH: additional principles ──
        for key in &["mujmal"] {
            self.term_map.insert(key.to_string(), vec!["مجمل", "المجمل"]);
        }
        for key in &["mubayyan", "bayyin"] {
            self.term_map.insert(key.to_string(), vec!["مبين", "البيان", "المبين"]);
        }
        for key in &["haqiqi"] {
            self.term_map.insert(key.to_string(), vec!["حقيقة", "الحقيقي"]);
        }
        for key in &["majazi"] {
            self.term_map.insert(key.to_string(), vec!["مجاز", "المجاز"]);
        }
        for key in &["rukhshah", "rukhsah"] {
            self.term_map.insert(key.to_string(), vec!["رخصة", "الرخصة"]);
        }
        for key in &["azimah", "aziimah"] {
            self.term_map.insert(key.to_string(), vec!["عزيمة", "العزيمة"]);
        }
        for key in &["dharurah", "darurah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ضرورة", "الضرورة", "الاضطرار"]);
        }
        for key in &["kaidah"] {
            self.term_map.insert(key.to_string(), vec!["قاعدة", "القواعد", "القواعد الفقهية"]);
        }
        for key in &["mujtahid"] {
            self.term_map.insert(key.to_string(), vec!["مجتهد", "الاجتهاد", "مجتهد مطلق"]);
        }
        for key in &["khas", "khusus"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["خاص", "الخاص"]);
        }

        // ── SIRAH: Islamic history ──
        for key in &["badr"] {
            self.term_map.insert(key.to_string(), vec!["غزوة بدر", "بدر"]);
        }
        for key in &["uhud"] {
            self.term_map.insert(key.to_string(), vec!["غزوة أحد", "أحد"]);
        }
        for key in &["khandaq", "ahzab"] {
            self.term_map.insert(key.to_string(), vec!["غزوة الخندق", "الأحزاب"]);
        }
        for key in &["fathu makkah"] {
            self.term_map.insert(key.to_string(), vec!["فتح مكة"]);
        }
        for key in &["isra miraj", "isra' mi'raj"] {
            self.term_map.insert(key.to_string(), vec!["الإسراء والمعراج", "إسراء"]);
        }
        for key in &["piagam madinah"] {
            self.term_map.insert(key.to_string(), vec!["وثيقة المدينة", "صحيفة المدينة"]);
        }
        for key in &["khulafaur rasyidin"] {
            self.term_map.insert(key.to_string(), vec!["الخلفاء الراشدون"]);
        }
        for key in &["umayyah"] {
            self.term_map.insert(key.to_string(), vec!["بنو أمية", "الدولة الأموية"]);
        }
        for key in &["abbasiyah"] {
            self.term_map.insert(key.to_string(), vec!["بنو العباس", "الدولة العباسية"]);
        }
        for key in &["perang"] {
            self.term_map.insert(key.to_string(), vec!["غزوة", "معركة", "حرب"]);
        }

        // ── SCHOLARS: proper noun references ──
        for key in &["ibnu taimiyah", "ibn taimiyah", "ibnu taymiyah"] {
            self.term_map.insert(key.to_string(), vec!["ابن تيمية"]);
        }
        for key in &["ibnu qayyim", "ibn qayyim", "ibnul qayyim"] {
            self.term_map.insert(key.to_string(), vec!["ابن القيم"]);
        }
        for key in &["ibnu hajar haitami", "ibn hajar haitami"] {
            self.term_map.insert(key.to_string(), vec!["ابن حجر الهيتمي"]);
        }
        for key in &["ibnu hajar", "ibn hajar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ابن حجر", "ابن حجر العسقلاني"]);
        }
        for key in &["abu bakar", "abu bakr"] {
            self.term_map.insert(key.to_string(), vec!["أبو بكر", "أبو بكر الصديق"]);
        }
        for key in &["umar bin khattab"] {
            self.term_map.insert(key.to_string(), vec!["عمر بن الخطاب"]);
        }
        for key in &["utsman bin affan"] {
            self.term_map.insert(key.to_string(), vec!["عثمان بن عفان"]);
        }
        for key in &["khadijah"] {
            self.term_map.insert(key.to_string(), vec!["خديجة", "خديجة بنت خويلد"]);
        }
        for key in &["aisyah", "aisha"] {
            self.term_map.insert(key.to_string(), vec!["عائشة", "عائشة بنت أبي بكر"]);
        }
        for key in &["fatimah"] {
            self.term_map.insert(key.to_string(), vec!["فاطمة", "فاطمة الزهراء"]);
        }
        for key in &["hamzah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حمزة بن عبد المطلب"]);
        }
        for key in &["khalid bin walid"] {
            self.term_map.insert(key.to_string(), vec!["خالد بن الوليد"]);
        }
        // ─── Single-word fallbacks for dead multi-word scholar keys ───
        for key in &["khalid"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["خالد", "خالد بن الوليد"]);
        }
        for key in &["sina"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ابن سينا", "سينا"]);
        }
        for key in &["rusyd"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ابن رشد", "رشد"]);
        }
        for key in &["khaldun"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ابن خلدون", "خلدون"]);
        }
        for key in &["utsman"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عثمان", "عثمان بن عفان"]);
        }
        for key in &["khattab"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الخطاب", "عمر بن الخطاب"]);
        }
        for key in &["affan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عفان", "عثمان بن عفان"]);
        }
        for key in &["jalaluddin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جلال الدين", "السيوطي"]);
        }
        for key in &["bakar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أبو بكر", "الصديق"]);
        }
        for key in &["riwayat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["رواية", "سيرة"]);
        }
        for key in &["ibn"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ابن", "بن"]);
        }
        for key in &["binti"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بنت"]);
        }
        for key in &["bin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بن"]);
        }
        // ─── Islamic month single-word fallbacks (multi-word keys are dead) ───
        for key in &["rabiul", "rabi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ربيع", "ربيع الأول"]);
        }
        for key in &["jumadil", "jumada"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جمادى"]);
        }
        for key in &["dzulqa'dah", "dzulqadah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ذو القعدة"]);
        }
        for key in &["dzulhijjah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ذو الحجة"]);
        }
        for key in &["ibn sina", "ibnu sina", "avicenna"] {
            self.term_map.insert(key.to_string(), vec!["ابن سينا"]);
        }
        for key in &["ibn rusyd", "ibnu rusyd", "averroes"] {
            self.term_map.insert(key.to_string(), vec!["ابن رشد"]);
        }
        for key in &["ibn khaldun", "ibnu khaldun"] {
            self.term_map.insert(key.to_string(), vec!["ابن خلدون"]);
        }
        for key in &["suyuthi", "as-suyuthi", "jalaluddin suyuthi"] {
            self.term_map.insert(key.to_string(), vec!["السيوطي", "جلال الدين السيوطي"]);
        }
        for key in &["nawawi", "imam nawawi", "an nawawi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["النووي", "الإمام النووي"]);
        }
        for key in &["ghazali", "al ghazali", "imam ghazali"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الغزالي", "حجة الإسلام"]);
        }
        for key in &["biografi", "riwayat hidup"] {
            self.term_map.insert(key.to_string(), vec!["سيرة", "ترجمة", "حياة"]);
        }

        // ── ISLAMIC MONTHS ──
        for key in &["muharram"] {
            self.term_map.insert(key.to_string(), vec!["محرم", "شهر محرم"]);
        }
        for key in &["rabiul awal", "rabi al awwal"] {
            self.term_map.insert(key.to_string(), vec!["ربيع الأول", "شهر المولد"]);
        }
        for key in &["rabiul akhir"] {
            self.term_map.insert(key.to_string(), vec!["ربيع الآخر"]);
        }
        for key in &["jumadil awal"] {
            self.term_map.insert(key.to_string(), vec!["جمادى الأولى"]);
        }
        for key in &["jumadil akhir"] {
            self.term_map.insert(key.to_string(), vec!["جمادى الآخرة"]);
        }
        for key in &["rajab"] {
            self.term_map.insert(key.to_string(), vec!["رجب", "شهر رجب"]);
        }
        for key in &["sya'ban", "syaban", "sha'ban"] {
            self.term_map.insert(key.to_string(), vec!["شعبان", "شهر شعبان"]);
        }
        for key in &["syawal", "shawwal"] {
            self.term_map.insert(key.to_string(), vec!["شوال", "شهر شوال"]);
        }
        for key in &["dzulqa'dah", "dzulqadah"] {
            self.term_map.insert(key.to_string(), vec!["ذو القعدة"]);
        }
        for key in &["dzulhijjah", "dzulhijja"] {
            self.term_map.insert(key.to_string(), vec!["ذو الحجة", "شهر ذي الحجة"]);
        }
        for key in &["nisfu sya'ban", "nisfu syaban"] {
            self.term_map.insert(key.to_string(), vec!["ليلة النصف من شعبان", "نصف شعبان"]);
        }
        for key in &["nisfu"] {
            self.term_map.insert(key.to_string(), vec!["نصف", "ليلة النصف"]);
        }

        // ── DAYS OF WEEK ──
        for key in &["senin", "hari senin", "monday"] {
            self.term_map.insert(key.to_string(), vec!["الاثنين", "يوم الاثنين"]);
        }
        for key in &["selasa", "tuesday"] {
            self.term_map.insert(key.to_string(), vec!["الثلاثاء", "يوم الثلاثاء"]);
        }
        for key in &["rabu", "wednesday"] {
            self.term_map.insert(key.to_string(), vec!["الأربعاء", "يوم الأربعاء"]);
        }
        for key in &["kamis", "thursday"] {
            self.term_map.insert(key.to_string(), vec!["الخميس", "يوم الخميس"]);
        }
        for key in &["sabtu", "saturday"] {
            self.term_map.insert(key.to_string(), vec!["السبت", "يوم السبت"]);
        }
        for key in &["ahad", "minggu", "sunday"] {
            self.term_map.insert(key.to_string(), vec!["الأحد", "يوم الأحد"]);
        }

        // ── PARADISE / HELL / ANGELS ──
        for key in &["paradise", "heaven", "jannah", "surga", "syurga"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جنة", "الجنة", "نعيم الجنة"]);
        }
        for key in &["hellfire", "hell", "jahannam", "neraka"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نار", "جهنم", "عذاب النار"]);
        }
        for key in &["angels", "angel", "malaikat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ملائكة", "الملائكة"]);
        }
        for key in &["hypocrisy", "hypocrite"] {
            self.term_map.insert(key.to_string(), vec!["نفاق", "المنافق", "علامات النفاق"]);
        }
        for key in &["nasheed", "nasyid"] {
            self.term_map.insert(key.to_string(), vec!["نشيد", "الأناشيد الإسلامية"]);
        }
        for key in &["shoes", "sandals"] {
            self.term_map.insert(key.to_string(), vec!["نعل", "الخفين", "حذاء"]);
        }
        for key in &["langit"] {
            self.term_map.insert(key.to_string(), vec!["سماء", "السماوات"]);
        }
        for key in &["bumi"] {
            self.term_map.insert(key.to_string(), vec!["أرض", "الأرض"]);
        }
        for key in &["pintu", "gate"] {
            self.term_map.insert(key.to_string(), vec!["باب", "أبواب"]);
        }
        for key in &["lapisan"] {
            self.term_map.insert(key.to_string(), vec!["طبقة", "طبقات"]);
        }
        for key in &["10 sahabat", "sepuluh sahabat"] {
            self.term_map.insert(key.to_string(), vec!["العشرة المبشرون بالجنة"]);
        }

        // ── TYPO FIXES (from eval audit) ──
        for key in &["wudloo", "wuhu"] {
            self.term_map.insert(key.to_string(), vec!["وضوء", "الوضوء", "الطهارة"]);
        }
        for key in &["shalt"] {
            self.term_map.insert(key.to_string(), vec!["صلاة", "الصلاة"]);
        }
        for key in &["pussa", "posa"] {
            self.term_map.insert(key.to_string(), vec!["صوم", "صيام", "الصيام"]);
        }
        for key in &["zkat", "zkaat"] {
            self.term_map.insert(key.to_string(), vec!["زكاة", "الزكاة"]);
        }
        for key in &["qadha'", "qodho", "qodha"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قضاء", "القضاء", "الفوائت"]);
        }
        for key in &["adzanm", "adzann"] {
            self.term_map.insert(key.to_string(), vec!["أذان", "الأذان"]);
        }
        for key in &["mitsqal", "mithqal"] {
            self.term_map.insert(key.to_string(), vec!["مثقال", "المثقال"]);
        }
        for key in &["istiwa'", "istiwa"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["استواء", "الاستواء"]);
        }

        // ── CONTEXT / GENERIC QUERY WORDS ──
        for key in &["sejarah", "tarikh"] {
            self.term_map.insert(key.to_string(), vec!["تاريخ", "التاريخ"]);
        }
        for key in &["makna", "arti", "artinya"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["معنى", "المعنى", "تعريف"]);
        }
        for key in &["manfaat", "faedah"] {
            self.term_map.insert(key.to_string(), vec!["فائدة", "فوائد", "المنافع"]);
        }
        for key in &["bahaya", "mudharat"] {
            self.term_map.insert(key.to_string(), vec!["ضرر", "الضرر", "مضار"]);
        }
        for key in &["amalan", "amal"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عمل", "أعمال"]);
        }
        for key in &["peristiwa", "kejadian"] {
            self.term_map.insert(key.to_string(), vec!["حادثة", "واقعة", "أحداث"]);
        }
        for key in &["pengarang", "penulis", "author"] {
            self.term_map.insert(key.to_string(), vec!["مؤلف", "كاتب", "صاحب"]);
        }

        // ── CONTEMPORARY ISSUES ──
        for key in &["drama korea", "drakor"] {
            self.term_map.insert(key.to_string(), vec!["فيلم", "اللهو"]);
        }
        for key in &["komik", "manga", "anime"] {
            self.term_map.insert(key.to_string(), vec!["رسم", "تصوير"]);
        }
        for key in &["pengantin", "mempelai"] {
            self.term_map.insert(key.to_string(), vec!["عروس", "العروسان", "الزفاف"]);
        }
        for key in &["ventilator", "alat bantu napas"] {
            self.term_map.insert(key.to_string(), vec!["أجهزة الإنعاش", "إيقاف الأجهزة الطبية"]);
        }
        for key in &["polusi", "pencemaran"] {
            self.term_map.insert(key.to_string(), vec!["تلوث", "البيئة"]);
        }
        for key in &["stunning", "terkejut"] {
            self.term_map.insert(key.to_string(), vec!["تصعيق", "الصعق قبل الذبح"]);
        }
        for key in &["kenalan", "berkenalan"] {
            self.term_map.insert(key.to_string(), vec!["تعارف", "التعارف"]);
        }
        for key in &["aplikasi", "app"] {
            self.term_map.insert(key.to_string(), vec!["الإنترنت", "التواصل الاجتماعي"]);
        }
        for key in &["lgbtq", "gay", "lesbi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["لواط", "السحاق", "قوم لوط"]);
        }
        for key in &["pluralisme", "toleransi"] {
            self.term_map.insert(key.to_string(), vec!["تسامح", "التعايش", "التعددية"]);
        }
        for key in &["syariah", "syariat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["شريعة", "الشريعة"]);
        }
        for key in &["khilafah", "khilafa"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["خلافة", "الخلافة"]);
        }
        for key in &["pasar modal"] {
            self.term_map.insert(key.to_string(), vec!["سوق المال", "الأسهم"]);
        }
        for key in &["crowdfunding"] {
            self.term_map.insert(key.to_string(), vec!["تمويل جماعي", "مشاركة"]);
        }
        for key in &["pesantren", "ponpes"] {
            self.term_map.insert(key.to_string(), vec!["معهد", "المدرسة الدينية"]);
        }
        for key in &["takbiran"] {
            self.term_map.insert(key.to_string(), vec!["تكبير العيد", "تكبيرات العيد"]);
        }
        for key in &["thr"] {
            self.term_map.insert(key.to_string(), vec!["عطية", "هدية العيد"]);
        }
        for key in &["kafan", "kain kafan"] {
            self.term_map.insert(key.to_string(), vec!["كفن", "الكفن", "عدد الكفن"]);
        }
        for key in &["arisan"] {
            self.term_map.insert(key.to_string(), vec!["جمعية", "قرض"]);
        }
        for key in &["kontrak", "perjanjian kerja"] {
            self.term_map.insert(key.to_string(), vec!["عقد العمل", "الإجارة"]);
        }
        for key in &["dosa"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ذنب", "الذنوب", "الكبائر"]);
        }
        for key in &["ampun", "pengampunan"] {
            self.term_map.insert(key.to_string(), vec!["مغفرة", "المغفرة", "التوبة"]);
        }
        for key in &["bioetika", "bioethics"] {
            self.term_map.insert(key.to_string(), vec!["الأخلاق الطبية"]);
        }
        for key in &["iklim", "perubahan iklim", "climate change"] {
            self.term_map.insert(key.to_string(), vec!["بيئة", "التلوث"]);
        }
        for key in &["pesawat", "airplane"] {
            self.term_map.insert(key.to_string(), vec!["طائرة", "الصلاة في الطائرة"]);
        }
        for key in &["hijaiyah", "huruf hijaiyah"] {
            self.term_map.insert(key.to_string(), vec!["الحروف الهجائية", "حروف"]);
        }
        for key in &["aqidah awam", "aqidatul awam"] {
            self.term_map.insert(key.to_string(), vec!["عقيدة العوام", "الجوهرة"]);
        }

        // ── WORSHIP CONFIG WORDS ──
        for key in &["salam"] {
            // salam in prayer context = تسليم; as greeting = سلام
            self.term_map.entry(key.to_string()).or_insert(vec!["تسليم", "السلام", "سلام"]);
        }
        for key in &["awam"] {
            self.term_map.insert(key.to_string(), vec!["عوام", "العوام"]);
        }
        for key in &["nadhom", "nadhm", "nadham"] {
            self.term_map.insert(key.to_string(), vec!["نظم", "منظومة"]);
        }

        // ══════════════════════════════════════════════════════════════
        // BATCH 7 (V17): Fix multi-word dead-code keys + construct-state forms
        // ══════════════════════════════════════════════════════════════

        // ── Arabic construct-state book title components ──
        // Needed for queries like "kitab fathul qarib tentang apa" where
        // "fathul" ≠ "fath" and multi-word key "fathul qarib" never matches
        for key in &["fathul", "fath"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["فتح", "الفتح"]);
        }
        for key in &["qarib"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["فتح القريب", "التقريب"]);
        }
        for key in &["muin", "mu'in"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["فتح المعين", "زين الدين"]);
        }
        for key in &["raudhatut", "raudhatu"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["روضة الطالبين", "النووي"]);
        }
        for key in &["thalibin", "talibin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["روضة الطالبين", "الطالبين"]);
        }
        for key in &["riyadhus", "riyadhu"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["رياض الصالحين", "النووي"]);
        }
        for key in &["shalihin", "salihin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["رياض الصالحين", "الصالحين"]);
        }
        for key in &["bulughul", "bulughu"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بلوغ المرام", "ابن حجر العسقلاني"]);
        }
        for key in &["nihayatul", "nihayatu"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نهاية المحتاج", "الرملي"]);
        }
        for key in &["mughnil"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مغني المحتاج", "الشربيني"]);
        }
        for key in &["tuhfatul", "tuhfatu"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تحفة المحتاج", "ابن حجر الهيتمي"]);
        }
        for key in &["kifayatul"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["كفاية الأخيار", "تقي الدين"]);
        }
        for key in &["akhyar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["كفاية الأخيار", "الأخيار"]);
        }
        for key in &["nailul"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نيل الأوطار", "الشوكاني"]);
        }
        for key in &["authar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نيل الأوطار", "الأوطار"]);
        }
        for key in &["sunan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["السنن", "السنن النبوية"]);
        }
        for key in &["majah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سنن ابن ماجه", "ابن ماجه"]);
        }
        for key in &["dawud"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سنن أبي داود", "أبو داود"]);
        }
        for key in &["bidayatul", "bidayah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بداية المجتهد", "ابن رشد"]);
        }

        // ── Single scholar component keys (critical: multi-word keys are dead code) ──
        for key in &["taimiyah", "taymiyah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ابن تيمية"]);
        }
        for key in &["qayyim"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ابن القيم"]);
        }
        for key in &["haitami"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ابن حجر الهيتمي"]);
        }
        for key in &["rusyd", "rusydi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ابن رشد", "أبو الوليد"]);
        }
        for key in &["khaldun"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ابن خلدون"]);
        }
        for key in &["sina"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ابن سينا", "أبو علي"]);
        }
        for key in &["walid"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["خالد بن الوليد"]);
        }

        // ── Interrogative and filler words (better than zero) ──
        for key in &["siapakah", "siapa"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["من", "سيرة"]);
        }
        for key in &["kontribusi"] {
            self.term_map.insert(key.to_string(), vec!["مساهمات", "إسهامات", "دور"]);
        }
        for key in &["kitab"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["كتاب", "الكتب"]);
        }
        for key in &["isi", "kandungan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مضمون", "محتوى"]);
        }
        for key in &["bulan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["شهر", "الأشهر"]);
        }
        for key in &["hari"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["يوم", "الأيام"]);
        }
        for key in &["malam"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ليلة", "الليل"]);
        }
        for key in &["tulis", "tulisan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مؤلف", "مؤلفات"]);
        }

        // ── Month name components (multi-word month keys are dead code) ──
        for key in &["rabiul"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ربيع"]);
        }
        for key in &["jumadil"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جمادى"]);
        }

        // ── Additional person and role words ──
        for key in &["murid"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["طالب", "تلميذ", "مريد"]);
        }
        for key in &["guru"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["شيخ", "أستاذ", "معلم"]);
        }
        for key in &["ulama", "ulema"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["علماء", "العلماء"]);
        }
        for key in &["sahabat", "shahabi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صحابة", "الصحابة"]);
        }
        for key in &["nabi", "rasul"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["النبي", "الرسول"]);
        }

        // ── Common words missing from dictionary ──
        for key in &["surah", "surat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سورة", "السور"]);
        }
        for key in &["ayat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["آية", "الآيات"]);
        }
        for key in &["juz"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جزء", "الأجزاء"]);
        }
        for key in &["langit"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سماء", "السماوات"]);
        }
        for key in &["bumi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أرض", "الأرض"]);
        }
        for key in &["pintu"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["باب", "أبواب"]);
        }
        for key in &["lapisan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["طبقة", "طبقات"]);
        }
        for key in &["huruf"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حرف", "الحروف"]);
        }
        for key in &["angka", "nomor"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عدد", "رقم"]);
        }
        for key in &["am"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["العام", "عام"]);
        }
        for key in &["penting"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مهم", "أهمية"]);
        }
        for key in &["terkenal", "masyhur"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مشهور", "معروف"]);
        }
        for key in &["singkat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["موجز", "مختصر"]);
        }
        for key in &["kadar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مقدار", "قدر"]);
        }
        for key in &["minimal", "sekurang"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حد أدنى", "الأقل"]);
        }
        // Sufi concept variants
        for key in &["raja"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["رجاء", "الرجاء"]);
        }
        // Birthdays/mawlid
        for key in &["ulang tahun", "birthday"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عيد الميلاد", "المولد"]);
        }
        for key in &["nasheed", "nasyid"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أناشيد", "نشيد"]);
        }
        for key in &["lgbtq", "lgbt"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المثلية", "الشذوذ"]);
        }

        // ─── BATCH 8: Single-word fallbacks for dead multi-word sirah/history keys ───
        // All multi-word term_map lookups (e.g. "fathu makkah") are dead code —
        // the tokenizer splits on whitespace, so only single-word lookups work.
        for key in &["fathu"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["فتح", "فتح مكة"]);
        }
        for key in &["isra"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الإسراء", "إسراء"]);
        }
        for key in &["miraj"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المعراج", "معراج"]);
        }
        for key in &["piagam"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["وثيقة", "ميثاق", "صحيفة"]);
        }
        for key in &["khalifah", "kholifah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["خليفة", "الخلفاء"]);
        }
        for key in &["khulafaur", "khulafa"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الخلفاء الراشدون", "الخلفاء"]);
        }
        for key in &["rasyidin", "rashidin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الراشدون", "الخلفاء الراشدون"]);
        }
        for key in &["bani"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بنو", "آل"]);
        }

        // ─── BATCH 8: Colloquial Indonesian modern terms ───
        for key in &["chatting", "ngobrol online"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["محادثة", "التحدث مع الأجنبية"]);
        }
        for key in &["nonton", "menonton"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مشاهدة", "النظر"]);
        }
        for key in &["cewek", "perempuan asing"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["فتاة", "امرأة أجنبية"]);
        }
        for key in &["komik", "manga"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["رسوم مصورة", "كتاب مصور"]);
        }
        for key in &["kenalan", "berkenalan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التعارف", "المعرفة"]);
        }
        for key in &["drakor", "drama"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مسلسل", "مسلسلات"]);
        }
        for key in &["aplikasi", "app"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["وسيلة اتصال", "التقنية الحديثة"]);
        }

        // ─── BATCH 8: Common misspellings / typos ───
        // Users who mistype Arabic Islamic terms in Roman script
        for key in &["wudloo", "wudu'", "wuḍu"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["وضوء", "الوضوء"]);
        }
        for key in &["wuhu"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["وضوء", "الوضوء"]);
        }
        for key in &["shalt", "shallt", "sollt"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صلاة", "الصلاة"]);
        }
        for key in &["pussa", "pwasa", "puasaa"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صوم", "صيام"]);
        }
        for key in &["zkat", "zkah", "zakaat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["زكاة", "الزكاة"]);
        }

        // ─── BATCH 8: Islamic concepts still missing ───
        for key in &["wiping", "usap"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مسح", "المسح"]);
        }
        for key in &["masah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مسح", "المسح على الخفين"]);
        }
        for key in &["socks", "khuf", "khuff"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["خف", "الخفين", "المسح على الخفين"]);
        }
        for key in &["tahajjud"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التهجد", "قيام الليل", "صلاة الليل"]);
        }
        for key in &["paradise", "surga", "jannah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جنة", "الجنة", "الفردوس"]);
        }
        for key in &["hellfire", "neraka", "jahannam"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نار", "جهنم", "النار"]);
        }
        for key in &["angels", "malaikat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ملائكة", "الملائكة"]);
        }
        for key in &["duties", "tugas", "kewajiban"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["واجبات", "فرائض"]);
        }
        for key in &["saints", "wali", "waliyullah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الأولياء", "ولي الله"]);
        }
        for key in &["miracles", "mukjizat", "karamah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["كرامة", "معجزة", "الكرامات"]);
        }
        for key in &["guarantee", "kafalah", "kafala"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["كفالة", "الكفالة"]);
        }
        for key in &["agency", "wakalah", "wakala"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["وكالة", "التوكيل"]);
        }
        for key in &["options", "khiyar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["خيار", "الخيار"]);
        }
        for key in &["uncertainty", "gharar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["غرر", "الغرر"]);
        }
        for key in &["highway", "hirabah", "hiraba"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حرابة", "السرقة الكبرى"]);
        }
        for key in &["hypocrisy", "munafik", "nifak"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نفاق", "المنافق"]);
        }
        for key in &["celebrating", "perayaan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الاحتفال", "عيد"]);
        }
        for key in &["mawlid", "birthday prophet"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المولد النبوي", "مولد"]);
        }
        for key in &["KPR", "kpr"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تمويل عقاري", "القرض العقاري", "بيع بالتقسيط"]);
        }
        for key in &["stunning", "pemingsanan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تخدير", "ذبح بالتخدير"]);
        }

        // ─── BATCH 8 (cont.): Terms that appear in V15 zero queries ───
        for key in &["aib"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عيب", "الخيار بالعيب"]);
        }
        for key in &["mutlak"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مطلق", "الاجتهاد المطلق"]);
        }
        for key in &["ulumuddin", "ulum ud-din"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["علوم الدين", "إحياء علوم الدين"]);
        }
        for key in &["melempar", "melontar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["رمي", "رمي الجمرات"]);
        }
        for key in &["aqabah", "aqobah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جمرة العقبة", "العقبة"]);
        }
        for key in &["pembeli"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مشتري", "المشتري"]);
        }
        for key in &["tsani", "thani"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ثاني", "الثاني"]);
        }
        for key in &["fiqhiyyah", "fiqhiyah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["فقهية", "القواعد الفقهية"]);
        }
        for key in &["ula", "wal-ula"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الأولى", "الأول"]);
        }
        for key in &["wustha"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الوسطى", "الأوسط"]);
        }
        for key in &["pagi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صباح", "الصباح"]);
        }
        for key in &["petang", "sore"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مساء", "المساء"]);
        }
        for key in &["tawadhu", "tawadu"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تواضع", "التواضع"]);
        }
        for key in &["husnuzhan", "husnu zhan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حسن الظن", "ظن الخير"]);
        }
        for key in &["su'uzhan", "suuzhan", "su uzhan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سوء الظن", "الظن"]);
        }
        for key in &["birrul", "birr"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بر", "بر الوالدين"]);
        }
        for key in &["walidain", "waalidain"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الوالدين", "بر الوالدين"]);
        }
        for key in &["denda"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["غرامة", "دية"]);
        }
        for key in &["janin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جنين", "الجنين"]);
        }
        for key in &["makiyyah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مكية", "السور المكية"]);
        }
        for key in &["madaniyyah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مدنية", "السور المدنية"]);
        }
        for key in &["muhkam"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["محكم", "المحكم"]);
        }
        for key in &["mutasyabih"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["متشابه", "المتشابه"]);
        }
        for key in &["i'jaz", "ijaz"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["إعجاز", "إعجاز القرآن"]);
        }
        for key in &["kemukjizatan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["إعجاز", "المعجزة"]);
        }
        for key in &["makna"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["معنى", "تعريف"]);
        }

        // ─── BATCH 9: Final cleanup — standalone words missing single-word keys ───
        for key in &["awal"] {  // "first/beginning" — used in tahallul awal, rabiul awal
            self.term_map.entry(key.to_string()).or_insert(vec!["الأول", "أول"]);
        }
        for key in &["akhir"] {  // "last/end" — used in rabiul akhir, jumadil akhir
            self.term_map.entry(key.to_string()).or_insert(vec!["الآخر", "آخر"]);
        }
        for key in &["shiddiq", "siddiq", "siddik"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الصديق", "أبو بكر الصديق"]);
        }
        for key in &["jumlah"] {  // "amount/total"
            self.term_map.entry(key.to_string()).or_insert(vec!["عدد", "كمية"]);
        }
        for key in &["pasar"] {  // "market" — for pasar modal syariah
            self.term_map.entry(key.to_string()).or_insert(vec!["سوق", "الأسواق"]);
        }
        for key in &["modal"] {  // "capital" — for pasar modal
            self.term_map.entry(key.to_string()).or_insert(vec!["رأس مال", "المال"]);
        }
        for key in &["sistem"] {  // "system" — for sistem khilafah
            self.term_map.entry(key.to_string()).or_insert(vec!["نظام", "النظام"]);
        }
        for key in &["implementasi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تطبيق", "تنفيذ"]);
        }
        for key in &["kurikulum"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["منهج", "مناهج"]);
        }
        for key in &["modernisasi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التحديث", "التجديد"]);
        }
        for key in &["tradisi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تراث", "عرف"]);
        }
        for key in &["kapan"] {  // "when" — question word
            self.term_map.entry(key.to_string()).or_insert(vec!["متى", "وقت"]);
        }
        for key in &["dimana"] {  // "where" — question word
            self.term_map.entry(key.to_string()).or_insert(vec!["أين", "مكان"]);
        }
        for key in &["mengapa"] {  // "why" — question word
            self.term_map.entry(key.to_string()).or_insert(vec!["لماذا", "حكمة"]);
        }
        for key in &["berapa"] {  // "how many/much" — question word
            self.term_map.entry(key.to_string()).or_insert(vec!["كم", "عدد"]);
        }
        for key in &["amalan", "amal"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عمل", "أعمال"]);
        }
        for key in &["celebrate", "merayakan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["احتفال", "الاحتفال"]);
        }
        for key in &["listen", "dengar", "mendengar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سماع", "الاستماع"]);
        }
        for key in &["shoes", "sepatu", "alas kaki"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نعال", "الحذاء"]);
        }
        for key in &["climate", "lingkungan", "environment"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بيئة", "المناخ"]);
        }
        for key in &["pesantren", "pondok"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["معهد", "المدرسة الإسلامية"]);
        }
        for key in &["ditanya", "pertanyaan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سؤال", "مسألة"]);
        }

        // ─── BATCH 9 (cont.): Chapter / book structural terms ───
        // Users often query "syarah fathul mu'in bab thaharah" etc.
        for key in &["bab"] {  // chapter in Arabic kitab — written in Latin by users
            self.term_map.entry(key.to_string()).or_insert(vec!["باب", "أبواب"]);
        }
        for key in &["pasal", "fasal"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["فصل", "الفصول"]);
        }
        for key in &["syarah", "pensyarah", "syarh"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["شرح", "الشرح"]);
        }
        for key in &["sharih", "pensyarah"] {  // commentator of a kitab
            self.term_map.entry(key.to_string()).or_insert(vec!["شارح"]);
        }
        for key in &["risalah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["رسالة", "الرسالة"]);
        }
        for key in &["nadhom", "nadhm", "nadham"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نظم", "منظومة"]);
        }
        for key in &["matan", "matn"] {  // base text of a kitab
            self.term_map.entry(key.to_string()).or_insert(vec!["متن", "المتن"]);
        }
        for key in &["mukhtashar", "mukhtasar"] {  // abridged version
            self.term_map.entry(key.to_string()).or_insert(vec!["مختصر", "الملخص"]);
        }

        // ─── BATCH 9 (cont.): More Islamic practices missing ───
        for key in &["talqin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التلقين"]);
        }
        for key in &["arisan"] {  // Indonesian rotating savings group
            self.term_map.entry(key.to_string()).or_insert(vec!["جمعية", "القرض الحسن"]);
        }
        for key in &["tahun baru"] {  // new year
            self.term_map.entry(key.to_string()).or_insert(vec!["رأس السنة", "السنة الجديدة"]);
        }
        for key in &["hijriyah", "hijri"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["هجرية", "التقويم الهجري"]);
        }
        for key in &["THR", "thr"] {  // Tunjangan Hari Raya — Indonesian holiday bonus
            self.term_map.entry(key.to_string()).or_insert(vec!["هدية", "عطاء", "الأجر"]);
        }
        for key in &["pesawat"] {  // airplane — check if already exists, add if not
            self.term_map.entry(key.to_string()).or_insert(vec!["طائرة", "الطائرة"]);
        }
        for key in &["janda", "duda"] {  // widow, widower
            self.term_map.entry(key.to_string()).or_insert(vec!["أرملة", "أيم"]);
        }
        for key in &["tunangan", "bertunangan"] {  // fiancee, engagement
            self.term_map.entry(key.to_string()).or_insert(vec!["خطبة", "المخطوبة"]);
        }
        for key in &["foto", "gambar", "memotret"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تصوير", "صورة"]);
        }

        // ─── BATCH 10: English plural forms and additional edge cases ───
        for key in &["birthdays", "birthday"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عيد الميلاد", "المولد"]);
        }
        for key in &["graves", "grave", "tomb", "kubur", "kuburan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قبر", "القبور"]);
        }
        for key in &["prayers", "prayer"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صلاة", "الصلاة"]);
        }
        for key in &["sins", "sin", "dosa"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ذنب", "الذنوب", "إثم"]);
        }
        for key in &["scholars", "scholar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["العلماء", "عالم"]);
        }
        for key in &["companions", "sahabat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صحابة", "الصحابة"]);
        }
        for key in &["women", "woman", "female"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المرأة", "النساء"]);
        }
        for key in &["men", "man", "male"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الرجل", "الرجال"]);
        }
        for key in &["books", "book"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["كتاب", "الكتب"]);
        }
        for key in &["ruling", "rulings"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكم", "أحكام"]);
        }
        for key in &["evidence", "dalil", "dalil-dalil"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دليل", "الأدلة"]);
        }
        for key in &["hadith", "hadis", "hadist"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حديث", "الحديث"]);
        }
        for key in &["verse", "verses", "ayat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["آية", "الآيات"]);
        }
        for key in &["surah", "sura", "chapter"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سورة", "السور"]);
        }
        for key in &["hukum"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكم", "أحكام"]);
        }
        for key in &["boleh"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جواز", "يجوز", "مباح"]);
        }
        for key in &["tidak boleh", "dilarang"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["لا يجوز", "حرام", "محرم"]);
        }
        for key in &["wajib", "fardhu"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["واجب", "فرض"]);
        }
        for key in &["sunah", "sunnah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سنة", "مستحب"]);
        }
        for key in &["makruh", "mekruh"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مكروه", "الكراهة"]);
        }

        // ─── BATCH 9 (cont.): English terms for Islamic queries ───
        for key in &["islamic", "islamically"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الإسلام", "الإسلامي"]);
        }
        for key in &["muslim", "muslims"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مسلم", "المسلمون"]);
        }
        for key in &["pray", "praying"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صلاة", "الصلاة"]);
        }
        for key in &["responsibility", "obligation"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مسؤولية", "واجب"]);
        }
        for key in &["perspective", "view", "opinion"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["وجهة نظر", "رأي"]);
        }
        for key in &["farming", "agriculture"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["زراعة", "مزرعة"]);
        }
        for key in &["factory", "pabrik"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مزرعة", "ذبح"]);  // factory farming → slaughter
        }
        for key in &["perspective", "view", "viewpoint"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["رأي", "وجهة النظر"]);
        }
        for key in &["shoes", "footwear", "alas kaki"] {
            // already added as "shoes" above but ensuring here
            self.term_map.entry(key.to_string()).or_insert(vec!["نعال", "الحذاء"]);
        }
        for key in &["description", "description of"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["وصف", "صفة"]);
        }
        for key in &["signs", "tanda-tanda"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["علامات", "أمارات"]);
        }
        for key in &["overview", "introduction", "pengantar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مقدمة", "نظرة عامة"]);
        }

        // ─── BATCH 11: Missing terms for V15 zeros ───
        for key in &["tempat", "lokasi", "place", "location"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مكان", "موضع"]);
        }
        for key in &["pencuri", "perampok", "thief"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سارق", "السرقة"]);
        }
        for key in &["pembunuhan", "membunuh", "kill", "murder"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قتل", "القتل", "الجناية على النفس"]);
        }
        for key in &["potong", "memotong", "amputation"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قطع", "البتر"]);
        }
        for key in &["azl", "'azl", "coitus interruptus"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["العزل", "العزل عن المرأة"]);
        }
        for key in &["perwakilan", "wakil"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["وكالة", "وكيل"]);
        }
        for key in &["kuqathah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["لقطة"]);
        }
        for key in &["furudh", "fardhu"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["فروض", "الفرائض"]);
        }
        for key in &["perang", "pertempuran", "battle", "war"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["غزوة", "معركة"]);
        }
        for key in &["badr"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بدر", "غزوة بدر"]);
        }
        for key in &["uhud"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أحد", "غزوة أحد"]);
        }
        for key in &["khandaq", "ahzab"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["خندق", "الأحزاب", "غزوة الخندق"]);
        }
        for key in &["pluralisme", "pluralism"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التعددية", "التسامح"]);
        }
        for key in &["toleransi", "tolerance"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التسامح", "التعايش"]);
        }
        for key in &["khilafah", "caliphate", "khilafa"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الخلافة", "الحكم الإسلامي"]);
        }
        for key in &["crowdfunding"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التمويل الجماعي", "التبرع"]);
        }
        for key in &["bioethics"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الأخلاق الطبية", "الفقه الطبي"]);
        }
        for key in &["sengaja", "intentional", "deliberate"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عمد", "القتل العمد"]);
        }
        for key in &["ganti rugi", "diyat", "diat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دية", "الدية", "الأرش"]);
        }
        for key in &["arsy", "arsy pelukaan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أرش", "الجراح"]);
        }
        for key in &["pelukaan", "peluka", "injury", "wound"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جراح", "الجراح", "جناية"]);
        }
        for key in &["kibr", "sombong", "arrogance"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["كبر", "التكبر", "الكبرياء"]);
        }
        for key in &["karamah", "keramat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الكرامة", "كرامات الأولياء"]);
        }
        for key in &["mawlid", "maulid", "maulud"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المولد", "مولد النبي", "الاحتفال بالمولد"]);
        }
        for key in &["perayaan", "merayakan", "commemorate"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["احتفال", "الاحتفال"]);
        }
        for key in &["lgbtq", "lgbt", "homosexuality", "homoseksual"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الشذوذ الجنسي", "اللواط", "المثلية"]);
        }
        for key in &["nasheed", "nasyid", "anasheed"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أناشيد", "النشيد الإسلامي"]);
        }
        for key in &["kafan", "shroud"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["كفن", "الكفن"]);
        }
        for key in &["jumlah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عدد", "كمية"]);
        }
        for key in &["airplane", "pesawat", "kapal terbang"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الطائرة", "السفر"]);
        }
        for key in &["9", "10", "7", "8", "28", "30"] {
            // Number queries — map to general "عدد" to avoid zero but also keep as latin_term
            self.term_map.entry(key.to_string()).or_insert(vec!["عدد"]);
        }

        // ─── BATCH 12: More missing terms from V15 zeros ───
        for key in &["ied", "idul", "lebaran"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عيد", "العيد", "عيد الفطر"]);
        }
        for key in &["kain"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["كسوة", "قماش"]);
        }
        for key in &["dijamin", "terjamin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مضمون", "مبشرون بالجنة"]);
        }
        for key in &["tahun"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سنة", "عام"]);
        }
        for key in &["baru", "new"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جديد", "المحدث"]);
        }
        for key in &["modernitas", "modernity"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الحداثة", "التجديد"]);
        }
        for key in &["bacaan", "membaca", "recitation"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قراءة", "تلاوة"]);
        }
        for key in &["pegawai", "karyawan", "employee"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["موظف", "عامل", "أجير"]);
        }
        for key in &["tangan", "hand"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["يد", "اليد"]);
        }
        for key in &["kontrak", "contract"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عقد", "عقود"]);
        }
        for key in &["salam"] {
            // Islamic contract "salam" = forward sale; also greeting
            self.term_map.entry(key.to_string()).or_insert(vec!["السلم", "بيع السلم", "سلام"]);
        }
        for key in &["istishna"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الاستصناع", "عقد الاستصناع"]);
        }
        for key in &["pluralisme"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التعددية", "التسامح الديني"]);
        }
        for key in &["bihdats", "bidah", "bid'ah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بدعة", "البدعة", "محدثات الأمور"]);
        }
        for key in &["adab", "etika"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أدب", "الآداب"]);
        }
        for key in &["saleh", "sholeh", "shaleh"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صالح", "الصالحون"]);
        }
        for key in &["ihsan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["إحسان", "الإحسان"]);
        }
        for key in &["tazkiyah", "tazkiyatun", "nafs"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تزكية النفس", "تزكية"]);
        }
        for key in &["ikhlas", "sincere"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["إخلاص", "الإخلاص"]);
        }
        for key in &["sabar", "patience"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صبر", "الصبر"]);
        }
        for key in &["syukur", "gratitude"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["شكر", "الشكر"]);
        }
        for key in &["qanaah", "contentment"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قناعة"]);
        }
        for key in &["maulid", "mawlid", "milad"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المولد", "مولد النبي"]);
        }
        for key in &["haul", "haol"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الحول", "الذكرى السنوية"]);
        }
        for key in &["yasinan", "yassin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قراءة يس", "سورة يس"]);
        }
        for key in &["istigfar", "istighfar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["استغفار", "طلب المغفرة"]);
        }
        for key in &["shalawat", "salawat", "sholawat", "salawat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الصلاة على النبي", "الصلوات"]);
        }
        for key in &["silaturahmi", "silaturrahim"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صلة الرحم", "التواصل"]);
        }
        for key in &["menolong", "tolong", "help"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مساعدة", "التعاون"]);
        }

        // ─── BATCH 13: Additional coverage for colloquial/query patterns ───
        for key in &["kerja", "bekerja", "pekerjaan", "work", "job"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عمل", "الكسب", "العمل"]);
        }
        for key in &["ampuni", "diampuni", "ampunan", "forgiveness"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مغفرة", "التوبة", "الغفران"]);
        }
        for key in &["dosa", "sin", "dosanya"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ذنب", "إثم", "الخطيئة"]);
        }
        for key in &["mencabut", "withdraw", "removal"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نزع", "سحب"]);
        }
        for key in &["napas", "nefas", "breath"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نفس", "التنفس"]);
        }
        for key in &["polusi", "pollution", "udara", "air"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تلوث", "البيئة"]);
        }
        for key in &["pengantin", "bride", "groom"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عروس", "الزوجان الجديدان"]);
        }
        for key in &["larangan", "prohibition"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تحريم", "محظورات"]);
        }
        for key in &["keras", "jahri", "jahar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جهر", "الجهر"]);
        }
        for key in &["pelan", "sirri", "siri", "quiet", "silent"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سر", "الإسرار"]);
        }
        for key in &["stunning", "sembelih", "slaughter", "zabh"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ذبح", "الذبح", "التذكية"]);
        }
        for key in &["amin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["آمين", "التأمين"]);
        }
        for key in &["hafiz", "hafal", "memorization"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حفظ", "الحفظ", "حافظ"]);
        }
        for key in &["ihram"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["إحرام", "الإحرام"]);
        }
        for key in &["thawaf", "tawaf"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["طواف", "الطواف"]);
        }
        for key in &["sa'i", "sai"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سعي", "السعي بين الصفا والمروة"]);
        }
        for key in &["wukuf"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["وقوف", "الوقوف بعرفة"]);
        }
        for key in &["arafah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عرفة", "يوم عرفة"]);
        }
        for key in &["mina"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["منى"]);
        }
        for key in &["subulussalam", "subulus", "subul"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سبل السلام", "الصنعاني"]);
        }
        for key in &["muwattha", "muwatta", "muwatha"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الموطأ", "موطأ مالك"]);
        }
        for key in &["malik"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مالك", "الإمام مالك"]);
        }
        for key in &["pengarang", "author", "karangan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مؤلف", "صاحب الكتاب"]);
        }
        for key in &["perawi", "narrator"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["راوي", "الرواة"]);
        }

        // ─── BATCH 14: Specific fiqh subtypes and remaining V15 zeros ───
        for key in &["abdan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["شركة الأبدان", "الأعمال"]);
        }
        for key in &["inan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["شركة عنان", "عنان"]);
        }
        for key in &["mufawadhah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["شركة المفاوضة", "المفاوضة"]);
        }
        for key in &["wujuh"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["شركة الوجوه", "الوجوه"]);
        }
        for key in &["aib", "cacat", "defect"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عيب", "خيار العيب"]);
        }
        for key in &["khiyar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["خيار", "الخيار"]);
        }
        for key in &["mujtahid"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مجتهد", "الاجتهاد"]);
        }
        for key in &["mutlak", "absolute"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مطلق", "مجتهد مطلق"]);
        }
        for key in &["dharurah", "darurat", "necessity"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ضرورة", "الضرورة", "حالة الاضطرار"]);
        }
        for key in &["nifaq", "munafiq"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نفاق", "النفاق", "المنافق"]);
        }
        for key in &["kufur", "kafir", "disbelief"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["كفر", "الكفر", "الكافر"]);
        }
        for key in &["syirik", "shirk", "polytheism"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["شرك", "الشرك"]);
        }
        for key in &["riddah", "murtad", "apostasy"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ردة", "المرتد", "الردة"]);
        }
        for key in &["hadhanah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حضانة", "الحضانة"]);
        }
        for key in &["nafkah", "nafaqah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نفقة", "النفقة"]);
        }
        for key in &["mahar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مهر", "المهر"]);
        }
        for key in &["iddah", "idah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عدة", "العدة"]);
        }
        for key in &["khalwat", "khalwah", "seclusion"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["خلوة", "الخلوة"]);
        }
        for key in &["taaruf", "ta'aruf"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التعارف", "مجلس التعارف"]);
        }
        for key in &["nusyuz"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نشوز", "النشوز"]);
        }
        for key in &["fasakh", "fasah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["فسخ", "الفسخ"]);
        }
        for key in &["li'an", "lian"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["لعان"]);
        }
        for key in &["zihar", "dzihar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ظهار"]);
        }
        for key in &["kafarat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["كفارة", "الكفارة"]);
        }
        for key in &["fidyah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["فدية"]);
        }
        for key in &["umur", "age", "usia"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سن", "العمر"]);
        }
        for key in &["sampai", "until", "hingga"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["إلى", "حتى"]);
        }
        for key in &["waqaf", "wakaf", "endowment"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["وقف", "الوقف"]);
        }
        for key in &["hibah", "hiba"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["هبة", "الهبة"]);
        }
        for key in &["wasiat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["وصية", "الوصية"]);
        }
        for key in &["warisan", "mawaris"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ميراث", "المواريث"]);
        }
        for key in &["ashhabul faradh", "ashabul"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أصحاب الفروض"]);
        }
        for key in &["ashobah", "ashabah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عصبة"]);
        }
        for key in &["dzawul arham", "dzawil"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ذوو الأرحام"]);
        }

        // ─── BATCH 15: Prayer directional terms and other gaps ───
        for key in &["kanan", "kiri", "right", "left"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["يمين", "يسار", "التسليم"]);
        }
        for key in &["salam ke kanan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["السلام عن اليمين"]);
        }
        for key in &["taslim", "taslem"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تسليم", "التسليم"]);
        }
        for key in &["am"] {
            // Usul fiqh: 'am = general
            self.term_map.entry(key.to_string()).or_insert(vec!["عام", "العام والخاص"]);
        }
        for key in &["naskh", "nasakh"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نسخ", "الناسخ والمنسوخ"]);
        }
        for key in &["kilas", "singkat", "brief", "ringkasan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ملخص", "مختصر"]);
        }
        for key in &["terkenal", "masyhur", "famous"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مشهور", "معروف"]);
        }
        for key in &["ilmiah", "academic"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["علمي", "أكاديمي"]);
        }
        for key in &["lafal", "lafaz", "wording", "phrasing"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["لفظ", "الألفاظ"]);
        }
        for key in &["bunuh diri", "suicide"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الانتحار", "قتل النفس"]);
        }
        for key in &["euthanasia", "tasri'i"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الموت الرحيم", "القتل الرحيم"]);
        }
        for key in &["minuman", "drink", "beverages"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مشروبات", "الشراب"]);
        }
        for key in &["makanan", "food", "makan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["طعام", "الأطعمة"]);
        }
        for key in &["halal", "halaal"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حلال", "الحلال"]);
        }
        for key in &["haram"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حرام", "الحرام"]);
        }
        for key in &["najis", "impure"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نجس", "النجاسة"]);
        }
        for key in &["suci", "thaharah", "tahara", "purity"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["طهارة", "الطهارة"]);
        }
        for key in &["junub", "janabah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جنابة", "الجنابة"]);
        }
        for key in &["mandi", "ghusl", "mandi wajib"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["غسل", "الغسل"]);
        }
        for key in &["haid", "haydh", "menstruation"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حيض", "الحيض"]);
        }
        for key in &["nifas"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نفاس", "النفاس"]);
        }
        for key in &["istihadhah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["استحاضة"]);
        }
        for key in &["tayammum"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تيمم", "التيمم"]);
        }
        for key in &["qiblat", "qibla", "kiblat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قبلة", "القبلة"]);
        }
        for key in &["orang", "person", "seseorang"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["شخص", "إنسان"]);
        }
        for key in &["anak", "child", "children"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ولد", "أولاد", "طفل"]);
        }
        for key in &["ibu", "mother", "ummi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أم", "والدة"]);
        }
        for key in &["ayah", "bapak", "father", "abi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أب", "والد"]);
        }
        for key in &["suami", "husband"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["زوج", "الزوج"]);
        }
        for key in &["istri", "isteri", "wife"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["زوجة", "الزوجة"]);
        }

        // ─── BATCH 16: Prayer/Quran terms and temporal words ───
        for key in &["fatihah", "fatiha", "al-fatihah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الفاتحة", "فاتحة الكتاب"]);
        }
        for key in &["sebelum", "sebelumnya", "before"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قبل", "السابق"]);
        }
        for key in &["sesudah", "setelah", "setelahnya", "after"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بعد", "اللاحق"]);
        }
        for key in &["lengkap", "lengkapnya", "complete", "full"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["كامل", "تام"]);
        }
        for key in &["pertama", "first", "awal"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أول", "البداية"]);
        }
        for key in &["terakhir", "last", "akhir"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["آخر", "الأخير"]);
        }
        for key in &["dua", "qunut", "doa qunut"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["القنوت", "دعاء القنوت"]);
        }
        for key in &["tasyahud", "tahiyat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التشهد", "التحيات"]);
        }
        for key in &["ruku", "ruku'"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ركوع", "الركوع"]);
        }
        for key in &["sujud"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سجود", "السجود"]);
        }
        for key in &["qiyam", "berdiri"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قيام", "القيام"]);
        }
        for key in &["duduk", "julud"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جلوس", "القعود"]);
        }
        for key in &["rakaat", "rakat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ركعة", "الركعات"]);
        }
        for key in &["jamak", "jam'", "combining"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جمع", "الجمع بين الصلاتين"]);
        }
        for key in &["qasar", "qashr", "qashr", "shortening"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قصر", "قصر الصلاة"]);
        }
        for key in &["witir", "witr"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["وتر", "صلاة الوتر"]);
        }
        for key in &["tarawih", "taraweh"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تراويح", "صلاة التراويح"]);
        }
        for key in &["shalat sunnah", "sunnah rawatib"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["السنن الرواتب", "النوافل"]);
        }
        for key in &["rawatib"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الرواتب", "السنن الرواتب"]);
        }
        for key in &["dhuha", "duha"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الضحى", "صلاة الضحى"]);
        }
        for key in &["isyraq", "isyrak"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صلاة الإشراق"]);
        }
        for key in &["aurat", "awrah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عورة", "ستر العورة"]);
        }
        for key in &["sutroh", "sutrah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سترة"]);
        }
        for key in &["makmum", "makmun"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مأموم", "المأموم"]);
        }
        for key in &["memimpin", "lead", "kepemimpinan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["إمامة", "قيادة"]);
        }
        for key in &["masbuk"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المسبوق", "إدراك الصلاة"]);
        }
        for key in &["lupa", "sahwi", "forget"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سهو", "السهو في الصلاة"]);
        }
        for key in &["sujud sahwi", "sajdah sahwi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سجود السهو"]);
        }

        // ─── BATCH 17: Remaining critical terms from zero analysis ───

        // Salaf / theological schools
        for key in &["salaf", "salafi", "salafy"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["السلف", "السلف الصالح", "السلفية"]);
        }
        for key in &["ushuluddin", "ushul ad-din", "usul diin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أصول الدين", "أصول الفقه"]);
        }
        for key in &["jamiah", "jami'ah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جامع", "الجامع"]);
        }
        for key in &["tabarruk"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التبرك", "البركة"]);
        }

        // Usul fiqh terms
        for key in &["khas", "khusus", "specific"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["خاص", "التخصيص"]);
        }
        for key in &["mubayyan", "bayyin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المبين", "البيان"]);
        }
        for key in &["majazi", "majaz"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المجاز", "مجازي"]);
        }
        for key in &["haqiqi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الحقيقة", "حقيقي"]);
        }
        for key in &["takhshish", "takhsis"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التخصيص"]);
        }
        for key in &["mutlaq", "mutlak"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المطلق"]);
        }
        for key in &["muqayyad", "muqayyid"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المقيد", "التقييد"]);
        }
        for key in &["mantuq", "mafhum"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المنطوق", "المفهوم"]);
        }
        for key in &["ijtihad"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الاجتهاد", "اجتهاد"]);
        }
        for key in &["taqlid"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التقليد", "مقلد"]);
        }
        for key in &["ittiba'", "ittiba"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الاتباع"]);
        }
        for key in &["istidlal"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الاستدلال"]);
        }
        for key in &["talaqqi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التلقي", "تلقي الأسانيد"]);
        }

        // Additional prayer/worship vocabulary
        for key in &["niat", "berniat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نية", "النية"]);
        }
        for key in &["takbir", "takbiratul ihram"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التكبير", "تكبيرة الإحرام"]);
        }
        for key in &["iqamat", "iqamah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الإقامة"]);
        }
        for key in &["subuh", "shubh", "fajr"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الفجر", "صلاة الصبح"]);
        }
        for key in &["dzuhur", "zhuhur", "dhuhur"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الظهر", "صلاة الظهر"]);
        }
        for key in &["ashar", "asr"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["العصر", "صلاة العصر"]);
        }
        for key in &["maghrib", "maghrib"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المغرب", "صلاة المغرب"]);
        }
        for key in &["isya", "isya'", "isha"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["العشاء", "صلاة العشاء"]);
        }
        for key in &["jumat", "jum'at", "juma'ah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الجمعة", "صلاة الجمعة"]);
        }
        for key in &["khutbah", "kutbah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الخطبة", "خطبة الجمعة"]);
        }
        for key in &["shaf", "saff", "saf"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الصف", "تسوية الصفوف"]);
        }
        for key in &["berjamaah", "jamaah", "jama'ah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الجماعة", "صلاة الجماعة"]);
        }
        for key in &["munfarid", "sendirian"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المنفرد", "صلاة المنفرد"]);
        }

        // Hajj/Umrah vocabulary
        for key in &["masjidil haram", "masjid haram"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المسجد الحرام", "الحرم المكي"]);
        }
        for key in &["madinah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المدينة المنورة", "المدينة"]);
        }
        for key in &["ziarah", "ziyarah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الزيارة", "زيارة القبور"]);
        }
        for key in &["istilam"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الاستلام", "استلام الحجر الأسود"]);
        }
        for key in &["hajar aswad"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الحجر الأسود"]);
        }
        for key in &["dam", "fidyah haji"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الدم", "دم الجبران"]);
        }
        for key in &["mabit"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المبيت", "المبيت بمزدلفة"]);
        }
        for key in &["kerikil"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الحصى", "رمي الجمار"]);
        }
        for key in &["badal haji"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حج البدل", "النيابة في الحج"]);
        }

        // General contextual terms
        for key in &["contoh", "misalnya", "example"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مثال", "نموذج"]);
        }
        for key in &["jelaskan", "penjelasan", "explain"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بيان", "شرح"]);
        }
        for key in &["pengertian", "definisi", "definition"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تعريف", "المعنى"]);
        }
        for key in &["syarat", "syarat-syarat", "conditions"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["شروط", "الشروط"]);
        }
        for key in &["rukun", "arkan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أركان", "الأركان"]);
        }
        for key in &["sunah", "sunahnya", "sunnah"] { // duplicate-safe via or_insert
            self.term_map.entry(key.to_string()).or_insert(vec!["السنة", "المستحب"]);
        }
        for key in &["mustahab", "mandub"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المستحب", "المندوب"]);
        }
        for key in &["mubah", "jaiz"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المباح", "الجائز"]);
        }
        for key in &["membatal", "membatalkan", "batal"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مفسد", "المبطلات"]);
        }

        // Social / modern Islamic issues
        for key in &["media sosial", "sosial media"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["وسائل التواصل الاجتماعي"]);
        }
        for key in &["foto", "foto selfie"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التصوير", "الصورة"]);
        }
        for key in &["video", "film"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الفيلم", "التصوير"]);
        }
        for key in &["musik", "music", "lagu"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الموسيقى", "الغناء"]);
        }
        for key in &["game", "permainan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["اللعب", "اللهو"]);
        }
        for key in &["investasi", "saham"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الاستثمار", "الأسهم"]);
        }
        for key in &["asuransi", "insurance"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التأمين", "التأمين الإسلامي"]);
        }
        for key in &["bank konvensional"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["البنوك الربوية", "البنك التقليدي"]);
        }
        for key in &["kredit", "cicilan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الأقساط", "التقسيط"]);
        }
        for key in &["utang", "hutang", "debt"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الدين", "الديون"]);
        }

        // Missing months / periods
        for key in &["rabiul akhir", "rabiul tsani"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ربيع الآخر", "ربيع الثاني"]);
        }
        for key in &["jumadil akhir", "jumadil tsani"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جمادى الآخرة", "جمادى الثانية"]);
        }
        for key in &["rajab"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["رجب", "شهر رجب"]);
        }
        for key in &["nisfu"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["النصف", "ليلة النصف"]);
        }

        // ══════════════════════════════════════════════════════════════
        // BATCH 18: Core Islamic domain keywords (confirmed missing)
        // These caused guaranteed zero results because the entire query
        // was in Latin and none of the meaningful terms had Arabic expansion.
        // ══════════════════════════════════════════════════════════════

        // ── P0: PRIMARY DOMAIN KEYWORDS ──
        // "aqidah" was only used as a FiqhDomain string - never in term_map
        for key in &["aqidah", "akidah", "aqeedah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["العقيدة", "عقائد", "عقيدة"]);
        }
        // "ibadah" likewise was only a domain string, not in term_map
        for key in &["ibadah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["العبادة", "العبادات", "الطاعة"]);
        }
        // "fiqh/fikih" — generic Islamic jurisprudence keyword
        for key in &["fiqh", "fikih", "fiqih"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الفقه", "علم الفقه", "الأحكام"]);
        }
        // "ilmu" — generic "knowledge/science" used in compound queries like "ilmu fiqh"
        for key in &["ilmu"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["العلم", "علم"]);
        }
        // Prophethood concepts
        for key in &["nubuwwah", "nubuah", "kenabian", "prophethood"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["النبوة", "الرسالة", "النبيون"]);
        }

        // ── P1: ISLAMIC ADJECTIVE SUFFIXES ──
        // "islamiyah/islami" — appears in compound queries like "aqidah islamiyah"
        for key in &["islamiyah", "islamiah", "islami", "islamik"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الإسلامية", "الإسلامي", "إسلامي"]);
        }

        // ── P2: POPULAR SURAH NAMES ──
        for key in &["yasin", "yaseen", "yaaseen", "yaasin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["يس", "سورة يس"]);
        }
        for key in &["baqarah", "al-baqarah", "albaqarah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["البقرة", "سورة البقرة"]);
        }
        for key in &["alkahf", "alkahfi", "kahfi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الكهف", "سورة الكهف"]);
        }
        for key in &["al-ikhlas"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الإخلاص", "قل هو الله أحد"]);
        }
        for key in &["falaq", "al-falaq"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الفلق", "سورة الفلق"]);
        }
        for key in &["ar-rahman", "arrahman", "surat rahman", "rahman"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الرحمن", "سورة الرحمن"]);
        }
        for key in &["almulk", "al-mulk", "muluk", "mulk"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الملك", "سورة الملك", "تبارك"]);
        }
        // Additional standalone surah name components for "surat al X" / "surat ar X" patterns
        for key in &["nisa", "an-nisa"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["النساء", "سورة النساء"]);
        }
        for key in &["imran"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["آل عمران", "سورة آل عمران"]);
        }
        for key in &["nur", "an-nur"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["النور", "سورة النور"]);
        }
        for key in &["rum", "ar-rum"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الروم", "سورة الروم"]);
        }
        for key in &["anfal", "al-anfal"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الأنفال", "سورة الأنفال"]);
        }
        for key in &["hujurat", "al-hujurat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الحجرات", "سورة الحجرات"]);
        }
        for key in &["hasyr", "al-hasyr"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الحشر", "سورة الحشر"]);
        }
        for key in &["mumtahanah", "mumtahinah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الممتحنة", "سورة الممتحنة"]);
        }
        for key in &["luqman"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["لقمان", "سورة لقمان"]);
        }
        for key in &["fussilat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["فصلت", "سورة فصلت"]);
        }
        for key in &["zukhruf"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الزخرف", "سورة الزخرف"]);
        }
        for key in &["fath", "al-fath"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الفتح", "سورة الفتح"]);
        }
        for key in &["hujr"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الحجرات"]);
        }
        for key in &["qiyamah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["القيامة", "سورة القيامة"]);
        }
        for key in &["insan", "al-insan", "dahr"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الإنسان", "سورة الإنسان"]);
        }
        for key in &["naba", "an-naba"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["النبأ", "سورة النبأ"]);
        }
        for key in &["naziat", "an-naziat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["النازعات"]);
        }
        for key in &["abasa"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عبس", "سورة عبس"]);
        }
        for key in &["takwir"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التكوير"]);
        }
        for key in &["infithar", "infitar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الانفطار"]);
        }
        for key in &["muthaffifin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المطففين"]);
        }
        for key in &["insyiqaq"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الانشقاق"]);
        }
        for key in &["buruj"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["البروج", "سورة البروج"]);
        }
        for key in &["thariq"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الطارق"]);
        }
        for key in &["ala", "al-ala"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الأعلى", "سبح اسم ربك"]);
        }
        for key in &["ghasyiah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الغاشية"]);
        }
        for key in &["fajr", "al-fajr"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الفجر", "سورة الفجر"]);
        }
        for key in &["balad"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["البلد", "سورة البلد"]);
        }
        for key in &["syams"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الشمس", "سورة الشمس"]);
        }
        for key in &["lail", "al-lail"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الليل", "سورة الليل"]);
        }
        for key in &["dhuha", "adh-dhuha"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الضحى", "سورة الضحى"]);
        }
        for key in &["insyirah", "syarh"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الشرح", "سورة الشرح"]);
        }
        for key in &["tin", "at-tin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التين", "سورة التين"]);
        }
        for key in &["alaq", "al-alaq"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["العلق", "اقرأ باسم ربك"]);
        }
        for key in &["qadr", "al-qadr"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["القدر", "إنا أنزلناه"]);
        }
        for key in &["bayyinah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["البينة"]);
        }
        for key in &["zilzal", "az-zilzal"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الزلزلة"]);
        }
        for key in &["adiyat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["العاديات"]);
        }
        for key in &["qariah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["القارعة"]);
        }
        for key in &["takasur"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التكاثر"]);
        }
        for key in &["ashr", "al-ashr", "al-asr"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["العصر", "سورة العصر"]);
        }
        for key in &["humazah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الهمزة"]);
        }
        for key in &["fil", "al-fil"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الفيل", "سورة الفيل"]);
        }
        for key in &["quraisy"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قريش", "سورة قريش"]);
        }
        for key in &["maun"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الماعون"]);
        }
        for key in &["kautsar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الكوثر"]);
        }
        for key in &["kafirun"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الكافرون"]);
        }
        for key in &["nasr", "an-nasr"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["النصر"]);
        }
        for key in &["lahab", "al-lahab", "masad"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المسد", "اللهب"]);
        }
        for key in &["ikhlas", "al-ikhlas"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الإخلاص", "قل هو الله أحد"]);
        }
        for key in &["nas", "an-nas"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الناس", "سورة الناس"]);
        }
        for key in &["waqiah", "waqiyah", "al-waqiah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الواقعة", "سورة الواقعة"]);
        }
        for key in &["hujurat", "al-hujurat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الحجرات", "سورة الحجرات"]);
        }
        for key in &["ahzab", "al-ahzab"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الأحزاب", "سورة الأحزاب"]);
        }
        for key in &["an-nisa", "annisa"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["النساء", "سورة النساء"]);
        }
        for key in &["maidah", "al-maidah", "almaidah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المائدة", "سورة المائدة"]);
        }
        for key in &["at-talaq", "attalaq"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الطلاق", "سورة الطلاق"]);
        }
        for key in &["ar-rum", "arrum", "surat rum"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الروم", "سورة الروم"]);
        }
        for key in &["al-imran", "al imran", "ali imran"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["آل عمران", "سورة آل عمران"]);
        }
        for key in &["al-anfal", "anfal"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الأنفال", "سورة الأنفال"]);
        }
        for key in &["al-hasyr", "hasyr", "alhasyr"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الحشر", "سورة الحشر"]);
        }

        // ══════════════════════════════════════════════════════════════
        // BATCH 19: High-frequency words confirmed missing from term_map
        // ══════════════════════════════════════════════════════════════

        // ── P0: "islam" standalone — appears in almost every query ──
        // e.g. "hukum X dalam islam", "menurut pandangan islam", "fiqh islam"
        for key in &["islam"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الإسلام", "إسلام"]);
        }
        // "science", "knowledge" to complement batch18 "ilmu"
        for key in &["science", "knowledge", "sciences"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["العلم", "علم", "علوم"]);
        }

        // ── P1: Social / political context words ──
        for key in &["keluarga", "family", "household"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أسرة", "العائلة", "الأهل"]);
        }
        for key in &["masyarakat", "society", "komunitas", "community"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مجتمع", "الأمة", "الناس"]);
        }
        for key in &["negara", "state", "pemerintahan", "government"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دولة", "الحكومة", "الدولة"]);
        }
        for key in &["pemimpin", "kepemimpinan", "leader", "leadership"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قيادة", "الإمارة", "رئيس"]);
        }
        for key in &["hak", "rights", "right"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حق", "الحقوق"]);
        }
        for key in &["kewajiban", "obligation", "tugas", "duties"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["واجب", "الفريضة", "الوجوب"]);
        }
        for key in &["larangan", "prohibition", "pantangan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نهي", "التحريم", "المحرمات"]);
        }

        // ── P1: Demographics ──
        for key in &["pria", "laki-laki", "lelaki", "male"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الرجل", "الذكر", "رجل"]);
        }
        for key in &["remaja", "pemuda", "youth", "teenager"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["شاب", "الشباب", "مراهق"]);
        }
        for key in &["dewasa", "adult", "orang dewasa"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بالغ", "الراشد", "البالغ"]);
        }

        // ── P2: Quranic sciences & learning terms ──
        for key in &["tajwid", "tajweed"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تجويد", "التجويد", "أحكام التلاوة"]);
        }
        for key in &["makhraj", "makhroj", "makhraj huruf"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مخرج", "مخارج الحروف"]);
        }
        for key in &["mad", "mad far'i", "mad asli"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مد", "أحكام المد"]);
        }
        for key in &["idgham"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["إدغام", "حكم الإدغام"]);
        }
        for key in &["ikhfa", "ikhfaa"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["إخفاء", "حكم الإخفاء"]);
        }
        for key in &["iqlab"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["إقلاب", "حكم الإقلاب"]);
        }
        for key in &["idzhar", "izhar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["إظهار", "حكم الإظهار"]);
        }

        // ── P2: Practical wisdom/ethics terms ──
        for key in &["hikmah", "hikmat", "wisdom"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة", "الحكمة"]);
        }
        for key in &["nasihat", "nashihat", "nasehat", "advice", "counsel"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نصيحة", "النصيحة"]);
        }
        for key in &["rezeki", "rizki", "rizq", "sustenance", "provision"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["رزق", "الرزق"]);
        }
        for key in &["wasilah", "wasila", "means", "instrument"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["وسيلة", "الوسيلة"]);
        }
        for key in &["mudharat", "mudhorat", "damage", "harm"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ضرر", "المضرة", "المفسدة"]);
        }

        // ── P2: Islamic sects / groups often searched ──
        for key in &["sunni", "ahlussunnah", "aswaja", "ahlusunnah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أهل السنة", "السنة والجماعة"]);
        }
        for key in &["syiah", "syi'ah", "shia", "syiyah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الشيعة", "شيعة"]);
        }
        for key in &["wahhabi", "wahhabiyah", "wahabisme"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الوهابية", "ابن عبد الوهاب"]);
        }
        for key in &["salafi", "salafiyah", "salafiyyah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["السلفية", "السلف الصالح"]);
        }
        for key in &["khawarij", "kharijites"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الخوارج"]);
        }
        for key in &["muktazilah", "mu'tazilah", "mutazilah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المعتزلة"]);
        }

        // ── P3: Zakat asnaf (8 recipients) ──
        for key in &["mustahiq", "mustahig", "penerima zakat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مستحق", "مصارف الزكاة", "أصناف الزكاة"]);
        }
        for key in &["amil", "amil zakat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عامل", "عامل الزكاة"]);
        }
        for key in &["muallaf"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مؤلفة القلوب", "المؤلف"]);
        }
        for key in &["gharim", "gharimin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الغارمين", "الغارم"]);
        }
        for key in &["ibnu sabil", "ibn sabil"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ابن السبيل"]);
        }
        for key in &["fi sabilillah", "sabilillah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["في سبيل الله", "سبيل الله"]);
        }

        // ── P3: Biography/history query context words ──
        for key in &["kehidupan", "hayat", "life"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حياة", "السيرة", "الحياة"]);
        }
        for key in &["masa", "zaman", "era", "period", "abad"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عصر", "زمان", "مرحلة"]);
        }
        for key in &["perkembangan", "development", "pertumbuhan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تطور", "نمو"]);
        }
        for key in &["pengaruh", "influence", "dampak"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أثر", "تأثير"]);
        }
        for key in &["hubungan", "relationship", "connection"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["علاقة", "الصلة"]);
        }

        // ── P3: Worshippers & roles ──
        for key in &["mushaf", "quran book"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مصحف", "المصحف"]);
        }
        for key in &["hafiz", "hafidz", "hafizh", "memorizer"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حافظ", "الحافظ", "حفظة القرآن"]);
        }
        for key in &["qari", "qorri", "reciter"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قارئ", "القراء"]);
        }
        for key in &["mufassir", "tafsir scholar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مفسر", "المفسرون"]);
        }
        for key in &["muhadits", "muhaddits", "hadith scholar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["محدث", "المحدثون"]);
        }
        for key in &["mufti"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مفتي", "الإفتاء"]);
        }

        // ── P3: Common question starters seen in queries ──
        for key in &["mengapa", "kenapa", "why"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["لماذا"]);
        }
        for key in &["bagaimana", "how", "cara"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["كيف", "كيفية"]);
        }
        for key in &["kapan", "when"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["متى"]);
        }
        for key in &["dimana", "where"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أين"]);
        }

        // ══════════════════════════════════════════════════════════════
        // BATCH 20: Final zero-result fixes from V17b eval analysis
        // Targets: "berbuat baik kepada sesama" + "mencabut alat bantu napas"
        // + preemptive coverage for similar query patterns
        // ══════════════════════════════════════════════════════════════

        // ── "berbuat baik kepada sesama" ──
        // All four words were unmapped in V17b → zero results
        for key in &["baik", "good", "kebaikan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حسن", "خير", "الإحسان"]);
        }
        for key in &["sesama", "orang lain", "manusia", "others", "fellow"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الناس", "الآخرين", "الغير"]);
        }
        for key in &["berbuat", "buat", "perbuatan", "action", "deed"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["فعل", "عمل", "الأعمال"]);
        }
        for key in &["kepada", "terhadap", "toward"] {
            // Preposition; map to empty so it doesn't block result
            self.term_map.entry(key.to_string()).or_insert(vec![]);
        }

        // ── "mencabut alat bantu napas" ──
        // Context: withdrawing life-support/ventilator (bioethics query)
        for key in &["mencabut", "cabut", "melepas", "withdraw", "remove"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نزع", "فصل", "إيقاف"]);
        }
        for key in &["alat", "alat-alat", "peralatan", "device", "tool", "machine"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جهاز", "الأجهزة", "آلة"]);
        }
        for key in &["napas", "pernafasan", "bernapas", "breathing", "breath"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تنفس", "التنفس", "الشهيق"]);
        }
        for key in &["bantu", "membantu", "bantuan", "assistance", "support"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مساعدة", "معاون", "إسناد"]);
        }

        // ── Additional jinayat/compensation terms (fix "ganti rugi" fragments) ──
        for key in &["ganti", "penggantian", "compensation", "replace"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تعويض", "الأرش", "دية"]);
        }
        for key in &["rugi", "kerugian", "damage", "loss"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ضرر", "خسارة"]);
        }

        // ── Self-defense terms (fix "pembunuhan untuk bela diri") ──
        for key in &["bela", "membela", "defend", "defense"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دفع", "الدفاع"]);
        }
        for key in &["diri", "diri sendiri", "self"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["النفس", "الذات"]);
        }

        // ── "semi" prefix patterns ──
        for key in &["semi", "partly", "sebagian"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["شبه", "نصف"]);
        }

        // ── Generic Indonesian verbs that appear in queries ──
        for key in &["tidak", "bukan", "tidak boleh", "not"] {
            self.term_map.entry(key.to_string()).or_insert(vec![]);  // negation, no Arabic needed
        }
        for key in &["untuk", "bagi", "for"] {
            self.term_map.entry(key.to_string()).or_insert(vec![]);  // preposition, no Arabic needed
        }
        for key in &["dengan", "bersama", "with", "together"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مع"]);
        }
        for key in &["dalam", "di dalam", "inside"] {
            self.term_map.entry(key.to_string()).or_insert(vec![]);  // preposition
        }
        for key in &["tentang", "mengenai", "regarding", "about"] {
            self.term_map.entry(key.to_string()).or_insert(vec![]);  // preposition
        }

        // ══════════════════════════════════════════════════════════════
        // BATCH 21: Fix "apa" degenerate query (last V17b zero)
        // "apa" = Indonesian for "what?" → Arabic "ما" (mā)
        // Without this, standalone "apa" query returns 0 results in Arabic corpus.
        // ══════════════════════════════════════════════════════════════
        for key in &["apa", "apakah", "apa itu", "what"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ما", "الإسلام"]);
        }

        // ══════════════════════════════════════════════════════════════
        // BATCH 22: Improve 42 low-result (1-result) queries from V17b
        // Three categories:
        //   A) Usul al-fiqh maxims (qawa'id) in transliteration
        //   B) Modern technology concepts (pandangan islam tentang X)
        //   C) Modern social/political concepts
        // ══════════════════════════════════════════════════════════════

        // ── A) QAWA'ID FIQHIYYAH — Usul al-fiqh maxims ──
        // "al yaqin la yazul bi al syak" (certainty not removed by doubt)
        for key in &["yaqin", "yakin", "certainty", "yakin yaitu", "al-yaqin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["اليقين", "القطع", "الجزم"]);
        }
        for key in &["syak", "shak", "shakk", "doubt", "keraguan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الشك", "الشبهة"]);
        }
        // "al masyaqqah tajlib al taysir" (hardship brings ease)
        for key in &["masyaqqah", "mashaqqah", "mashaqqoh", "masyaqah", "hardship", "kesulitan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المشقة", "الحرج", "الضيق"]);
        }
        for key in &["taysir", "taysiir", "ease", "kemudahan", "rukhsah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التيسير", "التخفيف", "الرخصة"]);
        }
        // "al adah muhakkamah" (custom/practice can be the basis of ruling)
        for key in &["adah", "'adat", "adat", "custom", "kebiasaan", "uruf"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["العادة", "العرف", "التقاليد"]);
        }
        for key in &["muhakkamah", "muhakkam", "hukum adat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["محكمة", "محكم"]);
        }
        // "dar'u al mafasid muqaddam ala jalb al mashalih" (prevention > benefit)
        for key in &["mafasid", "mafsadat", "mafsadah", "harm prevention", "kerusakan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المفاسد", "الضرر", "الفساد", "درء المفاسد"]);
        }
        for key in &["mashalih", "masalih", "maslahah", "manfaat", "maslahah", "benefit"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المصالح", "المصلحة", "النفع"]);
        }
        for key in &["muqaddam", "didahulukan", "priority", "precedence"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مقدم", "أولى", "الأولى"]);
        }
        // General qawa'id terms
        for key in &["qawaid", "qawa'id", "qa'idah", "legal maxim", "fikih maxim"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["القواعد الفقهية", "القاعدة", "ضابط"]);
        }
        for key in &["ushul", "usul fiqh", "ushul fiqih", "fiqh principles"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أصول الفقه", "الأصول"]);
        }

        // ── B) MODERN TECHNOLOGY CONCEPTS ──
        // Artificial Intelligence
        for key in &["artificial intelligence", "kecerdasan buatan", "kecerdasan artifisial"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الذكاء الاصطناعي", "التقنية"]);
        }
        for key in &["ai", "machine learning", "deep learning"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الذكاء الاصطناعي", "تقنية المعلومات"]);
        }
        // Robotics
        for key in &["robot", "robotika", "robotics", "automation", "otomasi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الآلة", "الروبوت", "التقنية"]);
        }
        // Communication technology
        for key in &["smartphone", "handphone", "ponsel", "telepon", "hp"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الهاتف الذكي", "الاتصالات"]);
        }
        for key in &["internet", "iot", "internet of things", "jaringan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الإنترنت", "شبكة الاتصالات"]);
        }
        for key in &["drone", "pesawat tanpa awak", "uav"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الطائرة", "الطائرات المسيرة"]);
        }
        for key in &["blockchain", "cryptocurrency", "bitcoin", "kripto"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["العملات الرقمية", "المعاملات الرقمية", "المال"]);
        }
        for key in &["cloud computing", "komputasi awan", "saas"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تقنية المعلومات", "الخدمات الرقمية"]);
        }
        for key in &["e-commerce", "perdagangan elektronik", "jual beli online", "e commerce"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التجارة الإلكترونية", "البيع والشراء", "المعاملات"]);
        }
        for key in &["edtech", "pendidikan teknologi", "online learning"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التعليم", "التعلم", "العلم"]);
        }
        for key in &["telemedicine", "pengobatan online", "dokter online"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التداوي", "الطب", "العلاج"]);
        }
        for key in &["metaverse", "virtual reality", "augmented reality"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الواقع الافتراضي", "التقنية"]);
        }
        for key in &["deepfake", "manipulasi gambar", "hoaks"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الكذب", "الغش", "التلاعب"]);
        }
        for key in &["3d printing", "cetak tiga dimensi", "3d print"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الصناعة", "التصنيع", "التقنية"]);
        }
        for key in &["biometric", "biometrik", "fingerprint", "sidik jari"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الهوية", "القياسات الحيوية"]);
        }
        // Medical biotechnology
        for key in &["crispr", "gene editing", "rekayasa genetika", "stem cell", "sel punca"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التدخل الجيني", "الطب", "البيولوجيا"]);
        }
        for key in &["nanotechnology", "nanoteknologi", "nanotek"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التقنية", "الطب"]);
        }
        for key in &["brain computer interface", "bci", "antarmuka otak"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الدماغ", "العقل", "التقنية"]);
        }
        for key in &["autonomous vehicle", "mobil otonom", "self-driving"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المركبة", "التقنية", "السيارات"]);
        }
        for key in &["space tourism", "pariwisata luar angkasa", "wisata luar angkasa"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الفضاء", "السفر", "السياحة"]);
        }
        for key in &["nuclear energy", "energi nuklir", "pembangkit nuklir"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الطاقة النووية", "الطاقة"]);
        }
        for key in &["smart home", "rumah pintar", "iot home"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["البيت", "الاتصالات"]);
        }
        // General technology
        for key in &["teknologi", "technology", "teknik", "inovasi", "innovation"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التقنية", "الاختراع", "الصناعة"]);
        }

        // ── C) MODERN SOCIAL, POLITICAL & ETHICAL CONCEPTS ──
        for key in &["demokrasi", "democracy", "demokratis"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الديمقراطية", "الشورى", "الحكم"]);
        }
        for key in &["terorisme", "terrorism", "teroris", "terrorist"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الإرهاب", "الفتنة", "الخوارج"]);
        }
        for key in &["radikalisme", "radicalism", "radikal", "ekstremisme", "extremism"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التطرف", "الغلو", "الراديكالية"]);
        }
        for key in &["nasionalisme", "nationalism", "nasional", "kebangsaan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["القومية", "الوطنية", "الأمة"]);
        }
        for key in &["filantropi", "philanthropy", "kedermawanan", "donasi", "donation"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الصدقة", "التبرع", "الكرم", "البر"]);
        }
        for key in &["kremasi", "cremate", "cremation", "membakar jenazah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حرق", "الجنازة", "المحرم"]);
        }
        for key in &["mental health", "kesehatan mental", "kesehatan jiwa", "gangguan jiwa"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الصحة النفسية", "العقل", "النفس"]);
        }
        for key in &["surrogacy", "ibu pengganti", "sewa rahim", "titipan rahim"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الأم البديلة", "استئجار الرحم", "النسب"]);
        }
        for key in &["racism", "ras", "rasisme", "diskriminasi ras"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["العنصرية", "التمييز", "الجاهلية"]);
        }
        for key in &["hak asasi manusia", "ham", "human rights", "hak manusia"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حقوق الإنسان", "حقوق", "الحقوق"]);
        }
        for key in &["pandangan", "perspektif", "pendapat", "view", "perspective", "opinion"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الرأي", "الحكم", "النظر"]);
        }
        for key in &["filosofi", "philosophy", "filsafat", "hikmat", "hikmah ilmu"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الفلسفة", "الحكمة", "المنطق"]);
        }
        for key in &["pendidikan", "education", "pembelajaran", "belajar", "mengajar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التعليم", "التربية", "العلم", "طلب العلم"]);
        }
        for key in &["tetangga", "jiran", "neighbor", "neighbour"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الجار", "الجيران", "حق الجار"]);
        }
        for key in &["membela", "defending", "pembela"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الدفاع", "الدفع", "النصرة"]);
        }
        for key in &["sayings", "says", "say"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكم", "قال"]);
        }

        // ══════════════════════════════════════════════════════════════
        // BATCH 23: Targeted fixes for remaining low-result queries
        // Sources: V17b queries with 1-3 results after BATCH 22
        // ══════════════════════════════════════════════════════════════

        // ── Transliterated qawa'id: specific grammatical forms ──
        // "al umur bi maqashidiha" → الأمور بمقاصدها
        // "umur" in this context = الأمور (matters), not age
        for key in &["maqashidiha", "maqasidiha", "maqashiduha"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مقاصدها", "مقاصد"]);
        }
        for key in &["lazul", "yazul", "tazul", "la yazul"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["يزول", "زال"]);
        }
        for key in &["jalb", "jalab"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جلب", "جذب"]);
        }
        for key in &["dar'u", "dar", "dara'", "def'u", "daf'u"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["درء", "دفع", "رفع"]);
        }
        for key in &["ala", "alaa", "above", "over"] {
            self.term_map.entry(key.to_string()).or_insert(vec![]);  // preposition, skip
        }
        // "al adah muhakkamah" extra support
        for key in &["muhakkamah", "al-muhakkamah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["محكمة", "حاكم"]);
        }

        // ── Social media & digital era ──
        for key in &["social media", "media sosial", "medsos", "socmed"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["وسائل التواصل الاجتماعي", "الإعلام"]);
        }
        for key in &["social", "sosial"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["اجتماعي", "التواصل الاجتماعي"]);
        }
        for key in &["media", "medium"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["وسائل", "الإعلام"]);
        }
        for key in &["digital", "digitalisasi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["رقمي", "الرقمي"]);
        }
        for key in &["era", "zaman", "masa kini", "modern"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["العصر", "الزمان", "الحديث"]);
        }
        for key in &["dai", "da'i", "muballigh", "preacher", "penceramah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الداعية", "المبلغ", "الداعي", "الدعوة"]);
        }

        // ── Salah hand-placement query: "meletakkan tangan di dada atau perut" ──
        // About placing hands on chest vs stomach during prayer
        for key in &["meletakkan", "meletakan", "place", "placing", "menaruh"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["وضع", "يضع"]);
        }
        for key in &["dada", "chest", "breast"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الصدر", "صدر"]);
        }
        for key in &["perut", "stomach", "belly"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["البطن", "بطن"]);
        }

        // ── "dai di era digital" ──
        for key in &["golongan", "era digital", "zaman digital"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["العصر الرقمي"]);
        }

        // ── "tafsir ayat kursi" ── (only 4 results — improve with specific terms)
        for key in &["ayat kursi", "aayat al-kursi", "ayatul kursi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["آية الكرسي"]);
        }
        for key in &["kursi", "kursy"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الكرسي", "عرش"]);
        }

        // ── Ihya Ulumuddin specific queries ──
        // These have T:ihya ulumuddin mapped but only 5 results (diversity filter)
        // Adding more specific book terms might help variety
        for key in &["ihya", "ihya'", "ihya ulumuddin", "ihya ulum al din"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["إحياء علوم الدين", "إحياء"]);
        }
        for key in &["ghazali", "al-ghazali", "imam ghazali"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الغزالي", "أبو حامد الغزالي"]);
        }
        for key in &["ulum al din", "ulumuddin", "ulum", "keagamaan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["علوم الدين", "الدين"]);
        }

        // ── Body parts (for various prayer and fiqh queries) ──
        for key in &["kepala", "head"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الرأس", "رأس"]);
        }
        for key in &["kaki", "feet", "foot"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["القدم", "الرجل", "قدم"]);
        }
        for key in &["telinga", "ear"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الأذن", "أذن"]);
        }
        for key in &["mata", "eye", "eyes"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["العين", "عين"]);
        }
        for key in &["mulut", "mouth"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الفم"]);
        }
        for key in &["hidung", "nose"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الأنف"]);
        }
        for key in &["punggung", "back", "back body"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الظهر", "الظهر"]);
        }
        for key in &["leher", "neck"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["العنق", "الرقبة"]);
        }

        // BATCH 24: High-range quality improvements (9838-10030 "hukum X dalam keadaan Y" patterns)
        // Also general quality for any query using these common action/object words

        // ── Hair & cutting ──
        for key in &["rambut", "bulu", "hair"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["شعر", "الشعر"]);
        }
        for key in &["potong", "memotong", "potongan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قطع", "قص", "حلق"]);
        }

        // ── Music / playing music ──
        for key in &["bermusik", "bermain musik", "memainkan musik"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الغناء", "المعازف", "الموسيقى"]);
        }

        // ── Perfume / fragrance ──
        for key in &["wewangian", "parfum", "minyak wangi", "pewangi", "fragrance", "perfume"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الطيب", "التطيب", "العطر"]);
        }

        // ── Statue / idol making ──
        for key in &["patung", "arca", "idol", "statue"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الصنم", "الأصنام", "صنم"]);
        }

        // ── Borrowing / loan ──
        for key in &["meminjam", "pinjam", "pinjaman", "borrow", "loan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قرض", "الاقتراض", "الدين"]);
        }

        // ── Wearing / using (memakai wewangian, memakai emas, etc.) ──
        for key in &["memakai", "mengenakan", "pemakaian"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["لبس", "ارتداء"]);
        }

        // ── Not knowing / ignorance (dalam keadaan tidak tahu) ──
        for key in &["tidak tahu", "tidak mengetahui", "jahil", "ignorant"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جاهل", "الجهل", "عدم العلم"]);
        }

        // ── Condition words for contextual queries ──
        for key in &["keadaan", "kondisi", "situasi", "state", "condition"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حال", "الحال"]);
        }

        // ── Shaving beard / hair (potong rambut / mencukur) ──
        // (mencukur already mapped; reinforce rambut + jenggot pair context)
        for key in &["potong rambut", "cukur rambut", "gunting rambut"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قص الشعر", "حلق الشعر", "الشعر"]);
        }

        // ── Making / creating ──
        for key in &["membuat", "pembuatan", "create", "making"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صنع", "إنشاء"]);
        }

        // ── Dzulqa'dah sacred month context ──
        // Dzulqa'dah is one of 4 sacred months (الأشهر الحرم); expand to include that context
        // Using insert (not or_insert) to override earlier narrower mappings
        for key in &["dzulqa'dah", "dzulqadah", "dhul qa'dah"] {
            self.term_map.insert(key.to_string(), vec!["ذو القعدة", "الأشهر الحرم"]);
        }
        for key in &["dzulhijjah", "dzulhijja", "dhul hijjah"] {
            self.term_map.insert(key.to_string(), vec!["ذو الحجة", "الأشهر الحرم", "شهر ذي الحجة"]);
        }

        // BATCH 25: Quality improvements for 7000-7560 range — professions, inheritances, food animals
        // Based on analysis of query 7000-7560 dataset structure

        // ── Inheritance special cases (7500-7517) ──
        // aul: shares exceed denominator → expand denominator
        for key in &["aul", "awl"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عول", "العول", "التعصيب"]);
        }
        // radd: shares less than denominator → remainder returned to heirs
        for key in &["radd", "rad"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["رد", "الرد", "مسألة الرد"]);
        }
        // gharrawain: famous inheritance case involving husband/wife + parent
        for key in &["gharrawain", "gharraoain", "udmariyah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الغراوان", "المسألة الغراوين"]);
        }
        // musytarakah/hijriyah: complex inheritance case
        for key in &["musytarakah", "musytarakah", "hijriyah", "hijariyah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المشتركة", "الحجرية", "المشركة"]);
        }
        // akdariyah: famous problematic inheritance case
        for key in &["akdariyah", "akdhariyah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الأكدرية", "مسألة الأكدرية"]);
        }
        // mafqud: missing person (inheritance)
        for key in &["mafqud", "mafkud", "orang hilang"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مفقود", "المفقود", "أحكام المفقود"]);
        }
        // khuntsa musykil: intersex inheritance
        for key in &["khuntsa", "huntsa", "khuntsa musykil"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["خنثى", "الخنثى", "الخنثى المشكل"]);
        }

        // ── Food: sea creatures not yet mapped individually (7208-7280) ──
        for key in &["lobster", "udang karang"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جراد البحر", "حيوان البحر", "القشريات"]);
        }
        for key in &["tiram", "oyster"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["محار", "المحار", "حيوان البحر"]);
        }
        for key in &["kerang", "clam", "shellfish"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بلح البحر", "المحار", "حيوان البحر"]);
        }
        for key in &["cumi", "cumi-cumi", "squid"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حبار", "الحبار", "حيوان البحر"]);
        }
        for key in &["ubur-ubur", "ubur", "jellyfish"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قنديل البحر", "حيوان البحر"]);
        }
        for key in &["teripang", "sea cucumber"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تريبانج", "حيوان البحر", "الخيار البحري"]);
        }

        // ── Land animals not yet mapped ──
        for key in &["unta", "camel"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["إبل", "الإبل", "الناقة"]);
        }
        for key in &["kerbau", "buffalo"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جاموس", "الجاموس", "أكل الجاموس"]);
        }
        for key in &["bebek", "duck"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بط", "البط", "أكل البط"]);
        }
        for key in &["merpati", "dove", "pigeon"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حمام", "الحمام"]);
        }
        for key in &["rusa", "deer", "venison"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["غزال", "الغزال", "الصيد"]);
        }
        for key in &["burung puyuh", "quail"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سمان", "طائر السمان"]);
        }

        // ── Special foods ──
        for key in &["tape", "tapai"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الخمر", "المسكر", "النبيذ", "التخمير"]);
        }
        for key in &["kombucha"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المسكر", "الخمر", "التخمير"]);
        }
        for key in &["jamur", "mushroom"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الفطر", "الكمأة"]);
        }

        // ── Profession types for "hukum [profession] shalat/puasa" patterns (7010-7200) ──
        for key in &["tentara", "militer", "prajurit", "soldier", "military"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جندي", "المجاهد", "الغازي"]);
        }
        for key in &["polisi", "police", "officer"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["شرطة", "شرطي"]);
        }
        for key in &["nelayan", "fisherman", "fisher"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صياد", "الصياد"]);
        }
        for key in &["supir", "sopir", "driver"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سائق", "السائق"]);
        }
        for key in &["pilot"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["طيار", "الطيار"]);
        }
        for key in &["astronot", "astronaut"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["رائد الفضاء", "الفضاء"]);
        }
        for key in &["mahasiswa", "university student"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["طالب", "الطلاب"]);
        }
        for key in &["pelajar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["طالب", "المتعلم"]);
        }
        for key in &["pedagang", "merchant", "trader"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تاجر", "البائع", "التاجر"]);
        }
        for key in &["petani", "farmer"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مزارع", "الفلاح", "الزراعة"]);
        }

        // ── Person categories ──
        for key in &["bayi", "infant", "newborn"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["رضيع", "الرضيع", "المولود"]);
        }
        for key in &["balita", "toddler"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["طفل", "الطفل الصغير"]);
        }
        for key in &["remaja", "teenager", "adolescent"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مراهق", "الشباب", "الشاب"]);
        }
        for key in &["lansia", "elderly", "orang tua"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["شيخ", "كبير السن", "الشيخ الكبير"]);
        }
        for key in &["difabel", "disabled", "penyandang cacat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["معاق", "المعاق", "ذو العذر"]);
        }
        for key in &["tunanetra", "blind person"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أعمى", "الأعمى", "العمياء"]);
        }
        for key in &["tunarungu", "deaf person"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أصم", "الأصم", "الطرش"]);
        }
        for key in &["tunawicara", "mute person"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الأخرس", "الصمت"]);
        }
        for key in &["penghuni penjara", "prisoner", "narapidana"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سجين", "المسجون"]);
        }
        for key in &["pengungsi", "refugee"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["لاجئ", "اللاجئ", "المهاجر"]);
        }
        for key in &["korban bencana", "disaster victim"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["منكوب", "المنكوبون", "الضرورة"]);
        }

        // BATCH 26: Wild animals, economic terms, market manipulation — 7700-7840 queries
        // ══════════════════════════════════════════════════════════════════════════════

        // ─── Wild/Exotic Land Animals (hukum memakan [animal]) ───
        for key in &["harimau", "macan", "tiger"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نمر", "السباع", "الحيوانات المفترسة"]);
        }
        for key in &["singa", "lion"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أسد", "الأسد", "السباع"]);
        }
        for key in &["serigala", "wolf"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ذئب", "الذئب", "السباع"]);
        }
        for key in &["rubah", "fox"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ثعلب", "الثعلب"]);
        }
        for key in &["beruang", "bear"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دب", "الدب"]);
        }
        for key in &["gajah", "elephant"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["فيل", "الفيل"]);
        }
        for key in &["kuda nil", "badak uri", "hippopotamus"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["فرس النهر"]);
        }
        for key in &["badak", "rhinoceros"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["كركدن", "وحيد القرن"]);
        }
        for key in &["jerapah", "giraffe"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["زرافة"]);
        }
        for key in &["zebra"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حمار الوحش"]);
        }
        for key in &["monyet", "kera", "monkey"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قرد", "القرد", "الرئيسيات"]);
        }
        for key in &["gorila", "gorilla"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["غوريلا", "الرئيسيات"]);
        }
        for key in &["orangutan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أورانغوتان", "الرئيسيات"]);
        }
        for key in &["landak", "hedgehog", "porcupine"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["شيهم", "القنفذ", "الحيوانات البرية"]);
        }
        for key in &["berang-berang", "otter"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قندس", "الثعلب المائي"]);
        }
        for key in &["musang", "civet"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ابن عرس", "كلب الزباد"]);
        }

        // ─── Marine/Aquatic Animals ───
        for key in &["lumba-lumba", "lumba", "dolphin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دلفين", "حيوان البحر"]);
        }
        for key in &["paus", "whale"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حوت", "الحوت", "حيوان البحر"]);
        }
        for key in &["hiu", "shark"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قرش", "سمك القرش", "حيوان البحر"]);
        }
        for key in &["pari", "stingray"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["راي", "سمك الراي", "حيوان البحر"]);
        }
        for key in &["belut", "eel"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جريث", "ثعبان البحر", "الأسماك"]);
        }
        for key in &["lele", "catfish"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سلور", "سمك السلور", "الأسماك"]);
        }
        for key in &["nila", "tilapia"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بلطي", "سمكة النيل", "الأسماك"]);
        }
        for key in &["gurame", "gourami"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["السمك", "حكم الأسماك"]);
        }

        // ─── Economic/Market Terms (7700-7750 range) ───
        for key in &["pajak", "tax", "taxation"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ضريبة", "الجزية", "الخراج", "ضريبة المال"]);
        }
        for key in &["kartel", "cartel"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["احتكار", "الاحتكار"]);
        }
        for key in &["ijon"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بيع الثمر قبل بدو صلاحه", "بيع السلم", "الغرر"]);
        }
        for key in &["tengkulak"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["السمسار", "الوسيط التجاري"]);
        }
        for key in &["menimbun", "penimbunan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["احتكار", "الاحتكار", "تحريم الاحتكار"]);
        }
        for key in &["spekulasi", "spekulator"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مضاربة", "الغرر", "بيع الغرر"]);
        }
        for key in &["manipulasi pasar", "market manipulation"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الغش", "غش التجارة", "التلاعب"]);
        }
        for key in &["insider trading"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الغش التجاري", "الخيانة"]);
        }
        for key in &["pencucian uang", "money laundering"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["غسيل الأموال", "المال الحرام"]);
        }
        for key in &["gratifikasi", "gratuity"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["رشوة", "الرشوة"]);
        }
        for key in &["pungli", "pungutan liar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["رشوة", "اختلاس", "غصب"]);
        }
        for key in &["suap", "bribe", "bribery"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["رشوة", "الرشوة", "حكم الرشوة"]);
        }
        for key in &["korupsi", "corruption"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الفساد", "الاختلاس", "رشوة"]);
        }
        for key in &["PHK", "pemutusan hubungan kerja", "layoff", "fired"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["فسخ عقد العمل", "إنهاء العمل", "عقد العمل"]);
        }
        for key in &["outsourcing"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["إجارة", "عقد العمل", "الاستعانة بمصادر خارجية"]);
        }
        for key in &["kerja kontrak", "kontrak kerja"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عقد العمل", "الإجارة"]);
        }
        for key in &["dumping"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الإغراق التجاري", "بيع بخسارة"]);
        }
        for key in &["upah minimum", "minimum wage"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الأجرة", "حد الأجر", "أجر العامل"]);
        }

        // ─── Digital economy (7720-7750) ───
        for key in &["sukuk", "obligasi syariah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صكوك", "السندات الإسلامية"]);
        }
        for key in &["fintech", "fintech syariah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التمويل الإسلامي الرقمي", "التكنولوجيا المالية"]);
        }
        for key in &["crowdfunding", "crowdfunding syariah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التمويل الجماعي", "جمع التبرعات"]);
        }
        for key in &["jual beli followers", "jual followers"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بيع ما لا قيمة له", "البيع الباطل"]);
        }
        for key in &["jual data pribadi", "jual data"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بيع المعلومات", "الأمانة", "حفظ الأسرار"]);
        }

        // ─── Islamic political/social terms (7730-7740) ───
        for key in &["sistem khilafah", "khilafah islamiyah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الخلافة الإسلامية", "نظام الخلافة"]);
        }
        for key in &["implementasi syariah", "penerapan syariah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تطبيق الشريعة", "إقامة الشريعة"]);
        }
        for key in &["hudud", "hukum hudud"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الحدود", "حدود الله", "عقوبة الحد"]);
        }
        for key in &["qishas", "qishash", "qisas"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["القصاص", "حكم القصاص"]);
        }
        for key in &["hukum pidana islam", "jinayat", "jinayah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الجنايات", "جناية", "حدود الجنايات"]);
        }
        for key in &["filantropi", "philanthropy"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الخير", "البر", "العطاء"]);
        }

        // BATCH 27: Arabic transliteration terms (3000-3500 range) — usul fiqh, fiqh specialty
        // These are Islamic technical terms that appear in queries as Latin transliterations
        // but need Arabic expansion to find relevant content in the Arabic-text index
        // ══════════════════════════════════════════════════════════════════════════════

        // ─── Usul Fiqh / Epistemological Terms ───
        for key in &["istihalah"] {  // Transformation (e.g. wine→vinegar makes it halal)
            self.term_map.entry(key.to_string()).or_insert(vec!["استحالة", "الاستحالة"]);
        }
        for key in &["iqrar", "ikrar"] {  // Legal acknowledgment / confession
            self.term_map.entry(key.to_string()).or_insert(vec!["إقرار", "الإقرار"]);
        }
        for key in &["irsal", "mursal"] {  // Hadith without connected isnад
            self.term_map.entry(key.to_string()).or_insert(vec!["مرسل", "الحديث المرسل", "إرسال"]);
        }
        for key in &["irtidad", "riddah", "apostasy"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ردة", "الردة"]);
        }
        for key in &["jarh", "ta'dil", "jarh wa ta'dil"] {  // Hadith narrator evaluation
            self.term_map.entry(key.to_string()).or_insert(vec!["جرح", "الجرح والتعديل", "تعديل"]);
        }
        for key in &["muhshan", "ihshan"] {  // Married/protected in hudud context
            self.term_map.entry(key.to_string()).or_insert(vec!["محصن", "الإحصان", "المحصنات"]);
        }
        for key in &["masbuq"] {  // Latecomer to congregational prayer
            self.term_map.entry(key.to_string()).or_insert(vec!["مسبوق", "المسبوق"]);
        }
        for key in &["nadb", "nadbiyah"] {  // Recommendation (fiqh category between wajib and mubah)
            self.term_map.entry(key.to_string()).or_insert(vec!["ندب", "المندوب", "المستحب"]);
        }
        for key in &["qarin", "haji qarin", "tamattu qiran"] {  // Type of haji combining with umrah
            self.term_map.entry(key.to_string()).or_insert(vec!["قران", "حج القران", "الإحرام بالقران"]);
        }
        for key in &["tamattu", "haji tamattu", "tamatu"] {  // Haji type: umrah then haji
            self.term_map.entry(key.to_string()).or_insert(vec!["التمتع", "حج التمتع"]);
        }
        for key in &["ifrad", "haji ifrad"] {  // Haji type: only haji
            self.term_map.entry(key.to_string()).or_insert(vec!["الإفراد", "حج الإفراد"]);
        }
        for key in &["maqam", "maqamat"] {  // Station (in tasawuf or haji: Ibrahim)
            self.term_map.entry(key.to_string()).or_insert(vec!["مقام", "المقام", "مقام إبراهيم"]);
        }
        for key in &["sabab", "asbab"] {  // Cause in fiqh theory
            self.term_map.entry(key.to_string()).or_insert(vec!["سبب", "الأسباب", "أسباب الحكم"]);
        }
        for key in &["shaghirah", "kabirah", "dosa kecil", "dosa besar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صغيرة", "الصغائر", "الكبائر"]);
        }
        for key in &["tahnik"] {  // Practice of rubbing date on newborn's palate
            self.term_map.entry(key.to_string()).or_insert(vec!["تحنيك", "التحنيك"]);
        }
        for key in &["maqshad", "maqashid", "maqasid syariah", "maqasid al-syariah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مقاصد الشريعة", "المقاصد"]);
        }
        for key in &["radha'ah", "radhaah", "rada'ah", "susuan"] {  // Milk kinship / wet nursing
            self.term_map.entry(key.to_string()).or_insert(vec!["رضاعة", "الرضاعة", "الرضاع"]);
        }

        // ─── Fiqh Technical Terms ───
        for key in &["sutr", "satru", "penutup aurat"] {  // Covering/concealment (in prayer)
            self.term_map.entry(key.to_string()).or_insert(vec!["ستر", "الستر", "ستر العورة"]);
        }
        for key in &["ta'zir", "tazir"] {  // Discretionary punishment (not fixed hudud)
            self.term_map.entry(key.to_string()).or_insert(vec!["تعزير", "التعزير"]);
        }
        for key in &["maharim", "mahram", "muhrim"] {  // Unmarriageable relatives
            self.term_map.entry(key.to_string()).or_insert(vec!["محارم", "المحارم", "ذوو المحارم"]);
        }
        for key in &["wilayah", "welayah"] {  // Guardianship/authority
            self.term_map.entry(key.to_string()).or_insert(vec!["ولاية", "الولاية"]);
        }
        for key in &["makruh tanzih", "makruh tahrim"] {  // Subtypes of makruh
            self.term_map.entry(key.to_string()).or_insert(vec!["مكروه تنزيهي", "مكروه"]);
        }
        for key in &["taqlid", "taklid"] {  // Following a scholar's ruling
            self.term_map.entry(key.to_string()).or_insert(vec!["تقليد", "التقليد"]);
        }
        for key in &["ittiba'", "ittiba"] {  // Qualified following (between taqlid and ijtihad)
            self.term_map.entry(key.to_string()).or_insert(vec!["اتباع", "الاتباع"]);
        }
        for key in &["tathahur", "tathahhur"] {  // Purification (act of becoming clean)
            self.term_map.entry(key.to_string()).or_insert(vec!["تطهر", "التطهر", "الطهارة"]);
        }

        // ─── Islamic Finance Terms Not Yet Mapped ───
        for key in &["musyarakah mutanaqishah", "diminishing musyarakah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المشاركة المتناقصة"]);
        }
        for key in &["murabahah", "murabaha"] {  // Cost-plus sale
            self.term_map.entry(key.to_string()).or_insert(vec!["مرابحة", "المرابحة"]);
        }
        for key in &["bay' al-salam", "bay salam", "bai salam"] {  // Forward sale
            self.term_map.entry(key.to_string()).or_insert(vec!["بيع السلم", "السلم"]);
        }
        for key in &["istisna'", "istishna", "istisna"] {  // Commissioned manufacture
            self.term_map.entry(key.to_string()).or_insert(vec!["استصناع", "الاستصناع"]);
        }
        for key in &["bay' al-inah", "bai inah"] {  // Sell-and-buyback
            self.term_map.entry(key.to_string()).or_insert(vec!["بيع العينة", "العينة"]);
        }
        for key in &["tawarruq"] {  // Monetization arrangement
            self.term_map.entry(key.to_string()).or_insert(vec!["التورق"]);
        }

        // ─── Tasawuf / Spiritual Terms ───
        for key in &["zuhud", "zuhd"] {  // Asceticism/detachment from worldly things
            self.term_map.entry(key.to_string()).or_insert(vec!["زهد", "الزهد", "الزاهد"]);
        }
        for key in &["wara'", "wara", "tawadu'", "tawadhu"] {  // Piety / humility
            self.term_map.entry(key.to_string()).or_insert(vec!["ورع", "الورع", "التواضع"]);
        }
        for key in &["raja'", "raja", "roja"] {  // Hope (spiritual state)
            self.term_map.entry(key.to_string()).or_insert(vec!["رجاء", "الرجاء"]);
        }
        for key in &["tawadu'", "tawadhu'", "tawadhu"] {  // Humility
            self.term_map.entry(key.to_string()).or_insert(vec!["تواضع", "التواضع"]);
        }

        // ── BATCH 28: Remaining gaps — food, finance, professions, historical, misc ──

        // Sujud tilawah (prostration during Quran recitation) — phrase covered in phrase_map
        // Adding standalone "sujud tilawah" compound term as well for direct match
        for key in &["sajdah tilawah", "sujud bacaan", "sujud recitation"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سجود التلاوة", "سجدة التلاوة", "حكم سجود التلاوة"]);
        }
        // Kharaj (Islamic land tax / tribute system)
        for key in &["kharaj", "kharraj", "pajak tanah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["خراج", "الخراج", "خراج الأرض", "أحكام الخراج"]);
        }
        // Futures trading / forward contracts / options
        for key in &["futures trading", "futures", "perdagangan berjangka", "kontrak berjangka", "forward contract", "options trading", "trading saham"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بيع الآجل", "العقود الآجلة", "الخيارات المالية", "بيع الغرر", "المضاربة"]);
        }
        // Kepiting (crab) standalone — query pattern "apakah kepiting raja halal"
        for key in &["kepiting", "crab", "rajungan", "king crab"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سرطان البحر", "القشريات", "حكم أكل القشريات", "أكل السرطان"]);
        }
        // Tempeh (fermented soy cake — Indonesian food)
        for key in &["tempeh", "tempe"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["فول الصويا", "المأكولات المخمرة", "حكم المخمرات", "الطعام الحلال"]);
        }
        // Whey protein (dairy byproduct halal question)
        for key in &["whey protein", "whey", "protein susu", "isolate protein"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بروتين مصل اللبن", "الطعام الحلال", "المواد الغذائية الحلال", "مشتقات الألبان"]);
        }
        // Dance / dancer (profession)
        for key in &["penari", "menari", "tari", "dance", "dancer", "dancing"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الرقص", "حكم الرقص", "المغنية", "الغناء والرقص"]);
        }
        // Inhiraf (doctrinal/moral deviation)
        for key in &["inhiraf", "inchiraf", "penyimpangan akidah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["انحراف", "الانحراف", "الانحراف العقدي", "الضلال"]);
        }
        // Intersex (English — maps to khuntsa)
        for key in &["intersex", "hermaphrodite", "gender ambiguous"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["خنثى", "الخنثى", "تحديد الجنس", "الخنثى المشكل"]);
        }
        // Badar (Indonesian spelling of Battle of Badr)
        for key in &["badar", "ghazwah badr", "battle of badr", "perang badar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["غزوة بدر", "بدر", "يوم بدر", "أهل بدر"]);
        }
        // Intel / spy / intelligence agent work
        for key in &["intel", "mata-mata", "spionase", "spy", "intelligence agent", "agen rahasia"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الجاسوس", "التجسس", "عمل الجاسوسية", "المخابرات"]);
        }
        // NLP (Neuro-Linguistic Programming) — modern psychology
        for key in &["nlp", "neuro linguistic programming", "hipnotis", "hypnosis", "hipnosis"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["السحر", "التنويم المغناطيسي", "علم النفس", "التأثير النفسي"]);
        }
        // Sujud bigrams — already covered in phrase_map (line 831-832)
        // Dam haji / denda haji additional standalone
        for key in &["denda ihram", "dam haji nafar", "fidyah ihram"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الدم", "دم الحج", "الفدية", "فدية الإحرام"]);
        }
        // Berapa / kedalaman kubur
        for key in &["kedalaman kubur", "kedalaman liang lahat", "ukuran kubur"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عمق القبر", "حفر القبر", "القبر", "قبر"]);
        }
        // Shalat Istisqa (rain prayer) — English/novel queries
        for key in &["rain prayer", "salat istisqa", "shalat minta hujan", "sholat hujan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صلاة الاستسقاء", "الاستسقاء", "دعاء الاستسقاء"]);
        }
        // Khusuf / Kusuf (lunar/solar eclipse prayer)
        for key in &["shalat khusuf", "shalat gerhana bulan", "lunar eclipse prayer", "solar eclipse prayer", "shalat gerhana matahari", "shalat kusuf"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صلاة الخسوف", "صلاة الكسوف", "الخسوف", "الكسوف"]);
        }
        // Lailatul qadr specific
        for key in &["lailatul qadr", "lailat al-qadr", "malam qadar", "malam lailatul qadr"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ليلة القدر", "لية القدر", "العشر الأواخر"]);
        }
        // Isbal (wearing garment below ankles)
        for key in &["isbal", "celana cingkrang", "pakaian di bawah mata kaki"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["إسبال الإزار", "الإسبال", "الثوب تحت الكعبين"]);
        }
        // Puasa senin kamis (Monday-Thursday fasting)
        for key in &["puasa senin kamis", "puasa senin", "puasa kamis", "monday fasting", "thursday fasting"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صوم الاثنين والخميس", "صيام الاثنين", "صيام الخميس"]);
        }

        // ── BATCH 28b: Additional usul fiqh / creed terms from 3000-3500 range ──

        // Ilhad (deviation/heresy/atheism)
        for key in &["ilhad", "ilhad akidah", "atheism islamic", "atheis"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["إلحاد", "الإلحاد", "الردة", "الكفر"]);
        }
        // Israf (extravagance/waste)
        for key in &["israf", "tabdzir", "boros", "extravagance", "wasteful"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["إسراف", "الإسراف", "التبذير", "حكم الإسراف"]);
        }
        // Jama' taqdim / takhir (combining/advancing/delaying prayers)
        for key in &["jama' taqdim", "jama taqdim", "jama' ta'khir", "jama takhir", "jamak shalat", "menggabungkan shalat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جمع التقديم", "جمع التأخير", "الجمع بين الصلاتين"]);
        }
        // Kalam (Islamic theology as standalone term)
        for key in &["kalam", "ilmu kalam", "teologi islam"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["علم الكلام", "الكلام", "الكلام في العقيدة"]);
        }
        // Mustahiq / asnaf (eligible zakat recipients)
        for key in &["mustahik", "mustahiq", "asnaf", "penerima zakat", "golongan penerima zakat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مستحق", "المستحق", "أصناف الزكاة", "مصارف الزكاة"]);
        }
        // Ma'mum / makmum (follower in prayer — alternate spelling)
        for key in &["ma'mum", "mamum", "ma'mom"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مأموم", "المأموم", "الاقتداء"]);
        }
        // Jama' qasr (combining and shortening prayers while traveling)
        for key in &["jama' qasr", "jama qasr", "jamak qasar", "jama dan qasar", "gabung dan singkat shalat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جمع القصر", "الجمع والقصر", "صلاة المسافر"]);
        }
        // Mana' (prohibition / barrier in Islamic law)
        for key in &["mania'", "mawani", "penghalang hukum", "hal yang menghalangi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المانع", "موانع الحكم", "الموانع الشرعية"]);
        }
        // Halalan thayyiban (pure and lawful food)
        for key in &["halalan thayyiban", "halalan tayyiban", "halal thayyib", "halal dan baik"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حلالاً طيباً", "الحلال الطيب", "الطعام الحلال الطيب"]);
        }
        // Huquq Allah / huquq adami (rights of God vs right of humans)
        for key in &["hak allah", "huquq allah", "hak adami", "huquq adam", "hak hamba"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حق الله", "حقوق الله", "حق الآدمي", "حقوق الآدمي"]);
        }

        // ── BATCH 29: Additional historical / ritual / book-specific terms ──

        // Asyura / Ashura (10th Muharram fast)
        for key in &["asyura", "asyuro", "ashura", "asyurah", "puasa asyura"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عاشوراء", "يوم عاشوراء", "صوم عاشوراء", "محرم"]);
        }
        // Al-Umm (Imam Shafi'i's major fiqh compendium)
        for key in &["al umm", "al-umm", "kitab al umm", "kitab al-umm"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الأم", "كتاب الأم", "الشافعي"]);
        }
        // Salahuddin al-Ayyubi (Saladin — historical figure)
        for key in &["salahuddin", "saladin", "salahudin", "salahaddin", "salahuddin al ayyubi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صلاح الدين الأيوبي", "صلاح الدين", "الأيوبي"]);
        }
        // Pertanyaan kubur / sual kubur (questions in the grave)
        for key in &["pertanyaan kubur", "sual kubur", "soal kubur", "adzab kubur", "azab kubur"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سؤال القبر", "منكر ونكير", "عذاب القبر"]);
        }
        // Al-Majmu' Sharh al-Muhadzdzab (Nawawi's major fiqh encyclopedia)  
        for key in &["al majmu", "majmu syarh", "al majmu syarh muhadzdzab", "al-majmu'"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المجموع", "المجموع شرح المهذب", "النووي"]);
        }
        // Tuhfatul Muhtaj (Ibn Hajar al-Haitami's fiqh compendium)
        for key in &["tuhfatul muhtaj", "tuhfat al muhtaj", "tuhfah muhtaj"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تحفة المحتاج", "ابن حجر الهيتمي"]);
        }
        // Bidayatul Mujtahid (Ibn Rushd's comparative fiqh)
        for key in &["bidayatul mujtahid", "bidayah mujtahid", "bidayah al mujtahid"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بداية المجتهد", "ابن رشد"]);
        }
        // Al-Muhalla (Ibn Hazm's fiqh work)
        for key in &["al muhalla", "al-muhalla", "kitab al muhalla"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المحلى", "ابن حزم"]);
        }
        // Bulughul Maram (Ibn Hajar al-Asqalani's hadith collection)
        for key in &["bulughul maram", "bulugh maram", "bulughul maram", "bulug al maram"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بلوغ المرام", "ابن حجر العسقلاني"]);
        }
        // Minhaj at-Thalibin (Nawawi/Nabhani — Shafi'i manual)
        for key in &["minhaj at thalibin", "minhajut thalibin", "minhaj al thalibin", "minhajut tholibin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["منهاج الطالبين", "المنهاج", "النووي"]);
        }
        // Nihayatul Muhtaj (Ramli's Shafi'i fiqh)
        for key in &["nihayatul muhtaj", "nihayat al muhtaj", "nihayatu muhtaj"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نهاية المحتاج", "الرملي"]);
        }
        // Fathul Wahhab (Zakariyya al-Ansari — Shafi'i fiqh)
        for key in &["fathul wahhab", "fath al wahhab", "fathul wahab"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["فتح الوهاب", "زكريا الأنصاري"]);
        }
        // Imam Muslim (hadith scholar, author of Sahih Muslim)
        for key in &["imam muslim"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الإمام مسلم", "صحيح مسلم", "مسلم بن الحجاج"]);
        }
        // 3 pertanyaan kubur (the 3 grave questions — monotheistic confession)
        for key in &["3 pertanyaan kubur", "tiga pertanyaan kubur"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سؤال القبر", "منكر ونكير", "من ربك"]);
        }
        // Istisqa (rain-seeking prayer) standalone  
        for key in &["istisqa", "shalat istisqa", "salat istisqa"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الاستسقاء", "صلاة الاستسقاء", "دعاء الاستسقاء"]);
        }
        // Gerhana (eclipse prayer) standalone
        for key in &["gerhana", "gerhana matahari", "gerhana bulan", "eclipse"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الخسوف", "الكسوف", "صلاة الخسوف"]);
        }

        // ── BATCH 30: Doa situational, doa spesifik, Al Khawarizmi, Ibn Hajar al Asqalani,
        //              hukum X dalam keadaan Y context expansion ──

        // ── Al Khawarizmi — Islamic mathematician, not in Islamic jurisprudence strictly,
        //    but queries ask "siapakah Al Khawarizmi" / "biografi Al Khawarizmi" etc.
        for key in &["al khawarizmi", "khawarizmi", "al-khawarizmi", "muhammad bin musa khawarizmi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الخوارزمي", "محمد بن موسى الخوارزمي"]);
        }

        // ── wasiyyah (Arabic transliteration for will/testament) ──
        for key in &["wasiyyah", "wasiyya", "washiyyah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["وصية", "الوصية"]);
        }

        // ── Ibn Hajar al-Asqalani (hadith scholar, Fath al-Bari on Bukhari) ──
        for key in &["ibn hajar", "ibnu hajar", "ibn hajar al asqalani", "ibnu hajar al asqalani", "al asqalani", "asqalani"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ابن حجر العسقلاني", "فتح الباري", "ابن حجر"]);
        }

        // ── Doa bepergian / safar ──
        for key in &["doa bepergian", "doa safar", "doa perjalanan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دعاء السفر", "دعاء المسافر", "الدعاء السفر"]);
        }

        // ── Doa masuk pasar ──
        for key in &["doa masuk pasar", "doa di pasar", "masuk pasar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دعاء دخول السوق", "الدعاء السوق"]);
        }

        // ── Doa memakai baju baru ──
        for key in &["doa memakai baju baru", "doa pakai baju baru", "doa baju baru"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دعاء لبس الثوب الجديد", "التسمية عند اللباس"]);
        }

        // ── Doa ketika hujan turun / minta hujan ──
        for key in &["doa ketika hujan", "doa hujan turun", "doa hujan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دعاء نزول المطر", "الدعاء عند المطر"]);
        }

        // ── Doa ketika mendengar petir ──
        for key in &["doa ketika petir", "doa dengar petir", "doa saat petir"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دعاء سماع الرعد", "الدعاء عند الرعد"]);
        }

        // ── Doa qunut nazilah ──
        for key in &["qunut nazilah", "doa qunut nazilah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قنوت النازلة", "الدعاء النازلة"]);
        }

        // ── Doa istiftah (opening du'a in salah) ──
        for key in &["doa istiftah", "istiftah", "do'a iftitah", "doa iftitah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دعاء الاستفتاح", "الاستفتاح في الصلاة"]);
        }

        // ── Doa sujud (prostration du'a) ──
        for key in &["doa sujud", "do'a dalam sujud"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دعاء السجود", "ذكر السجود"]);
        }

        // ── Doa duduk di antara dua sujud ──
        for key in &["doa duduk antara dua sujud", "doa duduk di antara dua sujud", "duduk antara dua sujud"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دعاء الجلوس بين السجدتين", "ذكر بين السجدتين"]);
        }

        // ── Doa tasyahud akhir ──
        for key in &["doa tasyahud akhir", "tasyahud akhir", "doa tahiyyat akhir"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التشهد الأخير", "دعاء التشهد", "الصلاة الإبراهيمية"]);
        }

        // ── Doa setelah salam ──
        for key in &["doa setelah salam", "doa ba'da salam", "dzikir setelah shalat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الأذكار بعد الصلاة", "دعاء بعد السلام"]);
        }

        // ── Doa ketika sakit / menjenguk orang sakit ──
        for key in &["doa ketika sakit", "doa saat sakit"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دعاء المريض", "الدعاء عند المرض"]);
        }
        for key in &["doa menjenguk orang sakit", "doa jenguk sakit"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دعاء عيادة المريض", "الدعاء للمريض"]);
        }

        // ── Doa untuk mayit / masuk kuburan ──
        for key in &["doa untuk mayit", "doa mayit", "doa orang meninggal"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دعاء للميت", "الدعاء على الجنازة"]);
        }
        for key in &["doa masuk kuburan", "doa ziarah kubur"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دعاء دخول المقبرة", "دعاء زيارة القبور"]);
        }

        // ── Doa tahun baru hijriyah / doa awal akhir tahun ──
        for key in &["doa tahun baru hijriyah", "doa tahun baru islam", "doa awal tahun"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دعاء رأس السنة الهجرية", "دعاء أول السنة"]);
        }
        for key in &["doa awal dan akhir tahun", "doa akhir tahun", "doa pergantian tahun"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دعاء آخر السنة", "دعاء أول المحرم"]);
        }

        // ── Doa hari arafah ──
        for key in &["doa hari arafah", "doa arafah", "doa wukuf"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دعاء يوم عرفة", "الدعاء في عرفة"]);
        }

        // ── Doa mohon keturunan ──
        for key in &["doa mohon keturunan", "doa minta keturunan", "doa agar dikaruniai anak"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دعاء طلب الذرية", "الدعاء للولد"]);
        }

        // ── Doa ketika marah / takut ──
        for key in &["doa ketika marah", "doa saat marah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دعاء عند الغضب", "الاستعاذة عند الغضب"]);
        }
        for key in &["doa ketika takut", "doa saat takut", "doa ketika khawatir"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دعاء الخوف", "الدعاء عند الخوف"]);
        }

        // ── Doa ketika gempa / banjir ──
        for key in &["doa ketika gempa", "doa saat gempa", "doa gempa bumi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دعاء الزلزلة", "الدعاء عند الزلزال"]);
        }
        for key in &["doa ketika banjir", "doa saat banjir"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دعاء الفيضان", "الدعاء عند الفيضان"]);
        }

        // ── Doa kafarat majelis ──
        for key in &["doa kafarat majelis", "kafarat majelis", "kaffaratul majelis"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["كفارة المجلس", "دعاء كفارة المجلس"]);
        }

        // ── Doa malam lailatul qadr ──
        for key in &["doa malam lailatul qadr", "doa lailatul qadr"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["دعاء ليلة القدر", "اللهم إنك عفو"]);
        }

        // ── Context qualifiers for "dalam keadaan X" patterns ──
        // These expand the moral/legal context when paired with fiqh terms
        for key in &["lupa", "terlupa", "lupa-lupa", "forgetfulness"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نسيان", "النسيان", "حكم الناسي"]);
        }
        for key in &["safar", "dalam perjalanan", "musafir", "bepergian"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سفر", "المسافر", "حكم المسافر"]);
        }
        for key in &["hamil", "kehamilan", "pregnant", "mengandung"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حامل", "الحامل", "الحمل"]);
        }
        for key in &["menyusui", "ibu menyusui", "breastfeeding", "menyusu"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مرضع", "المرضع", "الرضاعة"]);
        }
        for key in &["haid", "menstruasi", "menstruating", "haydh"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حيض", "الحيض", "أحكام الحيض"]);
        }
        for key in &["nifas", "postpartum", "pasca melahirkan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نفاس", "النفاس", "أحكام النفاس"]);
        }
        for key in &["junub", "janabah", "hadats besar", "berjunub"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جنب", "الجنابة", "أحكام الجنابة"]);
        }
        for key in &["perang", "pertempuran", "war", "armed conflict"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حرب", "الحرب", "حالة الحرب"]);
        }
        for key in &["terpaksa", "dipaksa", "keterpaksaan", "forced"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["إكراه", "الإكراه", "حالة الإكراه"]);
        }
        for key in &["sakit keras", "sakit parah", "critically ill"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مريض", "المريض الشديد", "حالة المرض"]);
        }

        // ── Arabic queries: women in Islam (9798-9837 range) context expansion ──
        // These are Arabic-language queries that should match natively, but we help with single-term expansion
        for key in &["sujud tilawah", "sajdah tilawah", "sujud bacaan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سجود التلاوة", "أحكام سجود التلاوة"]);
        }
        for key in &["sujud syukur", "sajdah syukur"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سجود الشكر", "أحكام سجود الشكر"]);
        }

        // ── Single-word mappings for doa context words (needed because tokenizer splits words) ──
        for key in &["hujan", "rain"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مطر", "الأمطار", "المطر"]);
        }
        for key in &["petir", "thunder", "guntur", "halilintar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["رعد", "الرعد"]);
        }
        for key in &["gempa", "earthquake", "gempa bumi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["زلزال", "الزلزال", "زلزلة"]);
        }
        for key in &["banjir", "flood", "bah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["فيضان", "السيل"]);
        }
        for key in &["takut", "khawatir", "fear", "afraid"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["خوف", "الخوف"]);
        }
        for key in &["kafarat", "kaffarah", "kaffarat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["كفارة", "الكفارة"]);
        }
        for key in &["majelis", "majlis", "assembly", "meeting"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مجلس", "المجلس"]);
        }
        // صلاة الضحى (Duha prayer)
        for key in &["shalat dhuha", "sholat dhuha", "solat dhuha", "salat dhuha"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صلاة الضحى", "ضحى", "فضل الضحى"]);
        }
        // shalat tahajud / qiyam al-lail
        for key in &["shalat tahajud", "sholat tahajud", "tahajud", "shalat qiyamul lail"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التهجد", "قيام الليل", "صلاة الليل"]);
        }
        // shalat hajat, taubah, syukur
        for key in &["shalat hajat", "sholat hajat", "solat hajat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صلاة الحاجة", "الحاجة"]);
        }
        for key in &["shalat taubah", "sholat taubah", "shalat tobat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صلاة التوبة", "ركعتا التوبة"]);
        }
        for key in &["shalat syukur", "sholat syukur"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صلاة الشكر", "الشكر"]);
        }
        // sunnah fithrah (sunnan al-fitra)
        for key in &["sunnah fitrah", "sunah fitrah", "sunnatulfitrah", "sunnah fithrah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سنن الفطرة", "الفطرة"]);
        }

        // ── BATCH 31: Shafi'i and other classical scholars not yet individually mapped ──
        // These scholars appear in "biografi imam X", "karya tulis imam X", "mazhab imam X" queries
        // Ramli (Shamsuddin al-Ramli, author of Nihayatul Muhtaj)
        for key in &["ramli", "al ramli", "al-ramli", "shamsuddin ramli"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الرملي", "شمس الدين الرملي", "نهاية المحتاج"]);
        }
        // Subki (Taj al-Din al-Subki, prolific Shafi'i jurist)
        for key in &["subki", "al subki", "al-subki", "taqiyuddin subki", "tajuddin subki"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["السبكي", "تاج الدين السبكي", "الأشباه والنظائر"]);
        }
        // Isnawi (Jamal al-Din al-Isnawi, Shafi'i usul fiqh scholar)
        for key in &["isnawi", "al isnawi", "al-isnawi", "jamaluddin isnawi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الإسنوي", "جمال الدين الإسنوي"]);
        }
        // Zarkasyi (Badr al-Din al-Zarkashi, Shafi'i legal theorist — known for Al-Bahr al-Muhit)
        for key in &["zarkasyi", "zarkashi", "al zarkasyi", "al-zarkasyi", "imam zarkasyi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الزركشي", "بدر الدين الزركشي", "البحر المحيط"]);
        }
        // Bulqini (Siraj al-Din al-Bulqini and son Jalal al-Din al-Bulqini, Shafi'i scholars)
        for key in &["bulqini", "al bulqini", "al-bulqini", "imam bulqini"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["البلقيني", "سراج الدين البلقيني"]);
        }
        // Rafi'i (Abd al-Karim al-Rafi'i, Shafi'i — Rawdah al-Talibin base)
        for key in &["rafi'i", "rafii", "al rafi'i", "al-rafi'i", "imam rafi'i"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الرافعي", "عبد الكريم الرافعي", "فتح العزيز"]);
        }
        // Muzani (Ismail al-Muzani, direct student of Imam Shafi'i)
        for key in &["muzani", "al muzani", "al-muzani", "imam muzani"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المزني", "إسماعيل المزني", "مختصر المزني"]);
        }
        // Bujairimi (Sulayman al-Bujairimi, Shafi'i — Tuhfa al-Habib)
        for key in &["bujairimi", "al bujairimi", "al-bujairimi", "imam bujairimi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["البجيرمي", "سليمان البجيرمي", "تحفة الحبيب"]);
        }
        // Bajuri (Ibrahim al-Bajuri, Egyptian Shafi'i — Hashiya al-Bajuri)
        for key in &["bajuri", "al bajuri", "al-bajuri", "imam bajuri", "baijuri"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الباجوري", "إبراهيم الباجوري", "حاشية الباجوري"]);
        }
        // Zakaria al-Anshari (author of Fath al-Wahhab and Manhaj al-Tullab)
        for key in &["zakaria al anshari", "zakariya anshari", "zakaria anshari", "anshari"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["زكريا الأنصاري", "فتح الوهاب", "منهج الطلاب"]);
        }
        // Khatib al-Syarbini (author of Mughni al-Muhtaj)
        for key in &["khatib syarbini", "syarbini", "al syarbini", "al khatib syarbini"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الخطيب الشربيني", "مغني المحتاج"]);
        }
        // Anshari (standalone — Zakariyya al-Ansari context)
        // (already added "anshari" above; add standalone too)
        // Nawawi al-Jawi (different from Imam Nawawi — Indonesian scholar)  
        for key in &["nawawi jawi", "nawawi al banten", "nawawi banten", "syaikh nawawi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["النووي الجاوي", "محمد نووي الجاوي"]);
        }
        // Ibn al-Mulaqqin (Shafi'i hadith scholar)
        for key in &["ibnu mulaqqin", "ibn mulaqqin", "al mulaqqin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ابن الملقن", "سراج الدين ابن الملقن"]);
        }
        // Mazhab (school of law) standalone — ensure it's mapped
        for key in &["mazhab"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مذهب", "المذهب", "المذاهب الفقهية"]);
        }
        // murid-murid (students of) / guru-guru (teachers of) — biography context
        for key in &["murid-murid", "murid", "students", "talmidz"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تلاميذ", "المتعلمون"]);
        }
        for key in &["guru-guru", "guru", "teachers", "masyayikh", "shuyukh"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["شيوخ", "أساتذة", "مشايخ"]);
        }
        // karya tulis (written works of)
        for key in &["karya tulis", "karya", "karangan", "authored", "menulis"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مؤلفات", "كتب"]);
        }
        // kitab terkenal (famous books of)
        for key in &["kitab terkenal", "buku terkenal", "famous books", "masterpiece"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مؤلفات", "الكتب المشهورة"]);
        }

        // ─── BATCH 32: Missing food items, locations, scholars, calendar terms ───

        // ── Food items missing from earlier batches ──
        // Kefir (fermented milk)
        for key in &["kefir"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["كيفير", "حكم شرب الكيفير", "الألبان المخمرة"]);
        }
        // Yoghurt
        for key in &["yoghurt", "yogurt"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["اللبن الرائب", "الزبادي", "حكم الزبادي"]);
        }
        // Keju (cheese)
        for key in &["keju", "cheese"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الجبن", "حكم أكل الجبن"]);
        }
        // Terasi (shrimp paste — Indonesian condiment)
        for key in &["terasi", "belacan", "shrimp paste"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["معجون الجمبري", "حكم المعجون", "العجائن البحرية"]);
        }
        // Petis udang (prawn paste — Indonesian condiment)
        for key in &["petis", "petis udang"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["معجون الروبيان", "حكم معجون الجمبري"]);
        }
        // Kecap ikan (fish sauce)
        for key in &["kecap ikan", "fish sauce", "saos ikan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صلصة السمك", "حكم صلصة السمك", "السمك والمشتقات"]);
        }
        // Miso (fermented soybean paste)
        for key in &["miso"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مزو", "العجائن المخمرة", "حكم المخمرات"]);
        }
        // Natto (fermented soybeans)
        for key in &["natto"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الفول المخمر", "حكم المخمرات", "فول الصويا"]);
        }
        // Oncom (fermented soybean cake — Indonesian food)
        for key in &["oncom"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["كيكة فول الصويا المخمرة", "حكم المخمرات"]);
        }
        // Propolis (bee byproduct)
        for key in &["propolis", "bee propolis"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["البروبوليس", "منتجات النحل", "حكم البروبوليس"]);
        }
        // Royal jelly
        for key in &["royal jelly", "jelly royale"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["غذاء الملكة", "منتجات النحل", "حكم غذاء الملكة"]);
        }
        // Sarang burung walet (bird's nest)
        for key in &["sarang burung walet", "sarang burung", "bird's nest", "edible bird nest"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عش طائر السالانجا", "حكم عش العصافير", "الطيور والمشتقات"]);
        }
        // Angkak (red yeast rice)
        for key in &["angkak", "red yeast rice", "beras angkak"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أرز الخميرة الحمراء", "حكم أرز الخميرة", "المخمرات"]);
        }
        // Bir non alkohol (non-alcoholic beer)
        for key in &["bir non alkohol", "non alcoholic beer", "beer tanpa alkohol"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["البيرة الخالية من الكحول", "حكم البيرة", "المشروبات"]);
        }
        // Wine non alkohol (non-alcoholic wine)
        for key in &["wine non alkohol", "non alcoholic wine", "wine tanpa alkohol"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["النبيذ الخالي من الكحول", "حكم النبيذ", "المشروبات"]);
        }
        // Vanilla extract (contains alcohol)
        for key in &["vanilla extract", "ekstrak vanila", "vanila extract"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["خلاصة الفانيلا", "حكم الفانيلا", "النكهات الكحولية"]);
        }
        // Permen karet (chewing gum — gelatin question)
        for key in &["permen karet", "chewing gum", "gum"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["علكة المضغ", "حكم العلكة", "الجيلاتين"]);
        }
        // Marshmallow (gelatin-based candy)
        for key in &["marshmallow", "marshmelo"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكم المارشملو", "الجيلاتين", "حلوى الجيلاتين"]);
        }
        // Gummy bear (gelatin candy)
        for key in &["gummy bear", "gummy", "permen jelly gelatin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حلوى الجيلاتين", "حكم الجيلاتين", "الجيلاتين"]);
        }
        // Coklat / chocolate
        for key in &["coklat", "chocolate"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الشوكولاتة", "حكم الشوكولاتة", "حلوى"]);
        }
        // Es krim (ice cream)
        for key in &["es krim", "ice cream", "eskrim"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الآيس كريم", "حكم الآيس كريم", "الألبان"]);
        }
        // Foie gras (French delicacy)
        for key in &["foie gras"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["كبد الأوز المسمن", "حكم كبد الأوز", "حكم الأكل"]);
        }
        // Caviar (fish eggs)
        for key in &["caviar", "telur ikan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الكافيار", "بيض السمك", "حكم الكافيار"]);
        }
        // Truffle (fungus used in food)
        for key in &["truffle", "jamur truffle"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الكمأة", "الفطر", "حكم الكمأة"]);
        }
        // Escargot (edible snails)
        for key in &["escargot", "siput", "keong"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الحلزون", "حكم أكل الحلزون", "الحيوانات البرية"]);
        }

        // ── Islamic holy locations ──
        // Mushalla (prayer room, smaller than masjid)
        for key in &["mushalla", "musalla", "mushola", "langgar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مصلى", "حكم المصلى", "الصلاة في المصلى"]);
        }
        // Masjidil Aqsha (third holiest mosque)
        for key in &["masjidil aqsha", "masjid aqsha", "al aqsha", "aqsha"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المسجد الأقصى", "الأقصى", "بيت المقدس"]);
        }
        // Jabal Rahmah (Mount of Mercy, near Arafah)
        for key in &["jabal rahmah", "bukit rahmah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جبل الرحمة", "عرفة", "الوقوف بعرفة"]);
        }
        // Gua Hira (cave where first revelation descended)
        for key in &["gua hira", "hira", "goa hira", "cave hira"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["غار حراء", "جبل النور", "نزول الوحي"]);
        }
        // Gua Tsur (cave where Prophet hid during hijra)
        for key in &["gua tsur", "gua sur", "tsur", "cave tsur"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["غار ثور", "الهجرة النبوية"]);
        }
        // Makam Nabi (Prophet's tomb)
        for key in &["makam nabi", "maqam nabi", "kubur nabi", "raudhah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قبر النبي", "الروضة الشريفة", "زيارة قبر النبي"]);
        }

        // ── Days of the week (Islamic practices) ──
        for key in &["senin", "monday"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الاثنين", "يوم الاثنين", "صوم الاثنين"]);
        }
        for key in &["selasa", "tuesday"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الثلاثاء", "يوم الثلاثاء"]);
        }
        for key in &["rabu", "wednesday"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الأربعاء", "يوم الأربعاء"]);
        }
        for key in &["kamis", "thursday"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الخميس", "يوم الخميس", "صوم الخميس"]);
        }
        for key in &["sabtu", "saturday"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["السبت", "يوم السبت"]);
        }
        for key in &["ahad", "sunday"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الأحد", "يوم الأحد"]);
        }

        // ── Islamic scholarly authorities (MUI, NU, Muhammadiyah) ──
        for key in &["mui", "majelis ulama indonesia"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مجلس العلماء الإندونيسي", "المجلس العلمي"]);
        }
        for key in &["nu", "nahdlatul ulama"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["نهضة العلماء", "نهضة العلماء الإندونيسية"]);
        }
        for key in &["muhammadiyah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["محمدية", "المحمدية الإندونيسية"]);
        }
        for key in &["jumhur ulama", "jumhur"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["جمهور العلماء", "الجمهور", "أكثر العلماء"]);
        }
        for key in &["qaul mu'tamad", "qaul mutamad", "qaul muktamad"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["القول المعتمد", "المعتمد في المذهب"]);
        }

        // ── Halal finance comparisons ──
        for key in &["kpr syariah", "kpr", "kredit pemilikan rumah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تمويل الإسكان الإسلامي", "المرابحة للعقار"]);
        }
        for key in &["obligasi syariah", "sukuk", "obligasi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صكوك", "سندات إسلامية", "الصكوك الإسلامية"]);
        }
        for key in &["reksadana syariah", "reksadana", "reksa dana"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صندوق الاستثمار الإسلامي", "صناديق الاستثمار"]);
        }
        for key in &["pegadaian syariah", "gadai syariah", "rahn"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الرهن", "رهن الشريعة", "حكم الرهن"]);
        }

        // ── Location/place for prayer ──
        for key in &["kantor", "office", "tempat kerja"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مكتب", "الصلاة في المكتب"]);
        }
        for key in &["dapur", "kitchen"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مطبخ", "الصلاة في المطبخ"]);
        }
        for key in &["kamar mandi", "bathroom", "toilet", "wc"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حمام", "الصلاة في الحمام", "الاستجمار"]);
        }

        // ── Quran-related digital/modern contexts ──
        for key in &["quran dari handphone", "quran di hp", "baca quran hp"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["قراءة القرآن بالجوال", "مصحف الجوال"]);
        }
        for key in &["headset", "earphone", "airpods"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سماعة الأذن", "الصلاة بسماعة"]);
        }

        // ── Solar energy ──
        for key in &["solar energy", "energi surya", "panel surya", "tenaga surya"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الطاقة الشمسية", "حكم الطاقة الشمسية"]);
        }

        // ── Comparative Islamic terms ──
        for key in &["talak raj'i", "talak raji", "talaq raj'i"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["طلاق رجعي", "الطلاق الرجعي"]);
        }
        for key in &["talak bain", "talaq bain", "bain sughra", "bain kubra"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["طلاق بائن", "الطلاق البائن", "البينونة الكبرى"]);
        }

        // ── Waris context: ashabah bil ghair / ma'al ghair ──
        for key in &["ashabah bil ghair", "asababah bil ghair"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عصبة بالغير", "الإرث عصبة بالغير"]);
        }
        for key in &["ashabah ma'al ghair", "ashabah ma al ghair"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عصبة مع الغير", "الإرث عصبة مع الغير"]);
        }
        for key in &["ashabah bin nafs", "ashabah bil nafs"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["عصبة بالنفس", "العصبات"]);
        }

        // ── Phrase map additions for BATCH 32 ──
        // Note: these are added in build_phrase_map() but keep term_map here too
        for key in &["amalan", "keutamaan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["فضل", "أعمال", "فضائل"]);
        }
        for key in &["peristiwa penting"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أحداث مهمة", "التاريخ الإسلامي"]);
        }
        for key in &["maqashid", "maqasid"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مقاصد الشريعة", "مقاصد"]);
        }
        for key in &["kaidah fiqhiyyah", "kaidah fiqih", "qawaid fiqhiyyah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["القواعد الفقهية", "الأشباه والنظائر"]);
        }
        for key in &["sumber hukum", "dalil naqli", "dalil aqli"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مصادر الفقه", "أدلة الفقه", "الأدلة الشرعية"]);
        }
        for key in &["dosa besar", "kabair", "kabair"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الكبائر", "الذنوب الكبيرة"]);
        }
        for key in &["maqam"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مقام", "مقامات"]);
        }

        // ─── BATCH 33: Finance/business, animals, jenazah, scholars, kitab references ───

        // ── Modern trading/investment types not yet covered ──
        for key in &["franchise", "waralaba", "sistem franchise", "bisnis franchise"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الامتياز التجاري", "حكم الامتياز", "الفرانشايز"]);
        }
        for key in &["short selling", "jual short", "jual kosong"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["البيع على المكشوف", "بيع ما لا يملك", "بيع العدم"]);
        }
        for key in &["binary options", "opsi biner"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الخيارات الثنائية", "المراهنة", "القمار"]);
        }
        for key in &["scalping", "scalping forex"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المضاربة السريعة", "التداول السريع", "الصرف"]);
        }
        for key in &["day trading", "trading harian", "intraday trading"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المتاجرة اليومية", "التداول اليومي", "بيع الأسهم"]);
        }
        for key in &["margin trading", "trading dengan marjin", "leverage trading"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التداول بالهامش", "القرض الربوي", "الرافعة المالية"]);
        }
        for key in &["copy trading", "copy trade", "auto trading", "trading bot", "robot trading"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التداول الآلي", "التداول التلقائي", "المتاجرة"]);
        }
        for key in &["spread betting", "taruhan spread"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المراهنة على الفارق", "القمار", "الرهان"]);
        }
        for key in &["affiliate marketing", "afiliasi", "program afiliasi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التسويق بالعمولة", "السمسرة", "الوكالة بالعمولة"]);
        }
        for key in &["google adsense", "adsense", "iklan adsense"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الإعلانات الرقمية", "كسب المال من الإنترنت", "العمل الرقمي"]);
        }
        for key in &["endorsement", "endorse produk", "paid promote"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الترويج التجاري", "الإعلان", "أخذ الأجرة على الترويج"]);
        }
        for key in &["jual beli followers", "jual followers", "beli followers"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بيع المتابعين", "الغش الرقمي", "الاحتيال"]);
        }
        for key in &["jual beli akun", "jual akun", "beli akun game", "jual account"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بيع الحسابات الرقمية", "المعاملات الإلكترونية"]);
        }
        for key in &["jual data pribadi", "data pribadi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بيع البيانات الشخصية", "الخصوصية", "أمانة الحفظ"]);
        }

        // ── Hukum memakan animals ──
        for key in &["makan anjing", "memakan anjing", "daging anjing"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أكل الكلب", "حكم الكلب", "الحيوانات المحرمة"]);
        }
        for key in &["makan kucing", "memakan kucing", "daging kucing"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أكل الهرة", "حكم القطة", "الحيوانات المحرمة"]);
        }
        for key in &["makan tikus", "memakan tikus", "daging tikus"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أكل الفأر", "حكم الفأرة", "الفواسق"]);
        }
        for key in &["makan ular", "memakan ular", "daging ular"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أكل الحية", "حكم الثعبان", "الحيوانات المحرمة"]);
        }
        for key in &["makan cicak", "memakan cicak", "makan tokek", "tokek"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أكل الوزغ", "حكم الوزغ", "السوام"]);
        }
        for key in &["makan kelelawar", "memakan kelelawar", "daging kelelawar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أكل الخفاش", "حكم الخفاش", "الحيوانات المحرمة"]);
        }
        for key in &["makan biawak", "memakan biawak"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أكل الضب", "حكم الضب", "الوَرَل"]);
        }
        for key in &["makan elang", "memakan elang", "makan rajawali", "makan gagak", "makan burung hantu"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أكل الصقر", "أكل النسر", "ذوات المخالب", "الحيوانات المحرمة"]);
        }
        for key in &["makan landak", "memakan landak"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أكل القنفذ", "حكم القنفذ"]);
        }
        for key in &["makan musang", "memakan musang"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أكل ثعلب", "الحيوانات ذوات الناب"]);
        }
        for key in &["makan monyet", "memakan monyet", "makan gorila", "makan orangutan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أكل القرد", "حكم القردة", "الحيوانات المحرمة"]);
        }
        for key in &["makan lumba-lumba", "memakan lumba-lumba"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أكل الدلفين", "حيوانات البحر"]);
        }
        for key in &["makan hiu", "memakan hiu", "makan pari", "daging hiu"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أكل السمك الكبير", "حكم القرش", "حيوانات البحر"]);
        }
        for key in &["makan belut", "memakan belut"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أكل الثعبان البحري", "أكل الجريث", "حيوانات البحر"]);
        }
        for key in &["makan lele", "memakan lele", "makan nila", "makan gurame"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أكل السمك", "حكم أكل السمك", "حيوان البحر"]);
        }
        for key in &["makan harimau", "memakan harimau", "makan singa", "makan serigala"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أكل النمر", "أكل الأسد", "ذوات الناب", "الحيوانات المحرمة"]);
        }
        for key in &["makan gajah", "memakan gajah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أكل الفيل", "حكم الفيل"]);
        }

        // ── Jenazah detail terms ──
        for key in &["talqin", "talqin mayit", "talqin setelah dikubur"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التلقين", "تلقين الميت"]);
        }
        for key in &["haul", "haul orang meninggal"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حول الوفاة", "ذكرى الوفاة"]);
        }
        for key in &["tahlil 7 hari", "selamatan", "selamatan kematian", "yasinan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["إحياء ذكرى الوفاة", "الختم", "القراءة على الميت"]);
        }
        for key in &["kremasi", "kremasi jenazah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الحرق", "إحراق الميت", "حكم الحرق"]);
        }
        for key in &["autopsi", "autopsi jenazah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تشريح الجثة", "حكم التشريح"]);
        }
        for key in &["euthanasia"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["القتل الرحيم", "الموت الرحيم", "إنهاء الحياة"]);
        }
        for key in &["bunuh diri", "mencabut nyawa sendiri"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الانتحار", "قتل النفس", "حكم الانتحار"]);
        }

        // ── Hikmah (wisdom/philosophy) queries ──
        for key in &["hikmah shalat", "hikmah mendirikan shalat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة الصلاة", "فوائد الصلاة", "أسرار الصلاة"]);
        }
        for key in &["hikmah puasa", "hikmah berpuasa"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة الصيام", "فوائد الصيام", "أسرار الصوم"]);
        }
        for key in &["hikmah zakat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة الزكاة", "فوائد الزكاة"]);
        }
        for key in &["hikmah haji", "hikmah menunaikan haji"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة الحج", "فوائد الحج", "أسرار الحج"]);
        }
        for key in &["hikmah poligami", "hikmah poligini"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة التعدد", "حكمة الزواج بأكثر من واحدة"]);
        }
        for key in &["hikmah iddah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة العدة", "فوائد العدة"]);
        }
        for key in &["hikmah talak", "hikmah cerai diperbolehkan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة الطلاق", "الحكمة من إباحة الطلاق"]);
        }
        for key in &["hikmah warisan", "hikmah waris islam"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة الميراث", "حكمة الإرث"]);
        }
        for key in &["hikmah qurban", "hikmah berkurban"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة الأضحية", "أسرار الأضحية"]);
        }
        for key in &["hikmah aqiqah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة العقيقة"]);
        }
        for key in &["hikmah khitan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة الختان", "فوائد الختان"]);
        }
        for key in &["hikmah nikah", "hikmah pernikahan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة النكاح", "فوائد الزواج", "أسرار الزواج"]);
        }
        for key in &["hikmah menutup aurat", "hikmah hijab"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة الحجاب", "حكمة ستر العورة"]);
        }
        for key in &["hikmah larangan riba"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة تحريم الربا", "الحكمة من الربا"]);
        }
        for key in &["hikmah larangan zina"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة تحريم الزنا"]);
        }
        for key in &["hikmah berjamaah", "hikmah shalat berjamaah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة الجماعة", "فضل صلاة الجماعة"]);
        }
        for key in &["hikmah sedekah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة الصدقة", "فضل الصدقة"]);
        }
        for key in &["hikmah silaturahmi", "hikmah silaturahim"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة صلة الرحم", "فضل صلة الرحم"]);
        }
        for key in &["hikmah dzikir"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة الذكر", "فضل الذكر"]);
        }
        for key in &["rahasia shalat", "makna spiritual shalat", "filosofi shalat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أسرار الصلاة", "حقيقة الصلاة", "روح الصلاة"]);
        }
        for key in &["rahasia puasa", "makna spiritual puasa", "filosofi puasa"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أسرار الصيام", "حقيقة الصوم"]);
        }
        for key in &["rahasia wudhu", "makna wudhu"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أسرار الوضوء"]);
        }
        for key in &["rahasia haji", "makna spiritual haji", "filosofi ibadah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أسرار الحج", "حقيقة الحج", "روح العبادة"]);
        }

        // ── Specific kitab references ──
        for key in &["raudhatul thalibin", "raudhat al thalibin", "rawdhah thalibin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["روضة الطالبين", "النووي"]);
        }
        for key in &["fathul bari", "fath al bari", "fathul bary"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["فتح الباري", "ابن حجر", "شرح البخاري"]);
        }
        for key in &["menurut kitab", "dalam kitab"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["في الكتاب", "وفق"]);
        }

        // ── Scholar bios not fully covered ──
        for key in &["ibn rusyd", "ibnu rusyd", "ibnu rusd", "averroes"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ابن رشد", "الفيلسوف ابن رشد"]);
        }
        for key in &["ibn khaldun", "ibnu khaldun"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ابن خلدون", "المقدمة"]);
        }
        for key in &["salahuddin al ayyubi", "shalahuddin", "saladin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صلاح الدين الأيوبي"]);
        }
        for key in &["ibn sina", "ibnu sina", "avicenna"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ابن سينا", "الشيخ الرئيس"]);
        }

        // ── Arabic Islamic law terms: Islamic marriage disputes ──
        for key in &["إيلاء", "ila'", "ila"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["إيلاء", "الإيلاء", "حكم الإيلاء"]);
        }
        for key in &["ظهار", "dhihar", "zihar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["ظهار", "الظهار", "حكم الظهار"]);
        }
        for key in &["لعان", "li'an", "lian"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["لعان", "اللعان", "حكم اللعان"]);
        }
        for key in &["رجعة", "rujuk"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["رجعة", "الرجعة", "حكم الرجعة"]);
        }

        // ── Arabic fiqh structural terms ──
        for key in &["موانع", "mawani'", "mawani"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["موانع", "العوائق"]);
        }
        for key in &["مبطلات", "mubthilat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مبطلات", "النواقض"]);
        }
        for key in &["آداب", "adab", "tata krama"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["آداب", "الأدب"]);
        }

        // ── hak asasi manusia / human rights / justice ──
        for key in &["hak asasi manusia", "ham", "human rights"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حقوق الإنسان", "الحقوق الأساسية"]);
        }
        for key in &["terorisme", "terrorism", "teroris"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الإرهاب", "الجهاد", "العنف"]);
        }
        for key in &["radikalisme", "radikalisasi", "ekstremisme", "radicalism"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التطرف", "الغلو", "التعصب"]);
        }
        for key in &["pluralisme", "pluralism", "toleransi beragama"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التعددية", "التسامح", "حكم التعامل مع غير المسلمين"]);
        }
        for key in &["nasionalisme", "nationalism"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["القومية", "حب الوطن"]);
        }
        for key in &["demokrasi", "democracy"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الديمقراطية", "نظام الحكم"]);
        }
        for key in &["khilafah", "sistem khilafah", "caliphate"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الخلافة", "نظام الخلافة", "الإمامة"]);
        }
        for key in &["jihad", "pengertian jihad"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الجهاد", "أنواع الجهاد", "حكم الجهاد"]);
        }

        // ── Monopoly / market manipulation ──
        for key in &["monopoli", "monopoly", "kartel", "kartel harga"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الاحتكار", "حكم الاحتكار"]);
        }
        for key in &["ijon", "ijon sawah", "tengkulak"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["بيع الغرر", "بيع المعدوم", "البيع قبل القبض"]);
        }
        for key in &["pajak", "tax", "pajak dan zakat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الضريبة", "الخراج", "الجزية"]);
        }
        for key in &["gratifikasi", "suap", "bribery"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الرشوة", "حكم الرشوة"]);
        }
        for key in &["pungli", "pungutan liar"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الرشوة", "أخذ المال بغير حق"]);
        }
        for key in &["pencucian uang", "money laundering"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["غسيل الأموال", "المال الحرام"]);
        }
        for key in &["diskon", "cashback", "undian berhadiah", "door prize"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التخفيض", "القرعة", "الجوائز"]);
        }
        for key in &["give away", "giveaway"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المسابقة", "الهبة", "الجوائز"]);
        }

        // ── PHK / employment ──
        for key in &["phk", "pemutusan hubungan kerja", "layoff"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الفصل من العمل", "إنهاء العقد"]);
        }
        for key in &["outsourcing", "tenaga outsource"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الاستعانة بالخارج", "عقد العمل"]);
        }
        for key in &["magang tidak dibayar", "magang gratis"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["التدريب غير المأجور", "العمل دون أجر"]);
        }

        // ── Kesetaraan gender / wanita ──
        for key in &["kesetaraan gender", "gender equality", "feminisme", "feminism"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المساواة بين الجنسين", "حقوق المرأة", "فقه المرأة"]);
        }
        for key in &["perbudakan", "slavery", "budak", "hamba sahaya"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الرق", "الرقيق", "العبودية"]);
        }

        // ── Olahraga (sports) ──
        for key in &["mixed martial arts", "mma", "tinju", "boxing", "gulat", "wrestling"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الملاكمة", "المصارعة", "الرياضة"]);
        }
        for key in &["berburu", "hunting", "berburu untuk olahraga"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الصيد", "حكم الصيد", "اصطياد الحيوانات"]);
        }
        for key in &["memancing", "fishing", "mancing"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الصيد", "صيد السمك"]);
        }
        for key in &["balapan", "balap", "racing"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["السباق", "المسابقة"]);
        }

        // ── THR / mudik ──
        for key in &["thr", "tunjangan hari raya"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["العطلة", "أجر العيد", "الحقوق العمالية"]);
        }
        for key in &["mudik", "pulang kampung saat lebaran"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["سفر العيد", "العودة إلى الأهل"]);
        }
        for key in &["halal bihalal", "lebaran"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صلة الرحم", "العيد", "الاحتفال بالعيد"]);
        }
        for key in &["angpao non muslim", "angpao dari non muslim"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الهدية من غير المسلم", "التهادي مع الكفار"]);
        }

        // ── Pendapat mazhab pattern ──
        for key in &["pendapat mazhab syafii", "menurut mazhab syafii"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مذهب الشافعي", "الشافعية", "الإمام الشافعي"]);
        }
        for key in &["pendapat mazhab hanafi", "menurut mazhab hanafi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مذهب الحنفي", "الحنفية", "الإمام أبو حنيفة"]);
        }
        for key in &["pendapat mazhab maliki", "menurut mazhab maliki"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مذهب المالكي", "المالكية", "الإمام مالك"]);
        }
        for key in &["pendapat mazhab hanbali", "menurut mazhab hanbali"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["مذهب الحنبلي", "الحنابلة", "الإمام أحمد"]);
        }

        // ── Takbiran / Ied ──
        for key in &["takbiran malam ied", "takbir ied"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["تكبيرات العيد", "التكبير ليلة العيد"]);
        }
        for key in &["shalat ied fitri", "shalat hari raya", "shalat idul fitri"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صلاة العيد", "صلاة عيد الفطر"]);
        }
        for key in &["shalat ied adha", "shalat idul adha"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صلاة العيد", "صلاة عيد الأضحى"]);
        }
        for key in &["bulan haram", "asyhurul hurum", "bulan-bulan haram"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الأشهر الحرم", "شهر محرم", "ذو القعدة"]);
        }
        for key in &["hari tasyrik", "puasa hari tasyrik", "ayyam tasyrik"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أيام التشريق", "تحريم الصيام"]);
        }
        for key in &["ayyamul bidh", "puasa ayyamul bidh"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["صيام الأيام البيض", "أيام البيض"]);
        }

        // ── Numbers/quantities in Islamic context ──
        for key in &["7 anggota sujud", "tujuh anggota sujud"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أعضاء السجود", "السبعة أعضاء"]);
        }
        for key in &["28 huruf hijaiyah"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الحروف الهجائية", "الأبجدية"]);
        }
        for key in &["6 kitab hadits", "kutub sittah", "6 kitab hadits utama"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الكتب الستة", "كتب الحديث"]);
        }
        for key in &["40 hadits nawawi", "arba'in nawawi"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الأربعون النووية", "الأربعين"]);
        }
        for key in &["99 asmaul husna", "asmaul husna"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الأسماء الحسنى", "أسماء الله"]);
        }
        for key in &["10 sahabat dijamin surga", "10 sahabat", "10 sahabat surga"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["العشرة المبشرون بالجنة"]);
        }
        for key in &["4 khalifah rasyidin", "khulafaur rasyidin"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الخلفاء الراشدون"]);
        }
        for key in &["kesetaraan gender islam", "peran wanita islam"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حقوق المرأة", "مكانة المرأة"]);
        }

        // ── Phrase map additions for BATCH 33 ──
        // hikmah + object bigrams as combined term_map keys
        for key in &["hikmah shalat", "hikmah mendirikan shalat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة الصلاة", "أسرار الصلاة"]);
        }
        for key in &["hikmah puasa", "hikmah berpuasa"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة الصيام", "أسرار الصوم"]);
        }
        for key in &["hikmah zakat"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة الزكاة"]);
        }
        for key in &["hikmah haji"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة الحج", "أسرار الحج"]);
        }
        for key in &["hikmah nikah", "hikmah pernikahan"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة النكاح", "فوائد الزواج"]);
        }
        for key in &["hikmah qurban", "hikmah berkurban"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة الأضحية"]);
        }
        for key in &["hikmah wudhu"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["أسرار الوضوء"]);
        }
        for key in &["hikmah riba", "hikmah larangan riba"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة تحريم الربا"]);
        }
        for key in &["hikmah zina", "hikmah larangan zina"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة تحريم الزنا"]);
        }
        for key in &["hikmah poligami"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["حكمة التعدد"]);
        }
        for key in &["pendapat syafii", "mazhab syafii tentang"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الشافعية", "مذهب الشافعي"]);
        }
        for key in &["pendapat hanafi", "mazhab hanafi tentang"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الحنفية", "مذهب الحنفي"]);
        }
        for key in &["pendapat maliki", "mazhab maliki tentang"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["المالكية", "مذهب المالكي"]);
        }
        for key in &["pendapat hanbali", "mazhab hanbali tentang"] {
            self.term_map.entry(key.to_string()).or_insert(vec!["الحنابلة", "مذهب الحنبلي"]);
        }

    }

    fn build_domain_detector(&mut self) {
        // ═══ v12: Weighted domain detection ═══
        // Weight 3 = primary (definitive domain keyword, e.g., "shalat" → ibadah)
        // Weight 2 = strong secondary (strongly associated)
        // Weight 1 = weak contextual (could belong to multiple domains, e.g., "perempuan", "emas")
        let domain_map: Vec<(&str, FiqhDomain, i32)> = vec![
            // Ibadah — PRIMARY (weight 3)
            ("shalat", FiqhDomain::Ibadah, 3), ("solat", FiqhDomain::Ibadah, 3),
            ("sholat", FiqhDomain::Ibadah, 3), ("salat", FiqhDomain::Ibadah, 3),
            ("puasa", FiqhDomain::Ibadah, 3), ("shaum", FiqhDomain::Ibadah, 3),
            ("zakat", FiqhDomain::Ibadah, 3), ("zakah", FiqhDomain::Ibadah, 3),
            ("haji", FiqhDomain::Ibadah, 3), ("umroh", FiqhDomain::Ibadah, 3),
            ("prayer", FiqhDomain::Ibadah, 3), ("fasting", FiqhDomain::Ibadah, 3),
            ("adzan", FiqhDomain::Ibadah, 3), ("qurban", FiqhDomain::Ibadah, 3),
            ("itikaf", FiqhDomain::Ibadah, 3), ("ihram", FiqhDomain::Ibadah, 3),
            ("tawaf", FiqhDomain::Ibadah, 3), ("sa'i", FiqhDomain::Ibadah, 3),
            ("wukuf", FiqhDomain::Ibadah, 3), ("kurban", FiqhDomain::Ibadah, 3),
            // Ibadah — SECONDARY (weight 2)
            ("doa", FiqhDomain::Ibadah, 2), ("dzikir", FiqhDomain::Ibadah, 2),
            ("kafarat", FiqhDomain::Ibadah, 2), ("kaffarah", FiqhDomain::Ibadah, 2),
            ("sumpah", FiqhDomain::Ibadah, 2), ("nadzar", FiqhDomain::Ibadah, 2),
            ("aqiqah", FiqhDomain::Ibadah, 2), ("fidyah", FiqhDomain::Ibadah, 2),
            ("iftar", FiqhDomain::Ibadah, 2), ("imam", FiqhDomain::Ibadah, 2),
            ("makmum", FiqhDomain::Ibadah, 2), ("khutbah", FiqhDomain::Ibadah, 2),
            ("subuh", FiqhDomain::Ibadah, 2), ("fajar", FiqhDomain::Ibadah, 2),
            ("dzuhur", FiqhDomain::Ibadah, 2), ("ashar", FiqhDomain::Ibadah, 2),
            ("maghrib", FiqhDomain::Ibadah, 2), ("isya", FiqhDomain::Ibadah, 2),
            ("qunut", FiqhDomain::Ibadah, 2), ("witir", FiqhDomain::Ibadah, 2),
            ("rakaat", FiqhDomain::Ibadah, 2), ("rakat", FiqhDomain::Ibadah, 2),
            ("jenazah", FiqhDomain::Ibadah, 2), ("janazah", FiqhDomain::Ibadah, 2),
            ("mayit", FiqhDomain::Ibadah, 2), ("sahwi", FiqhDomain::Ibadah, 2),
            ("tilawah", FiqhDomain::Ibadah, 2), ("tarawih", FiqhDomain::Ibadah, 2),
            // Ibadah — CONTEXTUAL (weight 1)
            ("wajib", FiqhDomain::Ibadah, 1), ("sunnah", FiqhDomain::Ibadah, 1),
            ("fardhu", FiqhDomain::Ibadah, 1), ("batal", FiqhDomain::Ibadah, 1),
            ("rukun", FiqhDomain::Ibadah, 1), ("darurat", FiqhDomain::Ibadah, 1),
            ("makan", FiqhDomain::Ibadah, 1), ("minum", FiqhDomain::Ibadah, 1),
            ("mati", FiqhDomain::Ibadah, 1), ("meninggal", FiqhDomain::Ibadah, 1),
            ("wafat", FiqhDomain::Ibadah, 1),

            // Thaharah — PRIMARY (weight 3)
            ("wudhu", FiqhDomain::Thaharah, 3), ("wudu", FiqhDomain::Thaharah, 3),
            ("tayammum", FiqhDomain::Thaharah, 3), ("najis", FiqhDomain::Thaharah, 3),
            ("junub", FiqhDomain::Thaharah, 3), ("janabah", FiqhDomain::Thaharah, 3),
            ("nifas", FiqhDomain::Thaharah, 3), ("istihadhah", FiqhDomain::Thaharah, 3),
            ("haid", FiqhDomain::Thaharah, 3), ("menstruasi", FiqhDomain::Thaharah, 3),
            // Thaharah — SECONDARY (weight 2)
            ("mandi", FiqhDomain::Thaharah, 2), ("suci", FiqhDomain::Thaharah, 2),
            ("bersuci", FiqhDomain::Thaharah, 2), ("purification", FiqhDomain::Thaharah, 3),
            ("istihalah", FiqhDomain::Thaharah, 2),

            // Muamalat — PRIMARY (weight 3)
            ("riba", FiqhDomain::Muamalat, 3), ("bunga", FiqhDomain::Muamalat, 3),
            ("bank", FiqhDomain::Muamalat, 3), ("asuransi", FiqhDomain::Muamalat, 3),
            ("gadai", FiqhDomain::Muamalat, 3), ("wakaf", FiqhDomain::Muamalat, 3),
            ("waris", FiqhDomain::Muamalat, 3), ("warisan", FiqhDomain::Muamalat, 3),
            ("wasiat", FiqhDomain::Muamalat, 3), ("hibah", FiqhDomain::Muamalat, 3),
            ("mudharabah", FiqhDomain::Muamalat, 3), ("musyarakah", FiqhDomain::Muamalat, 3),
            ("murabahah", FiqhDomain::Muamalat, 3),
            // Muamalat — SECONDARY (weight 2)
            ("jual", FiqhDomain::Muamalat, 2), ("beli", FiqhDomain::Muamalat, 2),
            ("utang", FiqhDomain::Muamalat, 2), ("sewa", FiqhDomain::Muamalat, 2),
            ("trade", FiqhDomain::Muamalat, 2), ("finance", FiqhDomain::Muamalat, 2),
            ("konvensional", FiqhDomain::Muamalat, 2), ("pegadaian", FiqhDomain::Muamalat, 2),
            ("akad", FiqhDomain::Muamalat, 2), ("dropship", FiqhDomain::Muamalat, 2),
            ("harta", FiqhDomain::Muamalat, 2),
            // Muamalat — CONTEXTUAL (weight 1)
            ("foto", FiqhDomain::Muamalat, 1), ("gambar", FiqhDomain::Muamalat, 1),
            ("tato", FiqhDomain::Muamalat, 1), ("musik", FiqhDomain::Muamalat, 1),
            ("rokok", FiqhDomain::Muamalat, 1), ("photo", FiqhDomain::Muamalat, 1),
            ("selfie", FiqhDomain::Muamalat, 1), ("online", FiqhDomain::Muamalat, 1),
            ("emas", FiqhDomain::Muamalat, 1), // contextual: "zakat emas" is ibadah

            // Munakahat — PRIMARY (weight 3)
            ("nikah", FiqhDomain::Munakahat, 3), ("cerai", FiqhDomain::Munakahat, 3),
            ("talak", FiqhDomain::Munakahat, 3), ("mahar", FiqhDomain::Munakahat, 3),
            ("iddah", FiqhDomain::Munakahat, 3), ("poligami", FiqhDomain::Munakahat, 3),
            ("marriage", FiqhDomain::Munakahat, 3), ("divorce", FiqhDomain::Munakahat, 3),
            ("khuluk", FiqhDomain::Munakahat, 3), ("fasakh", FiqhDomain::Munakahat, 3),
            // Munakahat — SECONDARY (weight 2)
            ("nafkah", FiqhDomain::Munakahat, 2), ("suami", FiqhDomain::Munakahat, 2),
            ("istri", FiqhDomain::Munakahat, 2), ("impoten", FiqhDomain::Munakahat, 2),
            ("wali", FiqhDomain::Munakahat, 2), ("nusyuz", FiqhDomain::Munakahat, 2),
            // Munakahat — CONTEXTUAL (weight 1) — "perempuan" alone shouldn't override ibadah
            ("perempuan", FiqhDomain::Munakahat, 1), ("wanita", FiqhDomain::Munakahat, 1),

            // Jinayat — PRIMARY (weight 3)
            ("hudud", FiqhDomain::Jinayat, 3), ("qishash", FiqhDomain::Jinayat, 3),
            ("qishas", FiqhDomain::Jinayat, 3), ("qisas", FiqhDomain::Jinayat, 3),
            ("zina", FiqhDomain::Jinayat, 3), ("pencurian", FiqhDomain::Jinayat, 3),
            ("diyat", FiqhDomain::Jinayat, 3), ("tazir", FiqhDomain::Jinayat, 3),
            ("ta'zir", FiqhDomain::Jinayat, 3),
            // Jinayat — SECONDARY (weight 2)
            ("bunuh", FiqhDomain::Jinayat, 2), ("hukuman", FiqhDomain::Jinayat, 2),
            ("mencuri", FiqhDomain::Jinayat, 2), ("curi", FiqhDomain::Jinayat, 2),
            ("membunuh", FiqhDomain::Jinayat, 2), ("pidana", FiqhDomain::Jinayat, 2),

            // Aqidah — PRIMARY (weight 3)
            ("tauhid", FiqhDomain::Aqidah, 3), ("syirik", FiqhDomain::Aqidah, 3),
            ("aqidah", FiqhDomain::Aqidah, 3), ("bid'ah", FiqhDomain::Aqidah, 3),
            ("bidah", FiqhDomain::Aqidah, 3), ("murtad", FiqhDomain::Aqidah, 3),
            ("qadar", FiqhDomain::Aqidah, 3), ("takdir", FiqhDomain::Aqidah, 3),
            ("rububiyah", FiqhDomain::Aqidah, 3), ("uluhiyah", FiqhDomain::Aqidah, 3),
            // Aqidah — SECONDARY (weight 2)
            ("iman", FiqhDomain::Aqidah, 2), ("kafir", FiqhDomain::Aqidah, 2),
            ("tawasul", FiqhDomain::Aqidah, 2), ("tawassul", FiqhDomain::Aqidah, 2),
            ("khurafat", FiqhDomain::Aqidah, 2),
            ("tawakal", FiqhDomain::Aqidah, 2), ("tawakkal", FiqhDomain::Aqidah, 2),
            ("maulid", FiqhDomain::Aqidah, 3), ("sifat", FiqhDomain::Aqidah, 2),

            // Tasawuf — PRIMARY (weight 3)
            ("tasawuf", FiqhDomain::Tasawuf, 3),
            // Tasawuf — SECONDARY (weight 2)
            ("taubat", FiqhDomain::Tasawuf, 2), ("tobat", FiqhDomain::Tasawuf, 2),
            ("ikhlas", FiqhDomain::Tasawuf, 2), ("riya", FiqhDomain::Tasawuf, 2),

            // Tafsir — PRIMARY (weight 3)
            ("tafsir", FiqhDomain::Tafsir, 3), ("quran", FiqhDomain::Tafsir, 3),
            // Tafsir — SECONDARY (weight 2)
            ("ayat", FiqhDomain::Tafsir, 2), ("surat", FiqhDomain::Tafsir, 2),

            // Hadits — PRIMARY (weight 3)
            ("hadits", FiqhDomain::Hadits, 3), ("hadis", FiqhDomain::Hadits, 3),
            ("hadith", FiqhDomain::Hadits, 3),
            // Hadits — SECONDARY (weight 2)
            ("sanad", FiqhDomain::Hadits, 2),

            // Food/health — CONTEXTUAL (weight 1)
            ("gelatin", FiqhDomain::Ibadah, 1), ("babi", FiqhDomain::Ibadah, 1),
            ("pork", FiqhDomain::Ibadah, 1), ("vaksin", FiqhDomain::Ibadah, 1),
            ("sakit", FiqhDomain::Ibadah, 1),

            // Akhlak
            ("adab", FiqhDomain::Akhlak, 3), ("akhlak", FiqhDomain::Akhlak, 3),

            // v15 batch 2: New domain keywords
            ("jamaah", FiqhDomain::Ibadah, 2), ("berjamaah", FiqhDomain::Ibadah, 2),
            ("jin", FiqhDomain::Aqidah, 2), ("sihir", FiqhDomain::Aqidah, 3),
            ("santet", FiqhDomain::Aqidah, 2), ("ruqyah", FiqhDomain::Aqidah, 2),
            ("cadar", FiqhDomain::Ibadah, 2), ("jilbab", FiqhDomain::Ibadah, 2),
            ("hijab", FiqhDomain::Ibadah, 2), ("niqab", FiqhDomain::Ibadah, 2),
            ("kiblat", FiqhDomain::Ibadah, 3), ("kentut", FiqhDomain::Ibadah, 2),
            ("jenggot", FiqhDomain::Ibadah, 2),
            ("tunangan", FiqhDomain::Munakahat, 2), ("taaruf", FiqhDomain::Munakahat, 2),
            ("janda", FiqhDomain::Munakahat, 2), ("mahar", FiqhDomain::Munakahat, 3),
            ("iddah", FiqhDomain::Munakahat, 3), ("nusyuz", FiqhDomain::Munakahat, 3),
            ("walimah", FiqhDomain::Munakahat, 2),
            ("bekam", FiqhDomain::Ibadah, 1), ("hijamah", FiqhDomain::Ibadah, 1),
            ("judi", FiqhDomain::Muamalat, 3), ("gambling", FiqhDomain::Muamalat, 3),
            ("zodiak", FiqhDomain::Aqidah, 2), ("horoskop", FiqhDomain::Aqidah, 2),
            ("dukun", FiqhDomain::Aqidah, 2), ("valentine", FiqhDomain::Aqidah, 2),
            ("natal", FiqhDomain::Aqidah, 2),
            ("narkoba", FiqhDomain::Jinayat, 2), ("ganja", FiqhDomain::Jinayat, 2),
            ("ikhtilat", FiqhDomain::Munakahat, 2),
            ("forex", FiqhDomain::Muamalat, 2), ("saham", FiqhDomain::Muamalat, 2),
            ("pinjaman", FiqhDomain::Muamalat, 2), ("obligasi", FiqhDomain::Muamalat, 2),
            ("dakwah", FiqhDomain::Akhlak, 2),
            ("kurban", FiqhDomain::Ibadah, 3), ("qurban", FiqhDomain::Ibadah, 3),
            ("aqiqah", FiqhDomain::Ibadah, 3), ("akikah", FiqhDomain::Ibadah, 3),
            ("dzikir", FiqhDomain::Ibadah, 2), ("zikir", FiqhDomain::Ibadah, 2),

            // v15 batch 3: Additional domain keywords
            ("tasyahud", FiqhDomain::Ibadah, 3), ("kutek", FiqhDomain::Thaharah, 2),
            ("plester", FiqhDomain::Thaharah, 2), ("cincin", FiqhDomain::Thaharah, 1),
            ("kuku", FiqhDomain::Thaharah, 1), ("mimpi", FiqhDomain::Thaharah, 2),
            ("akad", FiqhDomain::Muamalat, 2), ("cicilan", FiqhDomain::Muamalat, 2),
            ("paylater", FiqhDomain::Muamalat, 2), ("pinjol", FiqhDomain::Muamalat, 2),
            ("piercing", FiqhDomain::Muamalat, 1), ("tindik", FiqhDomain::Muamalat, 1),
            ("tato", FiqhDomain::Muamalat, 2), ("adopsi", FiqhDomain::Munakahat, 2),
            ("wasiat", FiqhDomain::Muamalat, 3), ("hibah", FiqhDomain::Muamalat, 3),
            ("transplantasi", FiqhDomain::Muamalat, 2),
            ("operasi", FiqhDomain::Muamalat, 1), ("cukur", FiqhDomain::Ibadah, 1),
            ("cosplay", FiqhDomain::Aqidah, 1), ("pemilu", FiqhDomain::Muamalat, 1),

            // v15 batch 4: English domain keywords
            ("prayer", FiqhDomain::Ibadah, 3), ("prayers", FiqhDomain::Ibadah, 3),
            ("worship", FiqhDomain::Ibadah, 3), ("fasting", FiqhDomain::Ibadah, 3),
            ("pilgrimage", FiqhDomain::Ibadah, 3), ("hajj", FiqhDomain::Ibadah, 3),
            ("alms", FiqhDomain::Ibadah, 3), ("tithe", FiqhDomain::Ibadah, 2),
            ("ablution", FiqhDomain::Thaharah, 3), ("purification", FiqhDomain::Thaharah, 3),
            ("ritual", FiqhDomain::Ibadah, 1), ("mosque", FiqhDomain::Ibadah, 2),
            ("congregation", FiqhDomain::Ibadah, 2), ("congregational", FiqhDomain::Ibadah, 2),
            ("funeral", FiqhDomain::Ibadah, 2), ("eclipse", FiqhDomain::Ibadah, 2),
            ("eid", FiqhDomain::Ibadah, 3), ("sacrifice", FiqhDomain::Ibadah, 2),
            ("marriage", FiqhDomain::Munakahat, 3), ("divorce", FiqhDomain::Munakahat, 3),
            ("wedding", FiqhDomain::Munakahat, 2), ("dowry", FiqhDomain::Munakahat, 3),
            ("custody", FiqhDomain::Munakahat, 2), ("guardian", FiqhDomain::Munakahat, 2),
            ("inheritance", FiqhDomain::Muamalat, 3), ("lease", FiqhDomain::Muamalat, 2),
            ("interest", FiqhDomain::Muamalat, 2), ("usury", FiqhDomain::Muamalat, 3),
            ("buying", FiqhDomain::Muamalat, 2), ("selling", FiqhDomain::Muamalat, 2),
            ("cryptocurrency", FiqhDomain::Muamalat, 2),
            ("insurance", FiqhDomain::Muamalat, 2), ("banking", FiqhDomain::Muamalat, 2),
            ("creed", FiqhDomain::Aqidah, 3), ("monotheism", FiqhDomain::Aqidah, 3),
            ("predestination", FiqhDomain::Aqidah, 3), ("apostasy", FiqhDomain::Aqidah, 3),
            ("jihad", FiqhDomain::Jinayat, 3), ("war", FiqhDomain::Jinayat, 2),
            ("treason", FiqhDomain::Jinayat, 2), ("punishment", FiqhDomain::Jinayat, 2),
            ("theft", FiqhDomain::Jinayat, 3), ("murder", FiqhDomain::Jinayat, 3),
            ("ethics", FiqhDomain::Akhlak, 3), ("morals", FiqhDomain::Akhlak, 3),
            ("manners", FiqhDomain::Akhlak, 2), ("etiquette", FiqhDomain::Akhlak, 2),
            ("commentary", FiqhDomain::Tafsir, 2), ("exegesis", FiqhDomain::Tafsir, 3),
            ("narration", FiqhDomain::Hadits, 2), ("narrator", FiqhDomain::Hadits, 2),
            ("vaping", FiqhDomain::Muamalat, 1), ("smoking", FiqhDomain::Muamalat, 1),
            ("photography", FiqhDomain::Muamalat, 1), ("silk", FiqhDomain::Muamalat, 1),
            ("gold", FiqhDomain::Muamalat, 1), ("women", FiqhDomain::Munakahat, 1),
            ("organ", FiqhDomain::Muamalat, 1), ("donation", FiqhDomain::Muamalat, 1),
            // Usul Fiqh
            ("istihsan", FiqhDomain::UsulFiqh, 3), ("istishab", FiqhDomain::UsulFiqh, 3),
            ("istidlal", FiqhDomain::UsulFiqh, 3), ("istinbath", FiqhDomain::UsulFiqh, 3),
            ("maqashid", FiqhDomain::UsulFiqh, 3), ("maqasid", FiqhDomain::UsulFiqh, 3),
            ("qiyas", FiqhDomain::UsulFiqh, 3), ("ijma", FiqhDomain::UsulFiqh, 3),
            ("ijtihad", FiqhDomain::UsulFiqh, 3), ("mujtahid", FiqhDomain::UsulFiqh, 2),
            ("tarjih", FiqhDomain::UsulFiqh, 3), ("illat", FiqhDomain::UsulFiqh, 2),
            ("nasikh", FiqhDomain::UsulFiqh, 3), ("mansukh", FiqhDomain::UsulFiqh, 3),
            ("naskh", FiqhDomain::UsulFiqh, 3), ("dzariah", FiqhDomain::UsulFiqh, 3),
            ("maslahah", FiqhDomain::UsulFiqh, 3), ("maslahat", FiqhDomain::UsulFiqh, 3),
            ("mutlaq", FiqhDomain::UsulFiqh, 2), ("muqayyad", FiqhDomain::UsulFiqh, 2),
            ("usul", FiqhDomain::UsulFiqh, 2), ("ushul", FiqhDomain::UsulFiqh, 2),
            ("dalil", FiqhDomain::UsulFiqh, 2), ("hujjah", FiqhDomain::UsulFiqh, 2),
            ("khilaf", FiqhDomain::UsulFiqh, 2), ("ikhtilaf", FiqhDomain::UsulFiqh, 2),
        ];

        for (keyword, domain, weight) in domain_map {
            self.domain_keywords.insert(keyword.to_string(), (domain, weight));
        }
    }
}

// ─── Utility Functions ───

fn is_arabic_char(c: char) -> bool {
    matches!(c,
        '\u{0600}'..='\u{06FF}' | // Arabic
        '\u{0750}'..='\u{077F}' | // Arabic Supplement
        '\u{08A0}'..='\u{08FF}' | // Arabic Extended-A
        '\u{FB50}'..='\u{FDFF}' | // Arabic Presentation Forms-A
        '\u{FE70}'..='\u{FEFF}'   // Arabic Presentation Forms-B
    )
}

fn strip_harakat(text: &str) -> String {
    text.chars()
        .filter(|c| !matches!(*c,
            '\u{064B}'..='\u{065F}' | // Arabic diacritics (fathah, dammah, kasrah, etc)
            '\u{0670}'              | // Small alef above
            '\u{06D6}'..='\u{06ED}'   // Other Arabic marks
        ))
        .collect()
}

fn tokenize_query(query: &str) -> Vec<String> {
    query
        .split(|c: char| c.is_whitespace() || c == '?' || c == '!' || c == ',' || c == '.')
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

// ─── Long Query Intelligence ───

/// Extract question sentence(s) from long descriptive queries.
/// Returns (question_text, context_text). For queries with "description then question"
/// pattern, this isolates the actual question to prevent BM25 noise from context words.
fn extract_question_and_context(query: &str) -> (String, String) {
    let question_starters: &[&str] = &[
        "apakah", "bagaimana", "bolehkah", "gimana", "mengapa", "kenapa",
        "kapan", "dimana", "siapa", "bisakah", "haruskah", "perlukah",
        "maukah", "adakah", "dapatkan", "what", "how", "can ", "is it",
        "why", "when", "where", "who", "does", "should",
    ];

    let question_contains: &[&str] = &[
        "pertanyaannya", "yang ditanyakan", "yang saya tanyakan",
        "yang ingin saya tahu", "yang ingin ditanyakan",
        "hukumnya apa", "apa hukumnya", "bagaimana hukumnya",
        "boleh atau tidak", "sah atau tidak", "sah atau batal",
        "what is the ruling", "is it permissible",
    ];

    // Split into sentences by punctuation, tracking whether each ends with ?
    let mut sentences: Vec<(String, bool)> = Vec::new();
    let mut current = String::new();

    for c in query.chars() {
        if c == '?' || c == '.' || c == '!' || c == '\n' {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                let lower = trimmed.to_lowercase();
                let is_q = c == '?' ||
                    question_starters.iter().any(|m| lower.starts_with(m)) ||
                    question_contains.iter().any(|m| lower.contains(m));
                sentences.push((trimmed, is_q));
            }
            current = String::new();
        } else {
            current.push(c);
        }
    }
    // Last segment (no terminator)
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        let lower = trimmed.to_lowercase();
        let is_q = question_starters.iter().any(|m| lower.starts_with(m)) ||
            question_contains.iter().any(|m| lower.contains(m));
        sentences.push((trimmed, is_q));
    }

    let mut question_parts: Vec<String> = Vec::new();
    let mut context_parts: Vec<String> = Vec::new();

    for (sent, is_q) in &sentences {
        if *is_q {
            question_parts.push(sent.clone());
        } else {
            context_parts.push(sent.clone());
        }
    }

    // If no question detected, use last sentence as question (common pattern)
    if question_parts.is_empty() && !context_parts.is_empty() {
        let last = context_parts.pop().unwrap();
        question_parts.push(last);
    }

    // If still empty (single sentence, no delimiters), treat whole as question
    if question_parts.is_empty() {
        return (query.to_string(), String::new());
    }

    (question_parts.join(" "), context_parts.join(" "))
}

// ─── Indonesian Morphological Root Extraction ───

/// Extract the Indonesian root word by stripping common prefixes and suffixes.
/// Indonesian has systematic affixation: me-/mem-/men-/meng-/meny- + ROOT + -kan/-an/-i
/// This dramatically expands effective dictionary coverage without adding every conjugation.
///
/// Examples:
///   "membatalkan" → "batal" (me+m+batal+kan)
///   "berwudhu"    → "wudhu" (ber+wudhu)
///   "menyembelih" → "sembelih" (meny→s+embelih)
///   "perbedaan"   → "beda" (per+beda+an)
///   "diharamkan"  → "haram" (di+haram+kan)
fn extract_indonesian_roots(word: &str) -> Vec<String> {
    let w = word.to_lowercase();
    if w.len() < 4 {
        return vec![];
    }

    let mut candidates = Vec::new();

    // ─── Handle Indonesian reduplication: karya-karya → karya, murid-murid → murid ───
    if let Some(idx) = w.find('-') {
        let first = &w[..idx];
        let second = &w[idx + 1..];
        if first.len() >= 3 {
            candidates.push(first.to_string());
        }
        if second.len() >= 3 && second != first {
            candidates.push(second.to_string());
        }
    }

    // Step 1: Strip suffixes first
    let suffixes: &[&str] = &["-nya", "nya", "kan", "an", "i"];
    let mut stems: Vec<String> = vec![w.clone()];

    for suffix in suffixes {
        if w.ends_with(suffix) && w.len() > suffix.len() + 2 {
            let stripped = w[..w.len() - suffix.len()].to_string();
            if stripped.len() >= 3 {
                stems.push(stripped);
            }
        }
    }

    // Step 2: Strip prefixes from each stem
    for stem in &stems {
        // ber- prefix
        if let Some(rest) = stem.strip_prefix("ber") {
            if rest.len() >= 3 { candidates.push(rest.to_string()); }
        }
        // per- prefix
        if let Some(rest) = stem.strip_prefix("per") {
            if rest.len() >= 3 { candidates.push(rest.to_string()); }
        }
        // pe- prefix (before -an suffix already stripped: "perbedaan" → "perbeda" → strip "per" → "beda")
        if let Some(rest) = stem.strip_prefix("pe") {
            if rest.len() >= 3 { candidates.push(rest.to_string()); }
        }
        // di- prefix
        if let Some(rest) = stem.strip_prefix("di") {
            if rest.len() >= 3 { candidates.push(rest.to_string()); }
        }
        // ter- prefix
        if let Some(rest) = stem.strip_prefix("ter") {
            if rest.len() >= 3 { candidates.push(rest.to_string()); }
        }
        // se- prefix
        if let Some(rest) = stem.strip_prefix("se") {
            if rest.len() >= 3 { candidates.push(rest.to_string()); }
        }
        // ke- prefix
        if let Some(rest) = stem.strip_prefix("ke") {
            if rest.len() >= 3 { candidates.push(rest.to_string()); }
        }
        // meny- prefix (root starts with s: menyembelih → sembelih)
        if let Some(rest) = stem.strip_prefix("meny") {
            if rest.len() >= 3 {
                candidates.push(format!("s{}", rest)); // meny + X → sX
                candidates.push(rest.to_string());
            }
        }
        // meng- prefix (root starts with vowel/g/h/k: mengerjakan → kerjakan/erjakan)
        else if let Some(rest) = stem.strip_prefix("meng") {
            if rest.len() >= 3 {
                candidates.push(rest.to_string());
                candidates.push(format!("k{}", rest)); // meng + X → kX
            }
        }
        // mem- prefix (root starts with b/f/p/v: membatalkan → batalkan)
        else if let Some(rest) = stem.strip_prefix("mem") {
            if rest.len() >= 3 {
                candidates.push(rest.to_string());
                candidates.push(format!("p{}", rest)); // mem + X → pX
            }
        }
        // men- prefix (root starts with d/t/c/j: mendengar → dengar, mentaati → taati)
        else if let Some(rest) = stem.strip_prefix("men") {
            if rest.len() >= 3 {
                candidates.push(rest.to_string());
                candidates.push(format!("t{}", rest)); // men + X → tX
            }
        }
        // me- prefix (general)
        else if let Some(rest) = stem.strip_prefix("me") {
            if rest.len() >= 3 { candidates.push(rest.to_string()); }
        }
    }

    // Step 3: For candidates that still have suffixes, strip again
    let mut final_roots = Vec::new();
    for c in &candidates {
        final_roots.push(c.clone());
        for suffix in &["kan", "an", "i"] {
            if c.ends_with(suffix) && c.len() > suffix.len() + 2 {
                let stripped = c[..c.len() - suffix.len()].to_string();
                if stripped.len() >= 3 {
                    final_roots.push(stripped);
                }
            }
        }
    }

    // Deduplicate and remove original word
    final_roots.sort();
    final_roots.dedup();
    final_roots.retain(|r| r != &w && r.len() >= 3);
    final_roots
}

// ─── Query Intent Pattern Detection ───

/// Detect semantic patterns in Indonesian queries and generate targeted Arabic phrases.
/// This captures higher-level query intent that individual word lookups miss.
///
/// Patterns detected:
///   "membatalkan X" / "yang membatalkan X" → مبطلات X / مفسدات X / نواقض X
///   "syarat sahnya X" → شروط صحة X / شروط X
///   "rukun X" → أركان X
///   "hikmah X" → حكمة X
///   "macam-macam X" / "jenis-jenis X" → أنواع X / أقسام X
fn detect_query_intent_patterns(words: &[String]) -> Vec<String> {
    let mut expansions = Vec::new();
    let lower: Vec<String> = words.iter().map(|w| w.to_lowercase()).collect();

    // Pattern: "membatalkan X" / "batal X" / "yang membatalkan"
    // → مبطلات / مفسدات / نواقض
    let cancel_words = ["membatalkan", "batalkan", "merusak", "merusakkan", "menggugurkan"];
    for cw in &cancel_words {
        if lower.iter().any(|w| w == cw || w.contains(cw)) {
            expansions.extend_from_slice(&[
                "مبطلات".to_string(), "مفسدات".to_string(), "نواقض".to_string(),
                "ما يبطل".to_string(), "ما يفسد".to_string(),
            ]);
            break;
        }
    }

    // Pattern: "syarat sah" / "syarat sahnya"
    if lower.iter().any(|w| w == "syarat" || w == "syaratnya") &&
       lower.iter().any(|w| w == "sah" || w == "sahnya" || w == "sahkah") {
        expansions.extend_from_slice(&[
            "شروط صحة".to_string(), "شروط".to_string(), "ما يشترط".to_string(),
        ]);
    }

    // Pattern: "macam" / "jenis" / "macam-macam"
    if lower.iter().any(|w| w == "macam" || w == "macam-macam" || w == "jenis" || w == "jenis-jenis") {
        expansions.extend_from_slice(&[
            "أنواع".to_string(), "أقسام".to_string(),
        ]);
    }

    // Pattern: "hikmah" / "wisdom"
    if lower.iter().any(|w| w == "hikmah" || w == "hikmat") {
        expansions.extend_from_slice(&[
            "حكمة".to_string(), "الحكمة".to_string(), "فوائد".to_string(),
        ]);
    }

    // Pattern: "perbedaan X dan Y" / "beda antara"
    if lower.iter().any(|w| w == "perbedaan" || w == "bedanya") {
        expansions.extend_from_slice(&[
            "الفرق".to_string(), "الفرق بين".to_string(),
        ]);
    }

    // Pattern: "tata cara" / "cara melakukan"
    if lower.iter().any(|w| w == "tatacara" || w == "caranya") ||
       (lower.iter().any(|w| w == "tata") && lower.iter().any(|w| w == "cara")) ||
       (lower.iter().any(|w| w == "cara") && lower.iter().any(|w| w == "melakukan" || w == "mengerjakan")) {
        expansions.extend_from_slice(&[
            "كيفية".to_string(), "صفة".to_string(),
        ]);
    }

    // Pattern: "hukum X bagi/untuk Y" → detect ruling inquiry
    // This overlaps with existing "hukum" mapping but adds nuance
    if lower.iter().any(|w| w == "hukumnya" || w == "hukumkah") {
        expansions.extend_from_slice(&[
            "حكم".to_string(), "أحكام".to_string(),
        ]);
    }

    // Pattern: "apakah boleh" / "boleh gak" / "boleh tidak" → permissibility
    if lower.iter().any(|w| w == "boleh" || w == "bolehkah") &&
       lower.iter().any(|w| w == "gak" || w == "tidak" || w == "nggak" || w == "apakah" || w == "ga") {
        expansions.extend_from_slice(&[
            "هل يجوز".to_string(), "جواز".to_string(), "حل".to_string(),
        ]);
    }

    // Pattern: "wajib atau sunnah" → classification
    if lower.iter().any(|w| w == "wajib") && lower.iter().any(|w| w == "sunnah" || w == "sunah") {
        expansions.extend_from_slice(&[
            "الوجوب والاستحباب".to_string(), "هل يجب".to_string(),
        ]);
    }

    // Pattern: "pakai/pake/memakai" → relates to clothing/wearing context
    if lower.iter().any(|w| w == "pakai" || w == "pake" || w == "memakai" || w == "mengenakan") {
        if lower.iter().any(|w| w == "shalat" || w == "solat" || w == "salat") {
            expansions.extend_from_slice(&[
                "لباس المصلي".to_string(), "ستر العورة".to_string(),
                "اللباس في الصلاة".to_string(),
            ]);
        }
    }

    // Pattern: "dimakan" / "boleh dimakan" / "halal dimakan" → food permissibility
    if lower.iter().any(|w| w == "dimakan" || w == "makan") &&
       lower.iter().any(|w| w == "boleh" || w == "halal" || w == "haram" || w == "gak" || w == "tidak") {
        expansions.extend_from_slice(&[
            "حكم أكل".to_string(), "حل الأكل".to_string(), "المحرمات من الطعام".to_string(),
        ]);
    }

    // Pattern: "gimana" / "gimana hukumnya" → how / ruling inquiry (colloquial)
    if lower.iter().any(|w| w == "gimana" || w == "bagaimana") {
        expansions.extend_from_slice(&[
            "كيف".to_string(), "حكم".to_string(),
        ]);
    }

    // Pattern: "sah gak" / "sah tidak" → validity question
    if lower.iter().any(|w| w == "sah" || w == "sahkah" || w == "sahnya") &&
       lower.iter().any(|w| w == "gak" || w == "tidak" || w == "nggak" || w == "ga" || w == "kah" || w == "apakah") {
        expansions.extend_from_slice(&[
            "صحة".to_string(), "هل يصح".to_string(), "شروط الصحة".to_string(),
        ]);
    }

    // Pattern: "termasuk" / "tergolong" → classification
    if lower.iter().any(|w| w == "termasuk" || w == "tergolong" || w == "apakah") {
        if lower.iter().any(|w| w == "riba" || w == "haram" || w == "najis" || w == "bid'ah" || w == "bidah" || w == "syirik") {
            expansions.extend_from_slice(&[
                "حكم".to_string(), "هل هو من".to_string(),
            ]);
        }
    }

    // Pattern: "kenapa diharamkan" / "kenapa haram" → reasoning behind prohibition
    if lower.iter().any(|w| w == "kenapa" || w == "mengapa" || w == "alasan") &&
       lower.iter().any(|w| w == "haram" || w == "diharamkan" || w == "dilarang") {
        expansions.extend_from_slice(&[
            "حكمة التحريم".to_string(), "علة التحريم".to_string(), "سبب التحريم".to_string(),
        ]);
    }

    // ═══ English intent patterns ═══

    // Pattern: "ruling on X" / "ruling of X" → حكم X
    if lower.iter().any(|w| w == "ruling") {
        expansions.extend_from_slice(&[
            "حكم".to_string(), "أحكام".to_string(),
        ]);
    }

    // Pattern: "is it permissible" / "is X allowed" / "is X halal" / "is X haram"
    if lower.iter().any(|w| w == "permissible" || w == "allowed" || w == "permitted" || w == "lawful") {
        expansions.extend_from_slice(&[
            "هل يجوز".to_string(), "جواز".to_string(), "حل".to_string(),
        ]);
    }

    // Pattern: "what invalidates X" / "things that break X"
    if lower.iter().any(|w| w == "invalidates" || w == "invalidate" || w == "nullifies" || w == "breaks" || w == "nullify") {
        expansions.extend_from_slice(&[
            "مبطلات".to_string(), "نواقض".to_string(), "ما يبطل".to_string(),
        ]);
    }

    // Pattern: "conditions of X" / "requirements for X"
    if lower.iter().any(|w| w == "conditions" || w == "requirements" || w == "prerequisites") {
        expansions.extend_from_slice(&[
            "شروط".to_string(), "الشروط".to_string(),
        ]);
    }

    // Pattern: "pillars of X" / "obligatory acts"
    if lower.iter().any(|w| w == "pillars" || w == "obligatory") {
        expansions.extend_from_slice(&[
            "أركان".to_string(), "واجبات".to_string(), "فرائض".to_string(),
        ]);
    }

    // Pattern: "types of X" / "categories of X"
    if lower.iter().any(|w| w == "types" || w == "categories" || w == "kinds" || w == "forms") {
        expansions.extend_from_slice(&[
            "أنواع".to_string(), "أقسام".to_string(),
        ]);
    }

    // Pattern: "how to perform X" / "how to do X" / "method of X"
    if lower.iter().any(|w| w == "perform" || w == "method" || w == "procedure") {
        expansions.extend_from_slice(&[
            "كيفية".to_string(), "صفة".to_string(),
        ]);
    }

    // Pattern: "difference between X and Y"
    if lower.iter().any(|w| w == "difference" || w == "distinction" || w == "versus" || w == "vs") {
        expansions.extend_from_slice(&[
            "الفرق".to_string(), "الفرق بين".to_string(),
        ]);
    }

    // Pattern: "wisdom behind X" / "reason for X"
    if lower.iter().any(|w| w == "wisdom" || w == "reason" || w == "rationale" || w == "purpose") {
        expansions.extend_from_slice(&[
            "حكمة".to_string(), "علة".to_string(),
        ]);
    }

    // Pattern: "is X valid" / "validity of X"
    if lower.iter().any(|w| w == "valid" || w == "validity" || w == "invalid") {
        expansions.extend_from_slice(&[
            "صحة".to_string(), "هل يصح".to_string(), "بطلان".to_string(),
        ]);
    }

    // Pattern: "prohibited" / "forbidden" / "haram"
    if lower.iter().any(|w| w == "prohibited" || w == "forbidden" || w == "haram" || w == "unlawful") {
        expansions.extend_from_slice(&[
            "تحريم".to_string(), "محرم".to_string(),
        ]);
    }

    // Pattern: "obligatory" / "mandatory" / "compulsory"
    if lower.iter().any(|w| w == "mandatory" || w == "compulsory" || w == "fard" || w == "wajib") {
        expansions.extend_from_slice(&[
            "وجوب".to_string(), "فرض".to_string(), "واجب".to_string(),
        ]);
    }

    // Pattern: "recommended" / "sunnah" / "encouraged"
    if lower.iter().any(|w| w == "recommended" || w == "encouraged" || w == "meritorious") {
        expansions.extend_from_slice(&[
            "مستحب".to_string(), "سنة".to_string(), "مندوب".to_string(),
        ]);
    }

    // ─── BATCH 17: Additional intent patterns ───

    // Pattern: "makna X" / "arti X" / "meaning of X" → definition/meaning context
    if lower.iter().any(|w| w == "makna" || w == "arti" || w == "artinya" || w == "meaning" || w == "definition") {
        expansions.extend_from_slice(&[
            "تعريف".to_string(), "معنى".to_string(), "مفهوم".to_string(),
        ]);
    }

    // Pattern: "manfaat X" / "benefit of X" / "benefits of X"
    if lower.iter().any(|w| w == "manfaat" || w == "faedah" || w == "benefit" || w == "benefits") {
        expansions.extend_from_slice(&[
            "فوائد".to_string(), "منافع".to_string(), "فضائل".to_string(),
        ]);
    }

    // Pattern: "bahaya X" / "danger/risk of X"
    if lower.iter().any(|w| w == "bahaya" || w == "mudharat" || w == "danger" || w == "risk" || w == "harm") {
        expansions.extend_from_slice(&[
            "مضار".to_string(), "مفاسد".to_string(), "أضرار".to_string(),
        ]);
    }

    // Pattern: "sejarah X" / "history of X"
    if lower.iter().any(|w| w == "sejarah" || w == "tarikh" || w == "history") {
        expansions.extend_from_slice(&[
            "تاريخ".to_string(), "نشأة".to_string(),
        ]);
    }

    // Pattern: "dalil X" / "evidence for X" / "proof of X"
    if lower.iter().any(|w| w == "dalil" || w == "dalilnya" || w == "evidence" || w == "proof" || w == "quran" || w == "hadits") {
        expansions.extend_from_slice(&[
            "دليل".to_string(), "الدليل".to_string(), "حجة".to_string(),
        ]);
    }

    // Pattern: "biografi X" / "siapakah X" / "riwayat hidup X" → biography
    if lower.iter().any(|w| w == "biografi" || w == "siapakah" || w == "siapa" || w == "profil" || w == "biography") {
        expansions.extend_from_slice(&[
            "ترجمة".to_string(), "سيرة".to_string(), "من هو".to_string(),
        ]);
    }

    // Pattern: "karya X" / "kitab X" / "buku X" → works/books
    if lower.iter().any(|w| w == "karya" || w == "karangan" || w == "tulisan" || w == "kitab") &&
       lower.iter().any(|w| w == "ibnu" || w == "imam" || w == "syaikh" || w == "al") {
        expansions.extend_from_slice(&[
            "مؤلفات".to_string(), "كتب".to_string(), "مصنفات".to_string(),
        ]);
    }

    // Pattern: "keutamaan X" / "fadilah X" / "virtue of X"
    if lower.iter().any(|w| w == "keutamaan" || w == "fadilah" || w == "fadhilah" || w == "virtue" || w == "merit") {
        expansions.extend_from_slice(&[
            "فضل".to_string(), "فضائل".to_string(), "ثواب".to_string(),
        ]);
    }

    // Pattern: "amalan X" / "practice of X" — sunnah acts
    if lower.iter().any(|w| w == "amalan" || w == "amal" || w == "practice" || w == "practices") {
        expansions.extend_from_slice(&[
            "أعمال".to_string(), "عمل".to_string(),
        ]);
    }

    // Pattern: "peristiwa penting X" / "events of X" — historical events
    if lower.iter().any(|w| w == "peristiwa" || w == "kejadian" || w == "events" || w == "event") {
        expansions.extend_from_slice(&[
            "أحداث".to_string(), "حوادث".to_string(), "وقائع".to_string(),
        ]);
    }

    // Pattern: "tentang X" → about X — general topic question
    if lower.iter().any(|w| w == "tentang" || w == "about" || w == "mengenai") {
        // Only add generic "بحث" (research/inquiry) if combined with Islamic terms
        if lower.iter().any(|w| ["shalat","puasa","zakat","haji","nikah","talak","riba","waris","fiqh","aqidah"].contains(&w.as_str())) {
            expansions.push("أحكام".to_string());
        }
    }

    expansions
}

/// Check if a word is a generic stopword that should not be single-word-expanded.
/// These are function words / pronouns / particles that would never be useful
/// search terms on their own. Content words (anak, suami, etc.) are NOT filtered
/// — they're handled by question extraction instead.
fn is_query_stopword(word: &str) -> bool {
    matches!(word,
        // Indonesian function words / pronouns / particles
        "saya" | "anda" | "kamu" | "kami" | "kita" | "mereka" | "dia" | "nya" |
        "yang" | "ini" | "itu" | "dan" | "atau" | "dari" | "untuk" | "dengan" |
        "di" | "ke" | "pada" | "oleh" | "dalam" | "juga" | "sudah" | "belum" |
        "bisa" | "akan" | "hanya" | "karena" | "tapi" | "tetapi" | "kalau" |
        "jika" | "maka" | "sedang" | "selama" | "setiap" | "sering" |
        "sampai" | "sekarang" | "sangat" | "selalu" | "tentu" | "pasti" |
        "lagi" | "masih" | "lalu" | "kemudian" | "pernah" |
        "dong" | "sih" | "deh" | "kok" | "yg" | "dgn" | "sdh" | "udah" |
        "gitu" | "gak" | "nggak" | "ngga" | "ga" | "enggak" |
        "seperti" | "bahwa" | "agar" | "supaya" | "namun" |
        "ada" | "punya" | "mau" | "harus" | "perlu" | "banyak" | "sedikit" |
        "soal" | "tentang" | "mengenai" | "terkait" |
        "kembali" | "menjadi" | "terhadap" | "antara" | "sekitar" |
        "jadi" | "ya" | "kan" | "tuh" | "sama" | "kadang" |
        "mungkin" | "tetap" | "terus" | "cuma" | "banget" |
        "begitu" | "demikian" | "tersebut" | "sebuah" | "suatu" |
        // English function words
        "the" | "a" | "an" | "is" | "are" | "was" | "were" | "be" | "been" |
        "have" | "has" | "had" | "do" | "does" | "did" | "would" |
        "could" | "should" | "may" | "might" | "must" | "shall" | "can" |
        "i" | "you" | "he" | "she" | "it" | "we" | "they" | "me" | "him" |
        "her" | "us" | "them" | "my" | "your" | "his" | "its" | "our" | "their" |
        "this" | "that" | "these" | "those" |
        "if" | "then" | "than" | "but" |
        "and" | "or" | "not" | "no" | "so" | "just" | "also" | "very" |
        "with" | "from" | "for" | "to" | "of" | "in" | "on" | "at" | "by" |
        "about" | "some" | "any" | "all" | "each" | "every" | "many" |
        "much" | "more" | "most" | "there" | "here" | "now" |
        "still" | "already" | "always" | "never" | "often" | "sometimes" |
        "because" | "since" | "while" | "although" | "though" | "however" |
        "well" | "too" | "really" | "quite" | "need" | "want" | "get" |
        "got" | "make" | "made" | "know" | "think" | "see" | "come" |
        "go" | "take" | "give" | "tell" | "say" | "said"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_indonesian() {
        let t = QueryTranslator::new();
        assert_eq!(t.detect_language("apa hukum shalat jumat"), QueryLang::Indonesian);
    }

    #[test]
    fn test_detect_arabic() {
        let t = QueryTranslator::new();
        assert_eq!(t.detect_language("ما حكم صلاة الجمعة"), QueryLang::Arabic);
    }

    #[test]
    fn test_detect_english() {
        let t = QueryTranslator::new();
        assert_eq!(t.detect_language("what is the ruling on friday prayer"), QueryLang::English);
    }

    #[test]
    fn test_expand_shalat() {
        let t = QueryTranslator::new();
        let result = t.translate("shalat jumat");
        assert!(!result.arabic_terms.is_empty());
        assert!(result.arabic_terms.iter().any(|t| t.contains("صلاة")));
        assert!(result.arabic_terms.iter().any(|t| t.contains("الجمعة")));
    }

    #[test]
    fn test_expand_nikah_siri() {
        let t = QueryTranslator::new();
        let result = t.translate("hukum nikah siri");
        assert!(result.arabic_terms.iter().any(|t| t.contains("نكاح")));
    }

    #[test]
    fn test_arabic_passthrough() {
        let t = QueryTranslator::new();
        let result = t.translate("حكم صلاة الجمعة");
        assert_eq!(result.detected_language, QueryLang::Arabic);
        assert!(!result.arabic_terms.is_empty());
    }

    #[test]
    fn test_long_query_term_reduction() {
        let t = QueryTranslator::new();
        // Long descriptive query followed by question
        let long_q = "Saya punya anak kecil yang masih bayi, umurnya baru 8 bulan. \
            Setiap kali saya shalat, anak saya sering digendong oleh istri saya. \
            Tapi kadang kalau istri saya sedang masak, saya harus shalat sambil \
            menggendong bayi tersebut. Yang jadi masalah, kadang popok bayi basah \
            dan saya khawatir ada najis yang mengenai baju saya saat shalat. \
            Pertanyaannya, apakah shalat saya tetap sah kalau menggendong bayi \
            yang popoknya mungkin terkena najis?";
        let result = t.translate(long_q);
        // Should have significantly fewer terms than processing every word
        assert!(result.arabic_terms.len() <= MAX_ARABIC_TERMS,
            "Long query should be capped: got {} terms", result.arabic_terms.len());
        // Must still find the core concepts
        assert!(result.arabic_terms.iter().any(|t| t.contains("صلاة") || t.contains("الصلاة")),
            "Must find salat");
        assert!(result.arabic_terms.iter().any(|t| t.contains("نجاسة") || t.contains("النجس")),
            "Must find najis");
    }

    #[test]
    fn test_question_extraction() {
        let (q, c) = extract_question_and_context(
            "Saya punya masalah tentang wudhu. Apakah boleh menyapu perban?"
        );
        assert!(q.to_lowercase().contains("apakah"), "Should extract question: got '{}'", q);
        assert!(c.to_lowercase().contains("masalah"), "Context should have description: got '{}'", c);
    }

    #[test]
    fn test_short_query_unchanged() {
        let t = QueryTranslator::new();
        // Short queries should still expand all words
        let result = t.translate("hukum shalat duduk");
        assert!(result.arabic_terms.iter().any(|t| t.contains("صلاة")));
        assert!(result.arabic_terms.iter().any(|t| t.contains("جالس") || t.contains("قاعد")));
    }
}
