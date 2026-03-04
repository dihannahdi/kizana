//! Arabic Light Stemmer & Normalizer for Islamic Text IR
//!
//! Implements Arabic text normalization and light stemming for improved
//! cross-lingual information retrieval in classical Islamic texts (turats).
//!
//! Based on:
//! - Larkey, Ballesteros & Connell (2007), "Light Stemming for Arabic IR"
//! - ISRI stemmer (Taghva et al., 2005)
//! - Custom extensions for classical Arabic (تراث) patterns
//!
//! This module provides three levels of text processing:
//! 1. Normalization — removes diacritics, normalizes letter variants
//! 2. Light stemming — removes definite article and common affixes
//! 3. Query expansion — generates morphological variants for OR-search

/// Arabic diacritical marks (tashkeel/harakat) — Unicode range
const TASHKEEL: &[char] = &[
    '\u{064B}', // FATHATAN  ً
    '\u{064C}', // DAMMATAN  ٌ
    '\u{064D}', // KASRATAN  ٍ
    '\u{064E}', // FATHA     َ
    '\u{064F}', // DAMMA     ُ
    '\u{0650}', // KASRA     ِ
    '\u{0651}', // SHADDA    ّ
    '\u{0652}', // SUKUN     ْ
    '\u{0653}', // MADDAH ABOVE
    '\u{0654}', // HAMZA ABOVE
    '\u{0655}', // HAMZA BELOW
    '\u{0670}', // SUPERSCRIPT ALEF
];

/// Tatweel (kashida) character — used for text justification
const TATWEEL: char = '\u{0640}'; // ـ

pub struct ArabicStemmer;

impl ArabicStemmer {
    pub fn new() -> Self {
        ArabicStemmer
    }

    /// Strip tashkeel (diacritical marks) from Arabic text
    /// This is the safest normalization — preserves all consonants
    pub fn strip_tashkeel(&self, text: &str) -> String {
        text.chars()
            .filter(|c| !TASHKEEL.contains(c) && *c != TATWEEL)
            .collect()
    }

    /// Normalize Arabic text for consistent matching
    /// Applies safe transformations that increase recall without hurting precision
    pub fn normalize(&self, text: &str) -> String {
        let mut result = String::with_capacity(text.len());

        for ch in text.chars() {
            // Skip tashkeel (diacritics)
            if TASHKEEL.contains(&ch) {
                continue;
            }
            // Skip tatweel (kashida)
            if ch == TATWEEL {
                continue;
            }

            // Normalize hamza-bearing alef variants to plain alef
            let normalized = match ch {
                '\u{0622}' => '\u{0627}', // آ  ALEF WITH MADDA → ا
                '\u{0623}' => '\u{0627}', // أ  ALEF WITH HAMZA ABOVE → ا
                '\u{0625}' => '\u{0627}', // إ  ALEF WITH HAMZA BELOW → ا
                '\u{0671}' => '\u{0627}', // ٱ  ALEF WASLA → ا
                '\u{0649}' => '\u{064A}', // ى  ALEF MAKSURA → ي
                other => other,
            };

            result.push(normalized);
        }

        result
    }

    /// Remove the definite article ال and common prefixes
    /// Returns the word without prefix
    pub fn remove_prefix(&self, word: &str) -> String {
        let chars: Vec<char> = word.chars().collect();
        let len = chars.len();

        if len < 3 {
            return word.to_string();
        }

        // Remove الـ (definite article al-)
        if len > 3 && chars[0] == '\u{0627}' && chars[1] == '\u{0644}' {
            return chars[2..].iter().collect();
        }

        // Remove و/ف/ب/ك + ال (conjunction/preposition + al-)
        if len > 4
            && ['\u{0648}', '\u{0641}', '\u{0628}', '\u{0643}'].contains(&chars[0])
            && chars[1] == '\u{0627}'
            && chars[2] == '\u{0644}'
        {
            return chars[3..].iter().collect();
        }

        // Remove single-letter prefixes: و، ف، ب، ك، ل
        if len > 3
            && [
                '\u{0648}', '\u{0641}', '\u{0628}', '\u{0643}', '\u{0644}',
            ]
            .contains(&chars[0])
        {
            return chars[1..].iter().collect();
        }

        word.to_string()
    }

    /// Remove common Arabic suffixes for light stemming
    pub fn remove_suffix(&self, word: &str) -> String {
        let chars: Vec<char> = word.chars().collect();
        let len = chars.len();

        if len < 3 {
            return word.to_string();
        }

        // 3-char suffixes (check first for longest match)
        if len > 5 {
            let s3: String = chars[len - 3..].iter().collect();
            // تهم، تهن، ناه، كها etc.
            if matches!(
                s3.as_str(),
                "ات\u{0647}" | "وه\u{0627}" | "ته\u{0627}" | "يات"
            ) {
                return chars[..len - 3].iter().collect();
            }
        }

        // 2-char suffixes
        if len > 4 {
            let s2: String = chars[len - 2..].iter().collect();
            match s2.as_str() {
                "ات" | // feminine plural  ات
                "ين" | // masculine plural accusative/genitive  ين
                "ون" | // masculine plural nominative  ون
                "ان" | // dual  ان
                "هم" | // their (m)  هم
                "هن" | // their (f)  هن
                "كم" | // your (pl)  كم
                "نا" | // our  نا
                "ها" | // her/its  ها
                "ية" | // nisba  ية
                "يه"   // nisba variant  يه
                => {
                    return chars[..len - 2].iter().collect();
                }
                _ => {}
            }
        }

        // 1-char suffixes
        if len > 3 {
            let last = chars[len - 1];
            match last {
                '\u{0629}' => {
                    // ة ta marbuta → remove
                    return chars[..len - 1].iter().collect();
                }
                '\u{0647}' | // ه ha
                '\u{064A}'   // ي ya
                => {
                    return chars[..len - 1].iter().collect();
                }
                _ => {}
            }
        }

        word.to_string()
    }

    /// Full light stem pipeline: normalize → remove prefix → remove suffix
    pub fn light_stem(&self, word: &str) -> String {
        let normalized = self.normalize(word);
        let no_prefix = self.remove_prefix(&normalized);
        self.remove_suffix(&no_prefix)
    }

    /// Generate morphological variants of an Arabic word for query expansion.
    /// 
    /// This is the key function for improving recall: for each Arabic search term,
    /// generate alternative forms that might appear in the corpus.
    /// Returns a Vec of variants INCLUDING the original word.
    pub fn expand_variants(&self, word: &str) -> Vec<String> {
        let mut variants = Vec::with_capacity(6);

        // 1. Always include original
        variants.push(word.to_string());

        // 2. Strip tashkeel (if word has diacritics)
        let no_tashkeel = self.strip_tashkeel(word);
        if no_tashkeel != word {
            variants.push(no_tashkeel.clone());
        }

        // 3. Normalize (hamza variants, alef maksura)
        let normalized = self.normalize(word);
        if normalized != word && normalized != no_tashkeel {
            variants.push(normalized.clone());
        }

        // 4. With/without definite article
        let base = if normalized.is_empty() {
            &no_tashkeel
        } else {
            &normalized
        };
        let chars: Vec<char> = base.chars().collect();

        if chars.len() >= 2 && chars[0] == '\u{0627}' && chars[1] == '\u{0644}' {
            // Has ال → add without
            let without_al: String = chars[2..].iter().collect();
            if without_al.chars().count() >= 2 && !variants.contains(&without_al) {
                variants.push(without_al);
            }
        } else if chars.len() >= 2 {
            // Doesn't have ال → add with
            let with_al = format!("\u{0627}\u{0644}{}", base);
            if !variants.contains(&with_al) {
                variants.push(with_al);
            }
        }

        // 5. Ta marbuta ↔ ha normalization
        //    e.g., صلاة → صلاه and vice versa
        if let Some(last) = chars.last() {
            if *last == '\u{0629}' {
                // ة → ه
                let mut alt: String = chars[..chars.len() - 1].iter().collect();
                alt.push('\u{0647}');
                if !variants.contains(&alt) {
                    variants.push(alt);
                }
            } else if *last == '\u{0647}' {
                // ه → ة
                let mut alt: String = chars[..chars.len() - 1].iter().collect();
                alt.push('\u{0629}');
                if !variants.contains(&alt) {
                    variants.push(alt);
                }
            }
        }

        // 6. Light stem (aggressive — only include if result is 3+ chars)
        let stemmed = self.light_stem(word);
        if stemmed.chars().count() >= 3
            && stemmed != word
            && !variants.contains(&stemmed)
        {
            variants.push(stemmed);
        }

        variants
    }

    /// Expand all Arabic terms in a query for improved recall
    /// Used at query time to broaden search without re-indexing
    pub fn expand_query_terms(&self, terms: &[String]) -> Vec<String> {
        let mut expanded = Vec::new();
        for term in terms {
            // Only expand single Arabic words, not phrases
            if term.contains(' ') {
                expanded.push(term.clone());
                continue;
            }
            // Check if term contains Arabic characters
            if term.chars().any(|c| is_arabic_char(c)) {
                let variants = self.expand_variants(term);
                expanded.extend(variants);
            } else {
                expanded.push(term.clone());
            }
        }
        expanded.sort();
        expanded.dedup();
        expanded
    }
}

/// Check if a character is in the Arabic Unicode block
fn is_arabic_char(c: char) -> bool {
    matches!(c as u32,
        0x0600..=0x06FF |  // Arabic
        0x0750..=0x077F |  // Arabic Supplement
        0x08A0..=0x08FF |  // Arabic Extended-A
        0xFB50..=0xFDFF |  // Arabic Presentation Forms-A
        0xFE70..=0xFEFF    // Arabic Presentation Forms-B
    )
}

// ─── Tests ───

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_tashkeel() {
        let s = ArabicStemmer::new();
        // الصَّلَاة → الصلاة
        assert_eq!(s.strip_tashkeel("الصَّلَاة"), "الصلاة");
        // وَضُوء → وضوء
        assert_eq!(s.strip_tashkeel("وَضُوء"), "وضوء");
    }

    #[test]
    fn test_normalize_hamza() {
        let s = ArabicStemmer::new();
        // أحكام → احكام
        assert_eq!(s.normalize("أحكام"), "احكام");
        // إسلام → اسلام
        assert_eq!(s.normalize("إسلام"), "اسلام");
        // آية → اية
        assert_eq!(s.normalize("آية"), "اية");
    }

    #[test]
    fn test_normalize_alef_maksura() {
        let s = ArabicStemmer::new();
        // فتوى → فتوي
        assert_eq!(s.normalize("فتوى"), "فتوي");
    }

    #[test]
    fn test_remove_prefix_definite_article() {
        let s = ArabicStemmer::new();
        let normalized = s.normalize("الصلاة");
        let result = s.remove_prefix(&normalized);
        assert_eq!(result, "صلاة");
    }

    #[test]
    fn test_remove_prefix_conjunction_al() {
        let s = ArabicStemmer::new();
        let normalized = s.normalize("والصلاة");
        let result = s.remove_prefix(&normalized);
        assert_eq!(result, "صلاة");
    }

    #[test]
    fn test_remove_suffix_feminine_plural() {
        let s = ArabicStemmer::new();
        let result = s.remove_suffix("صلوات");
        assert_eq!(result, "صلو");
    }

    #[test]
    fn test_remove_suffix_ta_marbuta() {
        let s = ArabicStemmer::new();
        let result = s.remove_suffix("صلاة");
        assert_eq!(result, "صلا");
    }

    #[test]
    fn test_light_stem_full_pipeline() {
        let s = ArabicStemmer::new();
        let stemmed = s.light_stem("الصلوات");
        // Should remove ال prefix and ات suffix
        assert!(stemmed.chars().count() < "الصلوات".chars().count());
    }

    #[test]
    fn test_expand_variants() {
        let s = ArabicStemmer::new();
        let variants = s.expand_variants("صلاة");
        assert!(variants.contains(&"صلاة".to_string()));
        // Should include variant with ال
        assert!(variants.iter().any(|v| v.starts_with("ال")));
        assert!(variants.len() >= 3);
    }

    #[test]
    fn test_expand_definite_article() {
        let s = ArabicStemmer::new();
        let variants = s.expand_variants("الصلاة");
        // Should include without ال
        assert!(variants.contains(&"الصلاة".to_string()));
        assert!(variants.iter().any(|v| !v.starts_with("ال")));
    }

    #[test]
    fn test_expand_query_terms_mixed() {
        let s = ArabicStemmer::new();
        let terms = vec!["صلاة".to_string(), "shalat".to_string(), "صلاة الجمعة".to_string()];
        let expanded = s.expand_query_terms(&terms);
        // Should expand Arabic single words
        assert!(expanded.len() > 3);
        // Should preserve Latin terms
        assert!(expanded.contains(&"shalat".to_string()));
        // Should preserve phrases as-is
        assert!(expanded.contains(&"صلاة الجمعة".to_string()));
    }
}
