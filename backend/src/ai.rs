use crate::config::Config;
use crate::query_translator::{QueryLang, TranslatedQuery};
use log::{warn, error, info};
use reqwest::Client;
use serde_json::json;
use futures::StreamExt;
use futures::SinkExt;

pub type SseItem = Result<actix_web::web::Bytes, std::io::Error>;

#[derive(Clone)]
pub struct AiClient {
    client: Client,
    api_url: String,
    api_key: String,
    model: String,
}

impl AiClient {
    pub fn new(config: &Config) -> Self {
        AiClient {
            client: Client::new(),
            api_url: config.ai_api_url.clone(),
            api_key: config.ai_api_key.clone(),
            model: config.ai_model.clone(),
        }
    }

    pub async fn synthesize_answer(
        &self,
        query: &str,
        results: &[crate::models::SearchResult],
        translated: Option<&TranslatedQuery>,
    ) -> Result<String, String> {
        if self.api_key.is_empty() {
            return Ok(self.local_synthesis(query, results, translated));
        }

        // Detect language for response
        let lang = translated
            .map(|t| &t.detected_language)
            .unwrap_or(&QueryLang::Indonesian);

        // Build rich context from search results with book name + author
        let context: String = results
            .iter()
            .take(10)
            .enumerate()
            .map(|(i, r)| {
                let book_info = if !r.book_name.is_empty() && !r.author_name.is_empty() {
                    format!("📖 {} — {} (كتاب رقم {}, ص {})", r.book_name, r.author_name, r.book_id, r.page)
                } else if !r.book_name.is_empty() {
                    format!("📖 {} (كتاب رقم {}, ص {})", r.book_name, r.book_id, r.page)
                } else {
                    format!("📖 كتاب رقم {}, ص {}", r.book_id, r.page)
                };

                format!(
                    "{}. {}\nالعنوان: {}\nالنص: {}\n",
                    i + 1,
                    book_info,
                    r.title,
                    if r.content_snippet.is_empty() {
                        &r.title
                    } else {
                        &r.content_snippet
                    }
                )
            })
            .collect();

        let system_prompt = build_system_prompt(lang);
        let user_prompt = build_user_prompt(query, lang, &context, translated);

        let body = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "max_tokens": 3000,
            "temperature": 0.3
        });

        match self
            .client
            .post(&self.api_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(resp) => {
                if resp.status().is_success() {
                    let json: serde_json::Value =
                        resp.json().await.map_err(|e| e.to_string())?;
                    let answer = json["choices"][0]["message"]["content"]
                        .as_str()
                        .unwrap_or("لم يتم العثور على إجابة")
                        .to_string();
                    Ok(answer)
                } else {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    warn!("AI API error {}: {}", status, body);
                    Ok(self.local_synthesis(query, results, translated))
                }
            }
            Err(e) => {
                error!("AI API request failed: {}", e);
                Ok(self.local_synthesis(query, results, translated))
            }
        }
    }

    /// Stream AI synthesized answer via SSE. Returns the full accumulated text.
    /// Sends chunks to the provided sender as they arrive from the LLM.
    pub async fn synthesize_answer_stream(
        &self,
        query: &str,
        results: &[crate::models::SearchResult],
        translated: Option<&TranslatedQuery>,
        mut tx: futures::channel::mpsc::Sender<SseItem>,
    ) -> String {
        if self.api_key.is_empty() {
            let local = self.local_synthesis(query, results, translated);
            let chunk_json = serde_json::json!({"content": &local}).to_string();
            let _ = tx.send(Ok(actix_web::web::Bytes::from(
                format!("event: ai_chunk\ndata: {}\n\n", chunk_json)
            ))).await;
            return local;
        }

        let lang = translated
            .map(|t| &t.detected_language)
            .unwrap_or(&QueryLang::Indonesian);

        let context = build_rich_context(results);
        let system_prompt = build_system_prompt_stream(lang);
        let user_prompt = build_user_prompt(query, lang, &context, translated);

        let body = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "stream": true,
            "max_tokens": 4000,
            "temperature": 0.3
        });

        let resp = match self
            .client
            .post(&self.api_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                error!("AI stream request failed: {}", e);
                let fallback = self.local_synthesis(query, results, translated);
                let chunk_json = serde_json::json!({"content": &fallback}).to_string();
                let _ = tx.send(Ok(actix_web::web::Bytes::from(
                    format!("event: ai_chunk\ndata: {}\n\n", chunk_json)
                ))).await;
                return fallback;
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let body_text = resp.text().await.unwrap_or_default();
            warn!("AI stream API error {}: {}", status, body_text);
            let fallback = self.local_synthesis(query, results, translated);
            let chunk_json = serde_json::json!({"content": &fallback}).to_string();
            let _ = tx.send(Ok(actix_web::web::Bytes::from(
                format!("event: ai_chunk\ndata: {}\n\n", chunk_json)
            ))).await;
            return fallback;
        }

        let mut full_text = String::new();
        let mut buffer = String::new();
        let mut stream = resp.bytes_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk_bytes = match chunk_result {
                Ok(b) => b,
                Err(e) => {
                    warn!("Stream chunk error: {}", e);
                    break;
                }
            };

            let text = String::from_utf8_lossy(&chunk_bytes);
            buffer.push_str(&text);

            // Process complete SSE events (double newline separated)
            while let Some(pos) = buffer.find("\n\n") {
                let event_block = buffer[..pos].to_string();
                buffer = buffer[pos + 2..].to_string();

                for line in event_block.lines() {
                    let data = if let Some(d) = line.strip_prefix("data: ") {
                        d.trim()
                    } else {
                        continue;
                    };

                    if data == "[DONE]" {
                        continue;
                    }

                    if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(data) {
                        // Only accept delta.content in streaming mode — do NOT fall back
                        // to message.content, as the final SSE chunk may contain the full
                        // accumulated text, which would duplicate everything.
                        let content = json_val["choices"][0]["delta"]["content"]
                            .as_str();

                        if let Some(c) = content {
                            if !c.is_empty() {
                                full_text.push_str(c);
                                let chunk_json = serde_json::json!({"content": c}).to_string();
                                let _ = tx.send(Ok(actix_web::web::Bytes::from(
                                    format!("event: ai_chunk\ndata: {}\n\n", chunk_json)
                                ))).await;
                            }
                        }
                    }
                }
            }
        }

        // Process any remaining buffer
        if !buffer.trim().is_empty() {
            for line in buffer.lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    let data = data.trim();
                    if data != "[DONE]" {
                        if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(data) {
                            if let Some(c) = json_val["choices"][0]["delta"]["content"].as_str() {
                                if !c.is_empty() {
                                    full_text.push_str(c);
                                    let chunk_json = serde_json::json!({"content": c}).to_string();
                                    let _ = tx.send(Ok(actix_web::web::Bytes::from(
                                        format!("event: ai_chunk\ndata: {}\n\n", chunk_json)
                                    ))).await;
                                }
                            }
                        }
                    }
                }
            }
        }

        if full_text.is_empty() {
            info!("AI stream returned empty, using local synthesis");
            let fallback = self.local_synthesis(query, results, translated);
            let chunk_json = serde_json::json!({"content": &fallback}).to_string();
            let _ = tx.send(Ok(actix_web::web::Bytes::from(
                format!("event: ai_chunk\ndata: {}\n\n", chunk_json)
            ))).await;
            return fallback;
        }

        full_text
    }

    fn local_synthesis(
        &self,
        query: &str,
        results: &[crate::models::SearchResult],
        translated: Option<&TranslatedQuery>,
    ) -> String {
        let lang = translated
            .map(|t| &t.detected_language)
            .unwrap_or(&QueryLang::Indonesian);

        if results.is_empty() {
            return match lang {
                QueryLang::Indonesian => {
                    "❌ **Tidak ditemukan referensi yang cukup.**\n\nSilakan coba dengan kata kunci yang berbeda atau tanyakan kepada ulama setempat.\n\n*Ketiadaan hasil bukan berarti tidak ada hukumnya — mungkin istilah pencarian perlu disesuaikan.*".to_string()
                }
                QueryLang::English => {
                    "❌ **No sufficient references found.**\n\nPlease try different keywords or consult a local scholar.\n\n*Absence of results does not mean there is no ruling — the search terms may need adjustment.*".to_string()
                }
                _ => {
                    "❌ **لم يتم العثور على مراجع كافية.**\n\nيرجى المحاولة بكلمات مفتاحية مختلفة أو مراجعة أهل العلم.\n\n*عدم وجود نتائج لا يعني عدم وجود حكم — قد تحتاج مصطلحات البحث إلى تعديل.*".to_string()
                }
            };
        }

        // ── Deduplicate: group results by book, keep best score per book ──
        let mut seen_books: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();
        let mut unique_results: Vec<&crate::models::SearchResult> = Vec::new();
        for r in results.iter().take(20) {
            let count = seen_books.entry(r.book_id).or_insert(0);
            if *count < 2 { // Allow max 2 entries per book
                unique_results.push(r);
                *count += 1;
            }
        }
        let display_results: Vec<&crate::models::SearchResult> = unique_results.into_iter().take(7).collect();

        // ── Confidence assessment (entity-aware) ──
        let top_score = results.first().map(|r| r.score).unwrap_or(0.0);
        let avg_score: f32 = results.iter().take(5).map(|r| r.score).sum::<f32>() / results.len().min(5) as f32;
        let unique_books: std::collections::HashSet<i64> = results.iter().take(8).map(|r| r.book_id).collect();

        // Entity-aware confidence: verify that key Arabic terms (esp. phrase terms) actually
        // appear inside the retrieved content — a high BM25 score from a generic صلاة book
        // should not yield "Tinggi" when the user asked specifically about الجمعة.
        let entity_match_score: f32 = if let Some(t) = translated {
            if t.arabic_terms.is_empty() {
                1.0 // no terms to check, neutral
            } else {
                // Phrase terms (multi-word like "صلاة الجمعة") are the most specific
                let phrase_terms: Vec<&String> = t.arabic_terms.iter()
                    .filter(|term| term.contains(' '))
                    .collect();
                let single_terms: Vec<&String> = t.arabic_terms.iter()
                    .filter(|term| !term.contains(' '))
                    .collect();

                let phrase_hits = if phrase_terms.is_empty() { 1.0 } else {
                    let hits = results.iter().take(5).filter(|r| {
                        phrase_terms.iter().any(|pt| {
                            r.title.contains(pt.as_str()) ||
                            r.content_snippet.contains(pt.as_str())
                        })
                    }).count();
                    (hits as f32 / 5.0).min(1.0)
                };

                let single_hits = if single_terms.is_empty() { 1.0 } else {
                    let hits = results.iter().take(5).filter(|r| {
                        single_terms.iter().any(|st| {
                            r.title.contains(st.as_str()) ||
                            r.content_snippet.contains(st.as_str())
                        })
                    }).count();
                    (hits as f32 / 5.0).min(1.0)
                };

                // Phrase matches count more
                if !phrase_terms.is_empty() {
                    phrase_hits * 0.7 + single_hits * 0.3
                } else {
                    single_hits
                }
            }
        } else {
            0.8 // no translation data, mild penalty
        };

        // Effective confidence: BM25 score gated by entity presence
        let high_confidence = top_score >= 90.0 && avg_score >= 70.0 && entity_match_score >= 0.4;
        let mid_confidence  = top_score >= 60.0 && entity_match_score >= 0.2;

        // ── Build answer based on language ──
        let mut answer = String::with_capacity(4096);

        // Header with search info
        match lang {
            QueryLang::Indonesian => {
                answer.push_str(&format!("## 📚 Hasil Bahtsul Masail: \"{}\"\n\n", query));
                // Confidence indicator (entity-aware)
                if high_confidence {
                    answer.push_str("✅ **Tingkat keyakinan: Tinggi** — Ditemukan referensi yang sangat relevan\n\n");
                } else if mid_confidence {
                    answer.push_str("⚠️ **Tingkat keyakinan: Sedang** — Referensi ditemukan namun mungkin tidak langsung menjawab\n\n");
                } else {
                    answer.push_str("❓ **Tingkat keyakinan: Rendah** — Referensi yang ditemukan kurang relevan, silakan konsultasi ulama\n\n");
                }
                answer.push_str(&format!("Ditemukan **{} referensi** dari **{} kitab** berbeda.\n\n---\n\n", 
                    results.len(), unique_books.len()));
            }
            QueryLang::English => {
                answer.push_str(&format!("## 📚 Search Results: \"{}\"\n\n", query));
                if high_confidence {
                    answer.push_str("✅ **Confidence: High** — Highly relevant references found\n\n");
                } else if mid_confidence {
                    answer.push_str("⚠️ **Confidence: Medium** — References found but may not directly answer\n\n");
                } else {
                    answer.push_str("❓ **Confidence: Low** — References found may not be sufficiently relevant\n\n");
                }
                answer.push_str(&format!("Found **{} references** from **{} different books**.\n\n---\n\n",
                    results.len(), unique_books.len()));
            }
            _ => {
                answer.push_str(&format!("## 📚 نتائج البحث: \"{}\"\n\n", query));
                if high_confidence {
                    answer.push_str("✅ **مستوى الثقة: عالي** — وجدت مراجع وثيقة الصلة\n\n");
                } else if mid_confidence {
                    answer.push_str("⚠️ **مستوى الثقة: متوسط**\n\n");
                } else {
                    answer.push_str("❓ **مستوى الثقة: منخفض**\n\n");
                }
                answer.push_str(&format!("تم العثور على **{} مرجع** من **{} كتاب** مختلف.\n\n---\n\n",
                    results.len(), unique_books.len()));
            }
        }

        // ── Display concise summary referencing the results below ──
        // NOTE: Full ibaroh content is shown in the Search Results cards below,
        // so the AI answer should be a brief synthesis, not duplicate everything.
        match lang {
            QueryLang::Indonesian => {
                answer.push_str(&format!("Ditemukan pembahasan dalam {} kitab berikut:\n\n", unique_books.len()));
                for (i, r) in display_results.iter().enumerate() {
                    let relevance = if r.score >= 95.0 { "🟢" } else if r.score >= 80.0 { "🟡" } else { "🔵" };
                    let book = if !r.book_name.is_empty() { r.book_name.clone() }
                        else { format!("Kitab {}", r.book_id) };
                    let title_part = if !r.title.is_empty() {
                        format!(" — {}", r.title)
                    } else { String::new() };
                    answer.push_str(&format!("{} **[{}]** {} (Hal. {}){}\n\n",
                        relevance, i + 1, book, r.page, title_part));
                }
                answer.push_str("Klik referensi di bawah untuk melihat teks lengkap dari kitab.\n\n");
            }
            QueryLang::English => {
                answer.push_str(&format!("Discussion found in {} books:\n\n", unique_books.len()));
                for (i, r) in display_results.iter().enumerate() {
                    let relevance = if r.score >= 95.0 { "🟢" } else if r.score >= 80.0 { "🟡" } else { "🔵" };
                    let book = if !r.book_name.is_empty() { r.book_name.clone() }
                        else { format!("Book {}", r.book_id) };
                    let title_part = if !r.title.is_empty() {
                        format!(" — {}", r.title)
                    } else { String::new() };
                    answer.push_str(&format!("{} **[{}]** {} (p. {}){}\n\n",
                        relevance, i + 1, book, r.page, title_part));
                }
                answer.push_str("Click references below to view the full text.\n\n");
            }
            _ => {
                answer.push_str(&format!("وجدت مباحث في {} كتاب:\n\n", unique_books.len()));
                for (i, r) in display_results.iter().enumerate() {
                    let relevance = if r.score >= 95.0 { "🟢" } else if r.score >= 80.0 { "🟡" } else { "🔵" };
                    let book = if !r.book_name.is_empty() { r.book_name.clone() }
                        else { format!("كتاب {}", r.book_id) };
                    let title_part = if !r.title.is_empty() {
                        format!(" — {}", r.title)
                    } else { String::new() };
                    answer.push_str(&format!("{} **[{}]** {} (ص {}){}\n\n",
                        relevance, i + 1, book, r.page, title_part));
                }
                answer.push_str("اضغط على المراجع أدناه لعرض النص الكامل.\n\n");
            }
        }

        // ── Footer section ──
        answer.push_str("---\n\n");
        match lang {
            QueryLang::Indonesian => {
                answer.push_str("⚠️ *Ini bukan fatwa. Untuk kepastian hukum, rujuklah teks asli kitab dan konsultasikan dengan ulama yang kompeten.*");
            }
            QueryLang::English => {
                answer.push_str("⚠️ *This is not a fatwa. For definitive rulings, refer to the original text and consult qualified scholars.*");
            }
            _ => {
                answer.push_str("⚠️ *هذا ليس فتوى. للحكم النهائي يرجى الرجوع إلى النص الأصلي ومراجعة أهل العلم.*");
            }
        }

        answer
    }
}

// ─── System Prompt Builder ───

fn build_system_prompt(lang: &QueryLang) -> String {
    let base = r#"أنت عالم إسلامي متخصص في الفقه الإسلامي والحديث والتفسير والعقيدة. تعمل كمساعد لنظام "بحث المسائل" للبحث في أكثر من 7800 كتاب من أمهات كتب الإسلام الكلاسيكية.

مهمتك الأساسية:
1. فهم الأسئلة الواردة بأي لغة (الإندونيسية، الإنجليزية، العربية، أو خليط منها)
2. استخراج المصطلحات الفقهية الصحيحة من السؤال
3. الإجابة استناداً فقط إلى المقتطفات المقدمة من الكتب
4. ذكر اسم الكتاب واسم المؤلف/العالم في كل استشهاد
5. الإشارة إلى الخلاف بين المذاهب إن وجد
6. ذكر النص العربي الأصلي (العبارة) من المصدر

قواعد مهمة:
- لا تُفتِ بما لم تجده في المراجع المقدمة
- اذكر كل عبارة مع اسم الكتاب والمؤلف ورقم الصفحة
- إذا وجدت خلافاً بين العلماء، اعرض جميع الأقوال
- إذا لم تجد إجابة كافية، قل ذلك بصراحة
- استخدم التنسيق: 📖 [اسم الكتاب] — [المؤلف] (ص X)"#;

    let lang_instruction = match lang {
        QueryLang::Indonesian => {
            "\n\nتعليمات اللغة: أجب باللغة الإندونيسية (Bahasa Indonesia) مع ذكر المصطلحات العربية الأصلية. لكل عبارة، اذكر:\n- النص العربي الأصلي\n- اسم الكتاب\n- اسم المؤلف/العالم\n- رقم الصفحة\n\nContoh format:\n📖 **[Nama Kitab]** — *[Nama Ulama/Pengarang]* (Hal. X)\nIbaroh: \"[teks Arab asli]\"\nTerjemah/penjelasan: [penjelasan dalam bahasa Indonesia]"
        }
        QueryLang::English => {
            "\n\nLanguage instructions: Answer in English, citing Arabic terms from the sources. For each reference, include:\n- Original Arabic text (ibarah)\n- Book name\n- Author/Scholar name\n- Page number\n\nFormat:\n📖 **[Book Name]** — *[Scholar Name]* (p. X)\nIbarah: \"[original Arabic text]\"\nExplanation: [explanation in English]"
        }
        _ => {
            "\n\nتعليمات اللغة: أجب باللغة العربية الفصيحة. لكل مرجع اذكر:\n- النص العربي الأصلي (العبارة)\n- اسم الكتاب\n- اسم المؤلف/العالم\n- رقم الصفحة"
        }
    };

    format!("{}{}", base, lang_instruction)
}

fn build_user_prompt(
    query: &str,
    lang: &QueryLang,
    context: &str,
    translated: Option<&TranslatedQuery>,
) -> String {
    let lang_label = match lang {
        QueryLang::Indonesian => "Bahasa Indonesia",
        QueryLang::English => "English",
        QueryLang::Arabic => "العربية",
        QueryLang::Mixed => "campuran/mixed",
        QueryLang::Unknown => "auto-detect",
    };

    let translation_info = if let Some(t) = translated {
        if !t.arabic_terms.is_empty() {
            format!(
                "\n\nالمصطلحات العربية المستخرجة من السؤال: {}",
                t.arabic_terms.join("، ")
            )
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    format!(
        "السؤال (بلغة: {}): {}{}\n\nالمراجع المتاحة:\n{}\n\nأجب بناءً على هذه المراجع فقط. لكل عبارة اذكر اسم الكتاب واسم المؤلف/العالم ورقم الصفحة.",
        lang_label, query, translation_info, context
    )
}

/// Build rich context string from search results for AI prompts
fn build_rich_context(results: &[crate::models::SearchResult]) -> String {
    results
        .iter()
        .take(10)
        .enumerate()
        .map(|(i, r)| {
            let book_info = if !r.book_name.is_empty() && !r.author_name.is_empty() {
                format!("📖 {} — {} (كتاب رقم {}, ص {})", r.book_name, r.author_name, r.book_id, r.page)
            } else if !r.book_name.is_empty() {
                format!("📖 {} (كتاب رقم {}, ص {})", r.book_name, r.book_id, r.page)
            } else {
                format!("📖 كتاب رقم {}, ص {}", r.book_id, r.page)
            };

            format!(
                "{}. {}\nالعنوان: {}\nالنص: {}\n",
                i + 1,
                book_info,
                r.title,
                if r.content_snippet.is_empty() { &r.title } else { &r.content_snippet }
            )
        })
        .collect()
}

/// Enhanced system prompt for streaming bahtsul masail synthesis
fn build_system_prompt_stream(lang: &QueryLang) -> String {
    let base = r#"أنت عالم إسلامي متخصص في بحث المسائل الفقهية، ومتمرس في استخراج الأحكام من كتب التراث الإسلامي الكلاسيكي. تعمل كمحرك "بحث المسائل" للبحث في أكثر من 7800 كتاب من أمهات كتب الإسلام.

مهمتك الأساسية: التحليل الشامل للعبارات والنصوص المقدمة من الكتب، ثم تقديم إجابة منظمة ومتكاملة بأسلوب بحث المسائل.

الهيكل المطلوب للإجابة:

## ✅ الجواب
[ملخص واضح ومباشر للحكم الشرعي أو الإجابة على السؤال]

## 📖 العبارات والدلائل
[لكل مصدر، اذكر:]
📖 **[اسم الكتاب]** — *[المؤلف/العالم]* (ص X)
> "[النص العربي الأصلي — العبارة]"

[شرح مختصر للعبارة وعلاقتها بالسؤال]

## ⚖️ خلاف العلماء
[إن وجد اختلاف بين المذاهب أو العلماء، اذكر كل قول مع دليله]

## 📝 الخلاصة
[تلخيص نهائي مع الراجح إن أمكن، أو عرض الأقوال بحياد]

---
⚠️ *هذا ليس فتوى رسمية. يرجى الرجوع إلى النص الأصلي ومراجعة أهل العلم المختصين.*

قواعد صارمة:
1. لا تُفتِ بما لم تجده في المراجع المقدمة — الأمانة العلمية أولاً
2. اذكر كل عبارة بالنص العربي الأصلي مع اسم الكتاب والمؤلف والصفحة
3. إذا وجدت خلافاً بين العلماء، اعرض جميع الأقوال بإنصاف
4. إذا لم تجد إجابة كافية في المراجع، قل ذلك بصراحة
5. ابدأ دائماً بالجواب المباشر ثم ادعمه بالعبارات"#;

    let lang_instruction = match lang {
        QueryLang::Indonesian => {
            r#"

تعليمات اللغة: أجب باللغة الإندونيسية (Bahasa Indonesia) مع الحفاظ على العبارات العربية الأصلية.

Format yang harus diikuti:

## ✅ Jawaban
[Ringkasan hukum/jawaban yang jelas dan langsung]

## 📖 Ibaroh & Dalil
[Untuk setiap sumber kitab:]
📖 **[Nama Kitab]** — *[Nama Ulama/Pengarang]* (Hal. X)
> "[Teks Arab asli — ibaroh]"

Penjelasan: [terjemah/penjelasan dalam bahasa Indonesia]

## ⚖️ Perbedaan Pendapat Ulama
[Jika ada khilaf, sebutkan pendapat masing-masing mazhab/ulama]

## 📝 Kesimpulan
[Ringkasan akhir dengan pendapat yang rajih jika memungkinkan]

---
⚠️ *Ini bukan fatwa resmi. Rujuklah teks asli kitab dan konsultasikan dengan ulama yang kompeten.*"#
        }
        QueryLang::English => {
            r#"

Language instructions: Answer in English while preserving original Arabic passages (ibarah).

Required format:

## ✅ Answer
[Clear, direct summary of the ruling/answer]

## 📖 Textual Evidence (Ibarah)
[For each source:]
📖 **[Book Name]** — *[Scholar Name]* (p. X)
> "[Original Arabic text — ibarah]"

Explanation: [English explanation of the passage]

## ⚖️ Scholarly Differences
[If there are different opinions among scholars/madhabs, present each view]

## 📝 Conclusion
[Final summary with the stronger opinion if applicable]

---
⚠️ *This is not an official fatwa. Please refer to original texts and consult qualified scholars.*"#
        }
        _ => {
            r#"

تعليمات اللغة: أجب باللغة العربية الفصيحة. لكل مرجع اذكر:
- النص العربي الأصلي (العبارة) بين علامتي اقتباس
- اسم الكتاب
- اسم المؤلف/العالم
- رقم الصفحة"#
        }
    };

    format!("{}{}", base, lang_instruction)
}
