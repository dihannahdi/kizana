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
    domain_keywords: HashMap<String, FiqhDomain>,
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
                } else {
                    // ═══ SHORT QUERY MODE (original behavior) ═══
                    let words = tokenize_query(&query);
                    for word in &words {
                        let lower = word.to_lowercase();
                        latin_terms.push(lower.clone());

                        // Check single-word expansion
                        if let Some(expansions) = self.term_map.get(&lower) {
                            for exp in expansions {
                                arabic_terms.push(exp.to_string());
                            }
                        }
                    }

                    // Check multi-word phrases (bigrams, trigrams)
                    let phrase_expansions = self.expand_phrases(&words);
                    for exp in phrase_expansions {
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
        // Prioritize multi-word phrases (more specific) over single words
        if arabic_terms.len() > MAX_ARABIC_TERMS {
            let phrases: Vec<String> = arabic_terms.iter()
                .filter(|t| t.contains(' '))
                .cloned()
                .collect();
            let singles: Vec<String> = arabic_terms.iter()
                .filter(|t| !t.contains(' '))
                .cloned()
                .collect();
            arabic_terms.clear();
            // Keep all phrases first (up to limit)
            for p in phrases.into_iter().take(MAX_ARABIC_TERMS) {
                arabic_terms.push(p);
            }
            // Fill remaining slots with single-word terms
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

        // Apply term limiting
        if arabic_terms.len() > MAX_ARABIC_TERMS {
            let phrases: Vec<String> = arabic_terms.iter()
                .filter(|t| t.contains(' '))
                .cloned()
                .collect();
            let singles: Vec<String> = arabic_terms.iter()
                .filter(|t| !t.contains(' '))
                .cloned()
                .collect();
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

        for (keyword, domain) in &self.domain_keywords {
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
                    FiqhDomain::Unknown => "unknown",
                };
                *domain_scores.entry(domain_key).or_default() += 1;
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
            self.term_map.insert(key.to_string(), vec!["الاستنساخ"]);
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
    }

    fn build_domain_detector(&mut self) {
        // Indonesian keywords → domain
        let domain_map: Vec<(&str, FiqhDomain)> = vec![
            // Ibadah
            ("shalat", FiqhDomain::Ibadah), ("solat", FiqhDomain::Ibadah),
            ("puasa", FiqhDomain::Ibadah), ("zakat", FiqhDomain::Ibadah),
            ("haji", FiqhDomain::Ibadah), ("umroh", FiqhDomain::Ibadah),
            ("prayer", FiqhDomain::Ibadah), ("fasting", FiqhDomain::Ibadah),
            ("doa", FiqhDomain::Ibadah), ("dzikir", FiqhDomain::Ibadah),
            ("adzan", FiqhDomain::Ibadah), ("qurban", FiqhDomain::Ibadah),
            ("itikaf", FiqhDomain::Ibadah),
            ("kafarat", FiqhDomain::Ibadah), ("kaffarah", FiqhDomain::Ibadah),
            ("sumpah", FiqhDomain::Ibadah), ("nadzar", FiqhDomain::Ibadah),
            ("aqiqah", FiqhDomain::Ibadah), ("fidyah", FiqhDomain::Ibadah),
            // Thaharah
            ("wudhu", FiqhDomain::Thaharah), ("tayammum", FiqhDomain::Thaharah),
            ("najis", FiqhDomain::Thaharah), ("haid", FiqhDomain::Thaharah),
            ("junub", FiqhDomain::Thaharah), ("mandi", FiqhDomain::Thaharah),
            ("suci", FiqhDomain::Thaharah), ("bersuci", FiqhDomain::Thaharah),
            ("nifas", FiqhDomain::Thaharah), ("istihadhah", FiqhDomain::Thaharah),
            // Muamalat
            ("riba", FiqhDomain::Muamalat), ("jual", FiqhDomain::Muamalat),
            ("beli", FiqhDomain::Muamalat), ("utang", FiqhDomain::Muamalat),
            ("gadai", FiqhDomain::Muamalat), ("sewa", FiqhDomain::Muamalat),
            ("bank", FiqhDomain::Muamalat), ("bunga", FiqhDomain::Muamalat),
            ("asuransi", FiqhDomain::Muamalat), ("waris", FiqhDomain::Muamalat),
            ("warisan", FiqhDomain::Muamalat), ("wakaf", FiqhDomain::Muamalat),
            ("trade", FiqhDomain::Muamalat), ("finance", FiqhDomain::Muamalat),
            // Munakahat
            ("nikah", FiqhDomain::Munakahat), ("cerai", FiqhDomain::Munakahat),
            ("talak", FiqhDomain::Munakahat), ("mahar", FiqhDomain::Munakahat),
            ("nafkah", FiqhDomain::Munakahat), ("iddah", FiqhDomain::Munakahat),
            ("poligami", FiqhDomain::Munakahat), ("suami", FiqhDomain::Munakahat),
            ("istri", FiqhDomain::Munakahat), ("marriage", FiqhDomain::Munakahat),
            ("divorce", FiqhDomain::Munakahat), ("khuluk", FiqhDomain::Munakahat),
            ("impoten", FiqhDomain::Munakahat), ("wali", FiqhDomain::Munakahat),
            // Jinayat
            ("pencurian", FiqhDomain::Jinayat), ("hudud", FiqhDomain::Jinayat),
            ("qishash", FiqhDomain::Jinayat), ("zina", FiqhDomain::Jinayat),
            ("bunuh", FiqhDomain::Jinayat), ("hukuman", FiqhDomain::Jinayat),
            // Aqidah
            ("tauhid", FiqhDomain::Aqidah), ("syirik", FiqhDomain::Aqidah),
            ("iman", FiqhDomain::Aqidah), ("kafir", FiqhDomain::Aqidah),
            ("bid'ah", FiqhDomain::Aqidah), ("bidah", FiqhDomain::Aqidah),
            ("murtad", FiqhDomain::Aqidah), ("aqidah", FiqhDomain::Aqidah),
            // Tasawuf
            ("tasawuf", FiqhDomain::Tasawuf), ("taubat", FiqhDomain::Tasawuf),
            ("ikhlas", FiqhDomain::Tasawuf), ("riya", FiqhDomain::Tasawuf),
            // Tafsir
            ("tafsir", FiqhDomain::Tafsir), ("ayat", FiqhDomain::Tafsir),
            ("surat", FiqhDomain::Tafsir), ("quran", FiqhDomain::Tafsir),
            // Hadits
            ("hadits", FiqhDomain::Hadits), ("hadis", FiqhDomain::Hadits),
            ("hadith", FiqhDomain::Hadits), ("sanad", FiqhDomain::Hadits),
            // Ibadah - expanded
            ("iftar", FiqhDomain::Ibadah), ("wajib", FiqhDomain::Ibadah),
            ("sunnah", FiqhDomain::Ibadah), ("fardhu", FiqhDomain::Ibadah),
            ("batal", FiqhDomain::Ibadah), ("rukun", FiqhDomain::Ibadah),
            ("qadha", FiqhDomain::Ibadah),
            ("vaksin", FiqhDomain::Ibadah), ("vaccine", FiqhDomain::Ibadah),
            ("imunisasi", FiqhDomain::Ibadah),
            ("sakit", FiqhDomain::Ibadah), ("sick", FiqhDomain::Ibadah),
            ("penyakit", FiqhDomain::Ibadah), ("darurat", FiqhDomain::Ibadah),
            ("makan", FiqhDomain::Ibadah), ("minum", FiqhDomain::Ibadah),
            ("subuh", FiqhDomain::Ibadah), ("fajar", FiqhDomain::Ibadah),
            ("jenazah", FiqhDomain::Ibadah), ("janazah", FiqhDomain::Ibadah),
            ("gelatin", FiqhDomain::Ibadah), ("babi", FiqhDomain::Ibadah),
            ("pork", FiqhDomain::Ibadah),
            ("dzuhur", FiqhDomain::Ibadah), ("ashar", FiqhDomain::Ibadah),
            ("maghrib", FiqhDomain::Ibadah), ("isya", FiqhDomain::Ibadah),
            ("qunut", FiqhDomain::Ibadah), ("witir", FiqhDomain::Ibadah),
            ("rakaat", FiqhDomain::Ibadah), ("rakat", FiqhDomain::Ibadah),
            ("mati", FiqhDomain::Ibadah), ("meninggal", FiqhDomain::Ibadah),
            ("wafat", FiqhDomain::Ibadah), ("mayit", FiqhDomain::Ibadah),
            ("taubat", FiqhDomain::Tasawuf), ("tobat", FiqhDomain::Tasawuf),
            // Muamalat - expanded
            ("foto", FiqhDomain::Muamalat), ("gambar", FiqhDomain::Muamalat),
            ("tato", FiqhDomain::Muamalat), ("musik", FiqhDomain::Muamalat),
            ("rokok", FiqhDomain::Muamalat), ("merokok", FiqhDomain::Muamalat),
            ("photo", FiqhDomain::Muamalat), ("picture", FiqhDomain::Muamalat),
            ("selfie", FiqhDomain::Muamalat),
            ("konvensional", FiqhDomain::Muamalat),
            // Munakahat - expanded
            ("nusyuz", FiqhDomain::Munakahat), ("nushuz", FiqhDomain::Munakahat),
            ("perempuan", FiqhDomain::Munakahat),
            ("fasakh", FiqhDomain::Munakahat), ("fasak", FiqhDomain::Munakahat),
            // Aqidah - expanded
            ("syirik", FiqhDomain::Aqidah), ("tawasul", FiqhDomain::Aqidah),
            ("khurafat", FiqhDomain::Aqidah),
            ("tawakal", FiqhDomain::Aqidah), ("tawakkal", FiqhDomain::Aqidah),
            ("tawakkul", FiqhDomain::Aqidah),
            // Jinayat - expanded
            ("mencuri", FiqhDomain::Jinayat), ("curi", FiqhDomain::Jinayat),
            // Muamalat - more
            ("asuransi", FiqhDomain::Muamalat), ("gadai", FiqhDomain::Muamalat),
            ("emas", FiqhDomain::Muamalat), ("pegadaian", FiqhDomain::Muamalat),
            ("online", FiqhDomain::Muamalat), ("nyanyian", FiqhDomain::Muamalat),
        ];

        for (keyword, domain) in domain_map {
            self.domain_keywords.insert(keyword.to_string(), domain);
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
