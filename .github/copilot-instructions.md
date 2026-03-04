# Kizana Search — Copilot Instructions  
## Prinsip Pengembangan: Perspektif Ulama Developer Senior

---

## 1. Identitas & Misi Proyek

**Kizana Search** adalah mesin pencari khazanah turats Islam — 7.872 kitab klasik Arab yang selama ini hanya dapat diakses oleh mereka yang menguasai bahasa Arab. Misi kami: **menurunkan khazanah itu kepada ulama, santri, dan masyarakat awam yang bertanya dalam bahasa mereka sendiri**.'[]

Pengguna kita adalah komunitas pesantren Indonesia, akademisi Islam, dan masyarakat umum yang:
- Bertanya dalam **Bahasa Indonesia**, **Bahasa Inggris**, atau **campuran keduanya**
- Menggunakan **terminologi Arab yang sudah terserap** ke Bahasa Indonesia (shalat, wudhu, ta'wil, mafsadat, dll.)
- Tidak selalu tahu nama kitab, pengarang, mazhab, atau halaman yang relevan
- Membutuhkan jawaban yang dapat dipertanggungjawabkan secara keilmuan (*musnad*, bisa ditelusuri ke sumbernya)

---

## 2. Prinsip Utama: Jembatan Bahasa Menuju Turats

### 2.1 Masalah Inti

Queries datang dalam bahasa manusia biasa; database berisi teks Arab klasik. Gap ini bukan sekadar masalah *machine translation* — ini masalah **lintas dunia epistemik**:

| Pengguna berkata | Yang dicari di kitab |
|---|---|
| "hukum vaksin" | التطعيم، اللقاح، إدخال نجاسة |
| "boleh gak foto makhluk hidup" | تصوير ذوات الأرواح، المصور |
| "shalat sambil pegang bayi mau ngompol" | حمل الطفل في الصلاة، حمل النجاسة |
| "nikah siri konsekuensinya apa" | النكاح السري، نكاح بغير ولي، نسب الولد |
| "istri minta cerai terus suami gak mau" | الخلع، شقاق، حكم القاضي بالطلاق |

**Prinsip**: Jangan pernah treat query sebagai literal string match ke teks Arab. Setiap query harus melalui proses **ta'bir** — mentransformasi ungkapan awam menjadi konsep fikih/usul yang tepat.

### 2.2 Lapisan Query Understanding

Setiap query harus diproses melalui empat lapisan:

```
[Query Pengguna]
       ↓
1. DETEKSI BAHASA & REGISTER
   - Bahasa: id / en / mixed / ar-transliterated
   - Register: awam / santri / akademik
   - Domain: fiqh / aqidah / tasawuf / tafsir / hadits / akhlak / muamalat
       ↓
2. NORMALISASI TERMINOLOGI
   - Transliterasi → Arab: shalat→صلاة, wudhu→وضوء, ta'wil→تأويل
   - Padanan konsep: "nikah siri"→نكاح بغير شهود/ولي, "riba"→ربا/فائدة
   - Sinonim lintas mazhab: berbeda istilah tapi satu konsep
       ↓
3. EKSPANSI SEMANTIK
   - Tambahkan konsep induk (genus): hukum foto → taswir → ذوات الأرواح
   - Tambahkan dalil terkait: kalau soal shalat → rukun, syarat, wajibat
   - Tambahkan kontra-konsep: kalau soal boleh/haram → khilaf ulama
       ↓
4. KONSTRUKSI QUERY ARAB
   - Multi-term query dengan bobot BM25
   - Kata kunci utama + konteks pendukung
```

---

## 3. Kamus Transliterasi & Pemetaan Konsep Wajib

### 3.1 Ibadah Mahdhah

| Indonesia/Inggris | Arabnya |
|---|---|
| shalat, solat, prayer | صلاة، الصلاة |
| wudhu, wudu, ablution | وضوء، الطهارة |
| tayammum | تيمم |
| puasa, shaum, fasting | صوم، صيام |
| zakat, zakah | زكاة |
| haji, hajj | حج |
| umroh, umrah | عمرة |
| sujud, prostration | سجود |
| ruku | ركوع |
| qunut | قنوت |
| shalat jamak, jama', combined prayer | الجمع بين الصلاتين |
| shalat qashar, qasr | قصر الصلاة |
| shalat jumat, friday prayer | صلاة الجمعة |
| i'tikaf | اعتكاف |

### 3.2 Thaharah (Bersuci)

| Indonesia/Inggris | Arabnya |
|---|---|
| najis, impure | نجاسة، النجس |
| hadats besar, junub | جنابة، الحدث الأكبر |
| hadats kecil | الحدث الأصغر |
| suci, tahir, pure | طهارة، طاهر |
| mani, sperma | مني |
| haid, menstruasi, menstruation | حيض، المحيض |
| nifas, postpartum | نفاس |
| istihadzah | استحاضة |
| air kencing, urine | بول |
| kotoran, tinja, feces | غائط |
| darah, blood | دم |
| muttanajjis | متنجس |

### 3.3 Muamalat & Ekonomi Islam

| Indonesia/Inggris | Arabnya |
|---|---|
| riba, bunga bank, interest | ربا، الفائدة |
| jual beli, transaksi, trade | بيع، المعاملات |
| utang piutang, debt | دين، قرض |
| gadai, collateral | رهن |
| sewa, ijarah, rent | إجارة |
| asuransi syariah | التأمين، تكافل |
| mudharabah, profit sharing | مضاربة |
| musyarakah, partnership | مشاركة |
| wakaf, endowment | وقف |
| hibah, gift | هبة |
| wasiat, will | وصية |
| waris, inheritance | الميراث، التركة |

### 3.4 Munakahat (Hukum Keluarga)

| Indonesia/Inggris | Arabnya |
|---|---|
| nikah, kawin, marriage | نكاح، زواج |
| nikah siri | نكاح بغير ولي، نكاح سري |
| cerai, talak, divorce | طلاق |
| khuluk, khulu', cerai gugat | خلع |
| iddah | عدة |
| nafkah, nafaqah | نفقة |
| mahar, mas kawin, dowry | مهر، صداق |
| poligami, polygamy | تعدد الزوجات |
| walimah | وليمة |
| wali nikah, marriage guardian | ولي |
| saksi nikah | الشهود |
| mut'ah | متعة |

### 3.5 Aqidah & Kalam

| Indonesia/Inggris | Arabnya |
|---|---|
| tauhid, monotheism | توحيد |
| syirik, shirk, polytheism | شرك |
| murtad, apostasy | ردة |
| bid'ah, innovation | بدعة |
| kafir | كفر |
| iman, faith | إيمان |
| tawakkal | توكل |
| tawarruk, tawasul | توسل |
| qadha qadar, fate | قضاء وقدر |

### 3.6 Terminologi Fiqh Lintas Mazhab

| Konsep | Istilah mazhab |
|---|---|
| Wajib/fardhu | فرض عند الحنفية / واجب عند الجمهور |
| Sunnah | مستحب / مندوب / نفل |
| Makruh | مكروه تحريمي / مكروه تنزيهي |
| Syarat | شرط |
| Rukun | ركن / فرض |
| Fasid vs. batal | فاسد (Hanafi) / باطل (jumhur) |

---

## 4. Prinsip Sintesis Jawaban (Metodologi Ulama)

### 4.1 Isnad Mind: Setiap Klaim Harus Bersumber

Seperti ulama hadits yang tidak menerima riwayat tanpa sanad, sistem kita **tidak boleh memberi jawaban tanpa menunjuk ke kitab dan halaman**. Format wajib respons:

```
Jawaban: [pernyataan hukum/penjelasan]
Sumber: [nama kitab atau book_id], hal. [page]
Konteks: [kutipan relevan dari kitab]
```

Jangan pernah generate jawaban fiqh tanpa `search_results` yang mendukungnya. Ini equivalent dengan fatwa tanpa dalil — haram secara epistemik.

### 4.2 Ikhtisar Khilaf: Tampilkan Perbedaan Pendapat

Ketika hasil pencarian menunjukkan pendapat berbeda, sintesis harus:

1. **Sebutkan posisi masing-masing** (Imam Syafi'i berkata X, Imam Malik berkata Y)
2. **Jelaskan dalil masing-masing** kalau ada di hasil
3. **Tunjukkan mana yang rajih** (lebih kuat) **atau biarkan pengguna memilih** — jangan memutus khilaf mu'tabar dengan sepihak
4. **Tandai bila ada ijma'** — kalau semua kitab sepakat, sebutkan

### 4.3 Takrij Query: Deteksi Konteks dari Bahasa Campuran

Ketika query campuran (contoh: *"boleh gak makan gelatin dari babi buat obat darurat"*):

- **"boleh gak"** → register awam, Bahasa Indonesia — pertanyaan hukum taklifi
- **"gelatin dari babi"** → fiqh makanan → حكم أكل لحم الخنزير، المحرمات
- **"obat darurat"** → ada klausul **العذر والضرورة** → الضرورة تبيح المحظورات
- Sistem harus mengekspansi ke: الضرورة، الاضطرار، حكم التداوي بالمحرم

Kode yang menangani ini harus mengimplementasi **tabel ekspansi kontekstual**, bukan hanya keyword matching.

### 4.4 Tingkat Kepercayaan Jawaban

Tandai jawaban dengan confidence tier:

| Tier | Kondisi | Label |
|---|---|---|
| **Qath'i** | Sumber sangat relevan, skor tinggi, konten jelas | ✅ Ditemukan langsung di kitab |
| **Zhanni** | Sumber relevan tapi tidak langsung, harus inferensi | ⚠️ Berdasarkan prinsip umum |
| **Ghaib** | Tidak ada sumber yang cukup relevan | ❌ Tidak ditemukan, butuh referensi lanjut |

---

## 5. Prinsip Kode: Rust Backend

### 5.1 Search Engine (`search.rs`)

**Wajib dikembangkan**: Query translation layer sebelum Tantivy QueryParser:

```rust
// TODO: Implementasi ini adalah prioritas utama
pub fn translate_query(raw_query: &str) -> TranslatedQuery {
    // 1. Detect language (id/en/ar/mixed)
    // 2. Normalize transliterasi
    // 3. Expand ke sinonim Arab
    // 4. Return multi-term Arabic query + original terms
}

pub struct TranslatedQuery {
    pub original: String,
    pub arabic_terms: Vec<String>,      // Ekspansi ke Arab
    pub latin_terms: Vec<String>,        // Terms dari query asli
    pub detected_domain: FiqhDomain,    // fiqh/aqidah/tafsir/etc
    pub detected_language: QueryLang,   // id/en/ar/mixed
}

pub enum FiqhDomain {
    Ibadah, Muamalat, Munakahat, Jinayat,
    Aqidah, Tasawuf, Tafsir, Hadits, Akhlak,
    Unknown,
}
```

**Query expansion rules** — harus di-hardcode dulu, AI-augmented kemudian:

```rust
// Dalam search.rs, sebelum query dikirim ke Tantivy:
fn expand_to_arabic(term: &str) -> Vec<&'static str> {
    match term.to_lowercase().as_str() {
        "shalat" | "solat" | "salat" | "prayer" => {
            vec!["صلاة", "الصلاة", "الصلوات"]
        }
        "wudhu" | "wudu" | "ablution" => {
            vec!["وضوء", "الوضوء", "الطهارة"]
        }
        "puasa" | "fasting" | "shaum" => {
            vec!["صوم", "صيام", "الصيام"]
        }
        "riba" | "bunga" | "interest" | "usury" => {
            vec!["ربا", "الربا", "فائدة", "ربا الفضل", "ربا النسيئة"]
        }
        "nikah" | "kawin" | "marriage" | "pernikahan" => {
            vec!["نكاح", "زواج", "التزويج", "عقد النكاح"]
        }
        "cerai" | "talak" | "divorce" => {
            vec!["طلاق", "الطلاق", "فراق"]
        }
        "waris" | "warisan" | "inheritance" => {
            vec!["ميراث", "الميراث", "التركة", "الإرث", "الفرائض"]
        }
        "zakat" | "zakah" | "zakāt" => {
            vec!["زكاة", "الزكاة", "زكاة المال"]
        }
        "haji" | "hajj" | "pilgrimage" => {
            vec!["حج", "الحج", "المناسك"]
        }
        // ... lanjutkan dengan seluruh kamus di atas
        _ => vec![],
    }
}
```

### 5.2 AI Synthesizer (`ai.rs`)

**System prompt wajib mencakup** instruksi multibahasa dan metodologi ulama:

```rust
const SYSTEM_PROMPT: &str = r#"
أنت عالم إسلامي ومطور برمجي متخصص في الفقه الإسلامي والتراث الإسلامي الكلاسيكي.
تعمل كمساعد لنظام "كيزانة" للبحث في 7872 كتابًا من أمهات كتب الإسلام.

مهمتك:
1. فهم الأسئلة الواردة بالإندونيسية أو الإنجليزية أو العربية أو خليط منها
2. استخراج المصطلحات الفقهية الصحيحة من السؤال
3. الإجابة استناداً فقط إلى مقتطفات الكتب المقدمة إليك
4. ذكر مصادرك بشكل صريح (اسم الكتاب ورقم الصفحة)
5. الإشارة إلى الخلاف بين المذاهب إن وجد

قواعد اللغة:
- إذا جاء السؤال بالإندونيسية، أجب بالإندونيسية مع المصطلحات العربية
- إذا جاء بالإنجليزية، أجب بالإنجليزية مع المصطلحات العربية
- إذا جاء بالعربية، أجب بالعربية
- في جميع الحالات، اذكر النص العربي الأصلي من المصدر

تحذير: لا تُفتِ بدون مصدر. لا تُجب بما لم تجده في المراجع المقدمة.
"#;
```

**User prompt harus menyertakan bahasa yang terdeteksi**:

```rust
fn build_user_prompt(query: &str, detected_lang: &str, results: &[SearchResult]) -> String {
    let lang_instruction = match detected_lang {
        "id" => "أجب باللغة الإندونيسية مع ذكر المصطلحات العربية الأصلية",
        "en" => "Answer in English, citing Arabic terms from the sources",
        "ar" => "أجب باللغة العربية الفصيحة",
        _ => "أجب بنفس لغة السؤال",
    };

    format!(
        "السؤال: {}\n\n{}\n\nالمراجع المتاحة:\n{}\n\nأجب بناءً على هذه المراجع فحسب.",
        query,
        lang_instruction,
        format_results_as_context(results)
    )
}
```

### 5.3 Result Ranking Enhancement

Tambahkan **domain relevance boost** ke scoring BM25 yang ada:

```rust
// Dalam search.rs setelah scoring BM25
fn apply_domain_boost(score: f32, result: &RawResult, domain: &FiqhDomain) -> f32 {
    // Kitab fiqh mendapat boost untuk query fiqh
    // Kitab tafsir mendapat boost untuk query tentang makna ayat
    // Kitab hadits mendapat boost untuk query "hadits tentang X"
    let domain_boost = match (domain, &result.book_category) {
        (FiqhDomain::Ibadah, "فقه") => 1.3,
        (FiqhDomain::Muamalat, "فقه") => 1.3,
        (FiqhDomain::Tafsir, "تفسير") => 1.4,
        (FiqhDomain::Hadits, "حديث") => 1.4,
        _ => 1.0,
    };
    score * domain_boost
}
```

---

## 6. Prinsip Kode: SvelteKit Frontend

### 6.1 Query Input UX

- **Placeholder harus multilingual**: *"Tanyakan masalah fikih... (B. Indonesia / English / عربي)"*
- **Deteksi bahasa real-time**: tunjukkan badge bahasa saat user mengetik
- Jangan koreksi spelling otomatis untuk terminologi Arab transliterasi — *"shalat"* bukan salah eja

### 6.2 Tampilan Hasil

Setiap result card harus menampilkan:
```
[Skor relevansi] [Nama bab/pasal dari TOC] 
Nama Kitab • Halaman N
Kutipan teks Arab (teks asli Arabic font)
Nama pengarang + mazhab jika tersedia
```

### 6.3 AI Answer Display

Jawaban AI harus dibedakan secara visual dari hasil pencarian mentah:
- Label jelas: *"Sintesis dari [N] referensi"* atau *"Berdasarkan kitab [X]"*
- Footnote citations yang bisa diklik untuk lompat ke kitab
- Confidence tier badge (lihat §4.4)

---

## 7. Prinsip Non-Teknis: Adab Keilmuan

### 7.1 Amanah dalam Mengutip

Sistem ini memegang amanah keilmuan yang besar. Satu kesalahan atribusi bisa:
- Menisbatkan pendapat kepada ulama yang tidak berpendapat demikian
- Mengacaukan khilaf mu'tabar dengan khilaf syaz
- Menipu pengguna yang menjadikan ini dasar fatwa atau keputusan hidup

**Kode wajib**: Setiap `SearchResult` yang ditampilkan ke pengguna harus menyertakan `book_id` dan `page` yang dapat diverifikasi manual.

### 7.2 Tawadhu' Epistemik

Kalau sistem tidak cukup yakin:
- Jangan generate jawaban dengan confidence palsu
- Tampilkan: *"Tidak ditemukan referensi yang cukup. Silakan tanyakan kepada ulama setempat."*
- Ini bukan kelemahan — ini adalah **adab thalabul ilmi**

### 7.3 Khilaf adalah Rahmat

Ketika dua kitab menunjukkan pendapat berbeda, jangan collapse menjadi satu jawaban. Khilaf ulama adalah **kekayaan intelektual**, bukan bug yang harus di-fix. Tampilkan keduanya.

### 7.4 Bahasa Awam ≠ Pertanyaan Lemah

Query *"boleh gak makan nasi goreng yang dimasak sama alkohol dikit"* adalah pertanyaan hukum yang serius, sama separti *"ما حكم الطبخ بالكحول"*. Jangan treat pertanyaan tidak formal sebagai noise — justru ini yang paling banyak datang dari pengguna riil.

---

## 8. Stack & Arsitektur

```
SvelteKit (port 3000)          → Frontend RTL Arabic, bilingual UI
  ↕ /api/* proxy
Actix-Web / Rust (port 8080)   → Query handler, auth, cache
  ├─ Tantivy BM25               → Lexical search over 3.4M+ TOC entries  
  ├─ SQLite (20GB)              → 7,872 books full text + TOC hierarchy
  ├─ Redis                      → Search result cache (1h TTL)
  └─ AI Synthesizer             → Grok/Claude → multibahasa answer synthesis
Nginx                          → Reverse proxy + SSL termination
```

**Database schema** yang harus selalu diingat:
```sql
-- Per book N (1..7872):
b{N} (id, content, part, page, number, services, is_deleted)  -- full text halaman
t{N} (id, content, page, parent, is_deleted)                   -- TOC / daftar isi
-- parent = 0 means root chapter; content has <span data-type="title"> markers
```

---

## 9. Roadmap Prioritas Fitur

Urutan pengembangan berdasarkan dampak keilmuan:

1. **P0** — Query translation layer (§5.1) — ini yang paling kritis sekarang
2. **P0** — Multilingual AI prompt (§5.2) — jawaban harus dalam bahasa pengguna
3. **P1** — Mazhab filter (Syafi'i / Hanafi / Maliki / Hanbali)
4. **P1** — Kitab metadata enrichment (pengarang, tahun, mazhab, bidang)
5. **P2** — Cross-reference: "kitab lain yang membahas topik sama"
6. **P2** — Fuzzy Arab search (tanpa harakat, varian imla')
7. **P3** — Bahtsul masail format output (masalah → jawaban → ta'bir → maraji')
8. **P3** — Personalized history: santri bisa simpan hasil riset per topic

---

## 10. Contoh Alur Lengkap Query

**Input**: `"cerai atas permintaan istri karena suami impoten, apa hukumnya?"`

**Step 1 — Deteksi**:
- Bahasa: Indonesia
- Domain: Munakahat
- Konsep kunci: "cerai atas permintaan istri" → خلع / فسخ النكاح
- Konsep pendukung: "impoten" → عنين، العنة، الجب
- Konteks: suami sebagai pihak bermasalah → يوجب للزوجة الفسخ

**Step 2 — Query Arab**:
```
خلع العنين | فسخ النكاح بالعيوب | العنة زوجة | خيار العيب في النكاح
```

**Step 3 — Cari di Tantivy** dengan terms di atas

**Step 4 — Sintesis AI** dengan instruction bahasa Indonesia:
```
"Berdasarkan [Kitab X hal. Y], istri berhak meminta fasakh nikah karena aib 
'unnah (impotensi suami) dengan syarat... Imam Syafi'i berpendapat [Z] 
sedangkan Imam Maliki berpendapat [W]..."
```

**Step 5 — Tampilkan** dengan link ke source, confidence tier, dan opsi buka kitab lengkap.

---

*Ditulis dengan semangat: "العلم نور"*  
*Kode yang baik adalah kode yang dapat dipertanggungjawabkan, seperti ilmu yang dapat dipertanggungjawabkan.*
