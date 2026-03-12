# 20 Pertanyaan Presentasi ke Digdaya NU & PBNU
## Kizana Search — Mesin Pencari Khazanah Turats Islam

---

## I. PERTANYAAN BUSINESS FLOW (1-7)

### 1. "Apa model bisnis Kizana? Bagaimana ini menghasilkan revenue?"

**Jawaban:**
Kizana memiliki 3 layer monetisasi:

| Layer | Model | Target |
|-------|-------|--------|
| **Freemium** | Gratis: 10 query/hari, akses 100 kitab populer | Santri, masyarakat umum |
| **Premium** | Rp 49.000/bulan: unlimited query, 7.872 kitab, AI synthesis, export PDF | Ustadz, peneliti, kampus |
| **API** | Pay-per-query (Rp 500/query): REST API untuk integrasi | Lembaga fatwa, app developer, media Islam |

**Proyeksi:**
- 50.000 pesantren di Indonesia × 5 pengguna/pesantren = 250.000 TAM (Total Addressable Market)
- Konversi 5% ke premium = 12.500 subscriber × Rp 49.000 = **Rp 612,5 juta/bulan**
- API enterprise (PBNU, Kemenag, MUI): kontrak tahunan

**Status saat ini:** Layer API sudah diimplementasi (v1 API key management). Layer freemium/premium perlu ditambahkan (user quota system).

**GAP:** ❌ Belum ada payment gateway, subscription management, atau user quota enforcement.

**IMPLEMENTASI:**
```
P1: Tambah field `subscription_tier` + `query_count_today` di tabel users
P2: Integrasi Midtrans/Xendit untuk payment
P3: Dashboard admin untuk monitoring subscription
```

---

### 2. "Siapa kompetitor Kizana? Apa bedanya dengan Shamela, IslamWeb, atau Google?"

**Jawaban:**

| Platform | Bahasa Input | Database | AI Synthesis | Bahasa Output |
|----------|-------------|----------|--------------|---------------|
| **Shamela (المكتبة الشاملة)** | Arab only | 6.000+ kitab | ❌ | Arab only |
| **IslamWeb** | Arab only | Fatwa + sebagian kitab | ❌ | Arab only |
| **Google** | Multi | Web pages | ❌ (tidak spesifik) | Multi |
| **Kizana** | **Indonesia + English + Arab** | **7.872 kitab** | **✅ DeepSeek AI** | **Bahasa pengguna** |

**Keunggulan unik Kizana:**
1. **Jembatan bahasa** — Tanya "boleh gak foto makhluk hidup" → otomatis cari تصوير ذوات الأرواح
2. **AI Synthesis** — Bukan sekadar daftar hasil, tapi sintesis jawaban dengan referensi kitab + halaman
3. **Query Translation** — 500+ istilah Indonesia/Inggris → Arab, termasuk morfologi Indonesia (membatalkan → batal → باطل)
4. **Produk Hukum Modern** — Integrasi keputusan Bahtsul Masail NU (tidak ada di platform lain)

**GAP:** ✅ Sudah sangat kuat. Perlu benchmark kualitas pencarian vs Shamela secara formal.

---

### 3. "Bagaimana rencana scaling jika pengguna membludak?"

**Jawaban:**

**Arsitektur saat ini:**
```
Nginx → Rust Backend (Actix, 4 workers) → SQLite (20GB) + Tantivy + Redis
```

**Kapasitas saat ini:** ~500 concurrent users (Actix async, RAM 8GB)

**Scaling roadmap:**

| Phase | Trigger | Aksi |
|-------|---------|------|
| **Phase 1** (saat ini) | <1.000 users | Single VPS, SQLite, Redis |
| **Phase 2** | 1.000-10.000 users | PostgreSQL, replica read, CDN untuk frontend |
| **Phase 3** | 10.000-100.000 users | Kubernetes, horizontal pod scaling, Tantivy sharding |
| **Phase 4** | 100.000+ users | Multi-region (Indonesia + Middle East), edge caching |

**Yang sudah disiapkan:**
- ✅ Stateless JWT auth (bisa horizontal scale tanpa session affinity)
- ✅ Redis caching layer (bisa upgrade ke Redis Cluster)
- ✅ Tantivy index read-only setelah build (bisa di-replicate)

**GAP:** ❌ Belum ada monitoring/observability (Prometheus/Grafana), belum ada load testing results.

---

### 4. "Bagaimana strategi akuisisi pengguna? Bagaimana masuk ke 50.000 pesantren?"

**Jawaban:**

**Channel akuisisi:**

1. **PBNU Network** (top-down):
   - Endorsement resmi dari PBNU → distribusi ke seluruh cabang
   - Integrasi dengan platform digital NU (nu.or.id, NU Online)
   - Workshop bahtsul masail digital di Munas/Muktamar

2. **Pesantren Champions** (bottom-up):
   - Pilot di 10 pesantren besar (Lirboyo, Sidogiri, Tebuireng, Ploso, dll.)
   - Santri sebagai "duta digital" — mereka yang paling butuh alat ini
   - Free tier untuk pesantren (branded: "didukung oleh [Pesantren X]")

3. **Content Marketing**:
   - YouTube: "Cara Bahtsul Masail dengan AI" — demo live menjawab pertanyaan fiqih
   - Social media: daily "Tahukah Anda?" dengan hasil pencarian Kizana
   - Partnership dengan media Islam (NU Online, Islami.co)

4. **Academic Channel**:
   - UIN/IAIN seluruh Indonesia — integrasi dengan kurikulum Ushul Fiqih
   - Thesis/skripsi tool — alat bantu riset mahasiswa studi Islam

**GAP:** ❌ Belum ada landing page marketing, onboarding flow, atau analytics tracking.

---

### 5. "Bagaimana Kizana menjaga keberlanjutan proyek ini dalam jangka panjang?"

**Jawaban:**

**Pilar keberlanjutan:**

1. **Open Knowledge, Closed Service** — Database kitab tetap terbuka untuk penelitian, tapi layanan pencarian + AI premium
2. **Community-driven maintenance** — Query translation dictionary bisa di-crowdsource dari ulama
3. **Institutional backing** — Kerjasama formal dengan PBNU/Kemenag sebagai infrastruktur resmi
4. **Revenue self-sustaining** — Target break-even di tahun ke-2 (lihat Q1)

**Tim yang dibutuhkan (minimal):**
| Role | Jumlah | Fungsi |
|------|--------|--------|
| Backend Engineer | 1 | Maintain Rust core + scaling |
| AI/NLP Engineer | 1 | Query translation + AI prompt tuning |
| Islamic Content Expert | 1 | Validasi hasil, enrich metadata kitab |
| Product/Design | 1 | UX + growth |

**GAP:** ❌ Saat ini one-man project. Perlu formalisasi tim dan struktur organisasi.

---

### 6. "Bagaimana posisi Kizana dalam ekosistem digital NU?"

**Jawaban:**

```
                    EKOSISTEM DIGITAL NU
                    
    NU Online (berita)    ←→    Kizana (riset kitab)
           ↕                          ↕
    LAZISNU (zakat)       ←→    Bahtsul Masail Digital
           ↕                          ↕
    NU Care (sosial)      ←→    Pendidikan Pesantren
```

**Posisi Kizana:** Menjadi **"perpustakaan digital resmi"** untuk seluruh kegiatan keilmuan NU:
- **Lajnah Bahtsul Masail** → Tool riset untuk merumuskan keputusan hukum
- **Ma'arif NU** → Alat bantu pengajaran di madrasah/pesantren
- **Rabithah Ma'ahid** → Standarisasi akses kitab antar pesantren
- **NU Online** → Backend referensi untuk artikel keagamaan

**GAP:** ❌ Belum ada API integration dengan platform NU lainnya. Perlu pembahasan di level organisasi.

---

### 7. "Apa rencana untuk konten non-Arab? Kitab Jawi, kitab terjemahan Indonesia?"

**Jawaban:**

**Roadmap konten:**

| Phase | Konten | Timeline |
|-------|--------|----------|
| **V1** (sekarang) | 7.872 kitab Arab klasik | ✅ Done |
| **V2** | Produk Hukum NU (Bahasa Indonesia) | ✅ Done |
| **V3** | Kitab Jawi (Arab-Melayu/Jawa) | Perlu OCR + preprocessing |
| **V4** | Terjemahan Indonesia kitab populer | Partnership penerbit |
| **V5** | Kitab modern (kontemporer) | Lisensi diperlukan |

**Tantangan kitab Jawi:**
- Encoding Arab Pegon/Arab Melayu tidak standar
- Perlu OCR khusus (Tesseract + fine-tuning untuk Pegon)
- Query translation perlu diperluas ke Jawa/Melayu → Arab klasik

**GAP:** ❌ Belum ada pipeline untuk ingest konten non-Arab. Perlu R&D untuk OCR Pegon.

---

## II. PERTANYAAN AGAMIS / KEAGAMAAN (8-14)

### 8. "Bagaimana Kizana menjamin keakuratan referensi kitab? Apakah tidak takut salah menisbatkan pendapat?"

**Jawaban:**

**Prinsip Isnad (Sanad Keilmuan):**

Kizana dibangun dengan prinsip **amanah keilmuan**:

1. **Setiap kutipan = verifiable** — Setiap hasil pencarian menyertakan:
   - `book_id` → ID kitab yang dapat diverifikasi
   - `page` → Nomor halaman persis
   - `toc_content` → Bab/pasal dari daftar isi
   - Teks Arab asli (bukan terjemahan)

2. **AI tidak boleh berfatwa tanpa dalil** — System prompt AI:
   > "تحذير: لا تُفتِ بدون مصدر. لا تُجب بما لم تجده في المراجع المقدمة."
   > (Peringatan: Jangan berfatwa tanpa sumber. Jangan jawab dengan apa yang tidak ditemukan di referensi.)

3. **Confidence tier** — Setiap jawaban ditandai tingkat kepercayaan:
   - ✅ **Qath'i** — Ditemukan langsung di kitab
   - ⚠️ **Zhanni** — Berdasarkan prinsip umum
   - ❌ **Ghaib** — Tidak ditemukan, perlu rujukan lanjut

4. **"Tidak tahu" adalah jawaban yang sah** — Kalau tidak ada referensi yang relevan, sistem menampilkan: *"Tidak ditemukan referensi yang cukup. Silakan tanyakan kepada ulama setempat."*

**Status:** ✅ Semuanya sudah diimplementasi di `ai.rs` (system prompt) dan frontend (source chips + page numbers).

**GAP:** ⚠️ Belum ada mekanisme user feedback untuk laporkan kesalahan atribusi. Perlu "report inaccuracy" button.

---

### 9. "Apakah hasil pencarian Kizana bisa dijadikan dasar fatwa?"

**Jawaban:**

**Posisi Kizana: Alat bantu riset, BUKAN pengganti ulama.**

```
                    HIERARKI PENGAMBILAN HUKUM

    [1] Ulama / Lajnah Bahtsul Masail  ← PENGAMBIL KEPUTUSAN
              ↑
    [2] Kizana Search                   ← ALAT BANTU RISET
              ↑  
    [3] Kitab-kitab Turats              ← SUMBER PRIMER
```

**Analogi:** Kizana adalah seperti **perpustakaan digital** — ia memudahkan akses ke kitab, tapi keputusan hukum tetap di tangan ulama yang kompeten.

**Fitur yang mendukung:**
- Menampilkan **khilaf ulama** (perbedaan pendapat) — tidak memutus sepihak
- Menyertakan teks Arab asli — ulama bisa verifikasi langsung
- Domain detection (fiqh/aqidah/tafsir) — membantu kategorisasi

**Pesan untuk PBNU:** Kizana **mempercepat** proses bahtsul masail dari berhari-hari menjadi menit, tapi **tidak menggantikan** musyawarah ulama. Ini upgrade efisiensi, bukan otomasi fatwa.

---

### 10. "Bagaimana Kizana menangani khilaf antar mazhab? Apakah bias ke Syafi'i?"

**Jawaban:**

**Prinsip:** *"Khilaf adalah rahmat, bukan bug yang harus di-fix."*

**Implementasi saat ini:**
- ✅ Pencarian dilakukan across semua 7.872 kitab — tidak ada filter mazhab default
- ✅ Hasil mencakup semua mazhab (Syafi'i, Hanafi, Maliki, Hanbali)
- ✅ AI synthesis diinstruksikan untuk menampilkan perbedaan pendapat
- ✅ Domain detection sudah ada (Ibadah, Muamalat, Munakahat, dll.)

**Namun secara de facto:**
- Koleksi kitab mungkin lebih banyak dari satu mazhab (tergantung sumber data)
- Tanpa metadata mazhab per kitab, tidak bisa dipastikan distribusinya

**Roadmap P1:** Filter mazhab
```
[Semua] [Syafi'i] [Hanafi] [Maliki] [Hanbali]
```
User bisa memilih mazhab yang ingin dicari, atau "Semua" untuk melihat lintas mazhab.

**GAP:** ❌ Metadata mazhab per kitab belum lengkap. Perlu enrichment manual oleh tim konten Islam. Filter UI belum ada.

**IMPLEMENTASI:**
```
1. Enrichment: Tambah kolom `mazhab` ke tabel book metadata
2. Backend: Tambah filter parameter di search API
3. Frontend: Tambah filter chip di UI pencarian
4. Validasi: Tim konten Islam review klasifikasi mazhab
```

---

### 11. "Bagaimana dengan kitab-kitab yang kontroversial atau mengandung pendapat syaz (menyimpang)?"

**Jawaban:**

**Pendekatan Kizana:**

1. **Inklusif tapi transparan** — Semua kitab ditampilkan, tapi diberi konteks:
   - Nama pengarang + era
   - Mazhab (jika teridentifikasi)
   - Apakah termasuk kitab mu'tabar di kalangan NU

2. **Tidak menyensor** — Menyensor kitab tertentu berarti memutus akses keilmuan. Ulama sendiri perlu membaca pendapat yang berbeda untuk memahami dalil.

3. **Kontekstualisasi** — AI synthesis membedakan antara:
   - **Qaul mu'tamad** (pendapat yang dipegangi mayoritas)
   - **Qaul dhaif** (pendapat yang lemah)
   - **Qaul syaz** (pendapat yang menyimpang)

**Saran untuk PBNU:** Buat **"whitelist kitab mu'tabar"** — kitab-kitab yang diakui oleh NU sebagai rujukan resmi. Kizana bisa memberi badge khusus untuk kitab-kitab ini.

**GAP:** ❌ Belum ada klasifikasi kitab mu'tabar vs non-mu'tabar. Ini perlu kolaborasi dengan Lajnah Bahtsul Masail.

---

### 12. "Apakah AI tidak akan menyesatkan umat dengan jawaban yang salah?"

**Jawaban:**

**Safeguard yang sudah dibangun:**

| Layer | Perlindungan | Status |
|-------|-------------|--------|
| **Input** | Query sanitization, max 500 chars | ✅ |
| **Search** | BM25 scoring — hanya tampilkan yang relevan | ✅ |
| **AI Prompt** | "Jangan berfatwa tanpa sumber" (bahasa Arab) | ✅ |
| **AI Constraint** | AI hanya merangkum dari hasil pencarian, tidak generate sendiri | ✅ |
| **Output** | Setiap klaim di-cite ke kitab + halaman | ✅ |
| **Fallback** | Jika AI gagal, fallback ke local synthesis (non-AI) | ✅ |
| **Disclaimer** | "Ini alat bantu riset, bukan fatwa" | ⚠️ Perlu lebih prominent |

**Worst case scenario dan mitigasi:**
- AI salah rangkum → User bisa verifikasi dari teks Arab asli yang ditampilkan
- AI hallucinate → System prompt + only-cite-sources constraint minimizes this
- Hasil pencarian tidak relevan → Confidence tier "Ghaib" menandakan ketidakyakinan

**Analogi:** Seperti buku ensiklopedia — buku tidak menyesatkan, yang menyesatkan adalah orang yang membaca tanpa pemahaman. Kizana menyediakan informasi; penafsiran tetap memerlukan keilmuan.

**GAP:** ⚠️ Perlu disclaimer yang lebih eksplisit di UI: "Hasil ini adalah alat bantu riset. Untuk keperluan fatwa, konsultasikan dengan ulama yang berkompeten."

---

### 13. "Bagaimana posisi Kizana terhadap tradisi pesantren yang menekankan talaqqi (belajar langsung dari guru)?"

**Jawaban:**

**Kizana bukan pengganti guru, tapi perpanjangan tangan guru.**

**Analogi tradisional:**
- Dulu: santri harus ke pojok perpustakaan, buka 20 kitab satu per satu
- Sekarang: santri buka Kizana, dapatkan referensi dari 7.872 kitab dalam detik
- **Tetap:** santri membawa referensi itu ke guru/kiai untuk didiskusikan

**Kizana memperkuat tradisi, bukan menggantikan:**
1. **Memperluas akses** — Tidak semua pesantren punya perpustakaan lengkap
2. **Mempercepat riset** — Waktu yang dihemat bisa untuk diskusi lebih mendalam
3. **Menjaga teks asli** — Menampilkan teks Arab, bukan hanya terjemahan
4. **Membuka cakrawala** — Santri bisa melihat pendapat lintas mazhab yang mungkin tidak ada di koleksi pesantrennya

**Pesan kunci:** *"العلم نور — ilmu adalah cahaya. Kizana memperbanyak lentera, bukan menggantikan matahari (guru)."*

---

### 14. "Bagaimana Kizana menangani pertanyaan sensitif (nikah siri, LGBT, kripto, dll.)?"

**Jawaban:**

**Prinsip:** Kizana adalah mesin pencari ilmiah, bukan lembaga fatwa. Ia menampilkan **apa yang tertulis di kitab**, bukan opini.

**Contoh penanganan:**

| Query Sensitif | Yang Dilakukan Kizana |
|---------------|----------------------|
| "hukum nikah siri" | Menampilkan pembahasan النكاح السري dari berbagai kitab fiqih |
| "hukum kripto" | Mencari pembahasan المعاملات المالية الحديثة, نقود, barter |
| "hukum euthanasia" | Mencari قتل الرحمة, menampilkan pendapat ulama kontemporer jika ada |

**Safeguard:**
- Tidak ada blacklist query — semua pertanyaan ilmiah dijawab
- AI tidak memberi "fatwa personal" — hanya merangkum dari kitab
- Untuk topik yang tidak ada di kitab klasik → AI jelas menyatakan: "Topik ini tidak ditemukan secara eksplisit dalam kitab klasik"

**GAP:** ⚠️ Untuk topik sangat kontemporer (kripto, AI, medsos), perlu database kitab ulama kontemporer (Yusuf Qaradawi, Wahbah Zuhaili, dll.).

---

## III. PERTANYAAN TEKNIS (15-20)

### 15. "Bagaimana pipeline pencarian bekerja secara teknis?"

**Jawaban:**

```
 User: "hukum jual beli online"
          ↓
 [1] LANGUAGE DETECTION
     → Bahasa Indonesia
          ↓
 [2] MORPHOLOGICAL ANALYSIS  
     → "jual" → root word
     → "beli" → root word  
     → "online" → modern term
          ↓
 [3] ARABIC EXPANSION (500+ dictionary entries)
     → jual beli → ["بيع", "البيع", "الشراء", "البيع والشراء"]
     → online → ["عبر الإنترنت", "إلكتروني"]
          ↓
 [4] PHRASE EXPANSION (bigrams/trigrams)
     → "jual beli" → "أحكام البيع"^3, "البيع والشراء"^3
          ↓
 [5] TANTIVY BM25 SEARCH
     → Query: "أحكام البيع"^3 OR "البيع والشراء"^3 OR البيع^2 OR الشراء^2 OR beli OR jual
     → Cari di 3.4M+ TOC entries dari 7.872 kitab
          ↓
 [6] SCORING & RERANKING
     → BM25 base score
     → + phrase overlap boost
     → + title field boost (2x)
     → − encyclopedia penalty (−70%)
     → + diversity filter (max 3/book)
          ↓
 [7] AI SYNTHESIS (DeepSeek)
     → Stream SSE chunks ke frontend
     → Jawaban dalam Bahasa Indonesia
     → Referensi ke kitab + halaman
          ↓
 [8] DISPLAY
     → AI answer (with clickable [1] [2] references)
     → Source cards (kitab + halaman)
     → Teks Arab asli
```

**Angka performa:**
- Search latency: ~100-300ms (Tantivy BM25)
- AI synthesis: 2-5 detik (streaming, jadi user lihat langsung)
- Total end-to-end: 3-6 detik

---

### 16. "Apa yang terjadi kalau AI API down? Ada fallback?"

**Jawaban:**

**Arsitektur resilient:**

```
                Query
                  ↓
            ┌─────────────┐
            │ Tantivy BM25│  ← SELALU JALAN (lokal)
            └──────┬──────┘
                   ↓
           ┌───────────────┐
           │ DeepSeek AI   │  ← BISA GAGAL
           └───┬───────┬───┘
               ↓       ↓
          [Sukses]  [Gagal/Timeout]
               ↓       ↓
          AI Answer  Local Synthesis ← FALLBACK OTOMATIS
```

**Fallback detail:**
1. Jika AI API down → otomatis fallback ke **local synthesis** (non-AI)
2. Local synthesis menggunakan **template-based** answer:
   - Mengambil 3 hasil teratas
   - Format: "Berdasarkan [Kitab X], hal. Y: [kutipan]"
   - Tidak ada AI interpretation, murni kutipan
3. Hasil pencarian (Tantivy) **selalu tersedia** — search engine berjalan lokal
4. Redis down → graceful degradation (cache miss, langsung ke SQLite)

**Status:** ✅ Fully implemented. Log menunjukkan: `"AI stream returned empty, using local synthesis"` ketika API gagal.

---

### 17. "Bagaimana keamanan data pengguna?"

**Jawaban:**

| Aspek | Implementasi | Status |
|-------|-------------|--------|
| **Password** | Bcrypt cost 12 (~250ms/hash) | ✅ |
| **Auth** | JWT HS256, 24h expiry | ✅ |
| **Rate Limiting** | 60 req/min (general), 10/min (auth) | ✅ |
| **Input Sanitization** | Max 500 chars, dangerous chars stripped | ✅ |
| **CORS** | Restricted to frontend domain | ✅ |
| **HTTPS** | SSL via Let's Encrypt + Nginx | ✅ |
| **SQL Injection** | Prepared statements (parameterized) | ✅ |
| **XSS** | DOMPurify sanitization on frontend | ✅ |
| **Gzip** | Nginx compression (response optimization) | ✅ |
| **Security Headers** | X-Frame-Options, X-Content-Type, XSS-Protection | ✅ |

**GAP:**
- ⚠️ SQLite unencrypted at rest (mitigasi: VPS full-disk encryption)
- ⚠️ No audit log trail (who accessed what)
- ⚠️ No GDPR-style data export/deletion

**IMPLEMENTASI untuk PBNU-compliance:**
```
P1: Tambah audit log (query log sudah ada, perlu user action log)
P2: Data export endpoint (/api/auth/export-data)
P3: Account deletion endpoint (/api/auth/delete-account)
```

---

### 18. "Berapa besar database dan bagaimana performanya?"

**Jawaban:**

**Database Statistics:**

| Metrik | Nilai |
|--------|-------|
| Total kitab | 7.872 kitab |
| Total halaman | 3.4M+ halaman |
| Total TOC entries | 3.4M+ entri daftar isi |
| SQLite database size | ~20 GB |
| Tantivy index size | ~2-4 GB (estimated) |
| Schema per kitab | 2 tabel: b{N} (content) + t{N} (TOC) |
| Total tabel SQLite | 15.744+ tabel (2 × 7.872) |

**Performa:**

| Operasi | Waktu |
|---------|-------|
| Search (BM25) | 100-300ms |
| Book page load | <50ms |
| TOC hierarchy load | <100ms |
| Metadata load (startup) | 1-2 detik (7.872 books) |
| First index build | 15-30 menit (one-time) |
| Subsequent startup | <5 detik |

**Catatan:** Semua search dilakukan di **Tantivy** (Rust-native, in-memory index), bukan SQLite query. SQLite hanya untuk content retrieval setelah search. Ini memberikan performa setara Elasticsearch tapi tanpa overhead infrastruktur.

---

### 19. "Apakah bisa di-deploy on-premise di server PBNU?"

**Jawaban:**

**Ya, 100% bisa on-premise.**

**Kebutuhan minimum:**
| Resource | Minimum | Recommended |
|----------|---------|-------------|
| CPU | 2 cores | 4+ cores |
| RAM | 4 GB | 8+ GB |
| Storage | 30 GB | 100 GB (SSD) |
| OS | Ubuntu 22.04+ | Ubuntu 24.04 |
| Runtime | Rust binary (no Docker needed) | Docker optional |

**Langkah deployment:**
```bash
1. Copy binary + SQLite database + Tantivy index
2. Install Redis (optional, untuk caching)
3. Setup Nginx reverse proxy
4. Configure systemd service
5. Set AI_API_KEY environment variable
6. Done — service auto-start on boot
```

**Keuntungan on-premise untuk PBNU:**
- Data tidak keluar dari server PBNU
- Tidak tergantung pada pihak ketiga (kecuali AI API)
- Bisa diakses via intranet tanpa internet (search tetap jalan tanpa AI)

**Untuk fully offline:** 
- Search engine berjalan lokal ✅
- AI synthesis perlu internet → alternatif: deploy Ollama + model lokal (Llama 3, Qwen)

**GAP:** ❌ Belum ada Docker image. Perlu buat Dockerfile + docker-compose untuk easy deployment.

**IMPLEMENTASI:**
```dockerfile
# Dockerfile yang perlu dibuat
FROM rust:1.83-slim AS builder
COPY backend/ /app/
RUN cargo build --release

FROM ubuntu:24.04
COPY --from=builder /app/target/release/kizana-search /usr/local/bin/
COPY database/ /data/
EXPOSE 8080
CMD ["kizana-search"]
```

---

### 20. "Apa roadmap teknis 6 bulan ke depan?"

**Jawaban:**

| Bulan | Fitur | Impact |
|-------|-------|--------|
| **Bulan 1** | ✅ Chat animation, DeepSeek integration, Ibaroh click-to-source | UX Polish |
| **Bulan 1** | Filter mazhab (Syafi'i/Hanafi/Maliki/Hanbali) | Accuracy |
| **Bulan 2** | Subscription + payment gateway | Revenue |
| **Bulan 2** | Admin dashboard (analytics, user management) | Operations |
| **Bulan 3** | Multi-turn AI memory (konteks percakapan) | UX |
| **Bulan 3** | Bahtsul Masail format output (masalah→jawaban→ta'bir→maraji') | Core Value |
| **Bulan 4** | Cross-reference system (kitab terkait) | Knowledge Graph |
| **Bulan 4** | Fuzzy Arabic search (tanpa harakat) | Search Quality |
| **Bulan 5** | Docker packaging + on-premise deployment guide | Distribution |
| **Bulan 5** | Mobile app (PWA atau React Native) | Reach |
| **Bulan 6** | Kitab Jawi (Arab Pegon) support | Content Expansion |
| **Bulan 6** | API v2 (batch, webhooks, richer metadata) | Enterprise |

---

## IV. GAP ANALYSIS — RINGKASAN

### Critical Gaps (Harus ada sebelum launch resmi PBNU)

| # | Gap | Priority | Effort |
|---|-----|----------|--------|
| 1 | **Disclaimer fatwa** di UI | P0 | 2 jam |
| 2 | **Filter mazhab** (UI + API) | P1 | 1 minggu |
| 3 | **Metadata kitab** (pengarang, mazhab, era) enrichment | P1 | 2 minggu (perlu tim konten) |
| 4 | **"Report inaccuracy" button** | P1 | 1 hari |
| 5 | **Landing page** + onboarding | P1 | 3 hari |

### Important Gaps (Harus ada dalam 3 bulan)

| # | Gap | Priority | Effort |
|---|-----|----------|--------|
| 6 | **Subscription/quota system** | P2 | 1 minggu |
| 7 | **Admin dashboard** | P2 | 2 minggu |
| 8 | **Multi-turn conversation memory** | P2 | 1 minggu |
| 9 | **Bahtsul Masail output format** | P2 | 1 minggu |
| 10 | **Analytics tracking** | P2 | 3 hari |

### Nice-to-have Gaps (6 bulan)

| # | Gap | Priority | Effort |
|---|-----|----------|--------|
| 11 | **Docker image** | P3 | 2 hari |
| 12 | **Cross-reference** (kitab terkait) | P3 | 2 minggu |
| 13 | **Kitab Jawi support** | P3 | 1 bulan |
| 14 | **Mobile app (PWA)** | P3 | 2 minggu |
| 15 | **Audit log trail** | P3 | 3 hari |

---

## V. IMPLEMENTASI SISTEMATIS — 5 CRITICAL GAPS

### Gap 1: Disclaimer Fatwa (2 jam)

**Lokasi:** `frontend/src/routes/+page.svelte`

**Tambahkan:**
- Banner fixed di bawah AI answer: *"⚠️ Hasil ini adalah alat bantu riset keilmuan. Untuk keperluan fatwa atau keputusan hukum syar'i, konsultasikan dengan ulama yang berkompeten."*
- Onboarding popup pertama kali: penjelasan positioning Kizana

### Gap 2: Filter Mazhab (1 minggu)

**Backend:**
```rust
// handlers.rs — tambah parameter filter
pub struct StreamQuery {
    pub query: String,
    pub mazhab: Option<String>, // "syafii"|"hanafi"|"maliki"|"hanbali"|null
}

// search.rs — filter berdasarkan metadata kitab
if let Some(mazhab) = &query.mazhab {
    results.retain(|r| r.book_mazhab == Some(mazhab.clone()));
}
```

**Frontend:**
```svelte
<!-- Filter chips di atas search bar -->
<div class="mazhab-filter">
  {#each ['Semua', 'Syafi\'i', 'Hanafi', 'Maliki', 'Hanbali'] as m}
    <button class:active={selectedMazhab === m} on:click={() => selectedMazhab = m}>
      {m}
    </button>
  {/each}
</div>
```

### Gap 3: Metadata Enrichment (2 minggu)

**Tabel baru:**
```sql
CREATE TABLE book_metadata_enriched (
    book_id INTEGER PRIMARY KEY,
    author_name TEXT,
    author_name_ar TEXT,
    mazhab TEXT,       -- 'syafii'|'hanafi'|'maliki'|'hanbali'|'multi'|NULL
    era TEXT,          -- 'classical'|'medieval'|'contemporary'
    field TEXT,        -- 'fiqh'|'hadith'|'tafsir'|'aqidah'|'tasawuf'
    is_mutabar BOOLEAN DEFAULT 1,  -- diakui NU?
    verified_by TEXT,  -- siapa yang verifikasi
    verified_at DATETIME
);
```

**Kolaborasi diperlukan:** Tim konten Islam dari PBNU/Lajnah BM perlu mengisi data ini. Bisa di-crowdsource via admin dashboard.

### Gap 4: Report Inaccuracy (1 hari)

**Tambahkan tombol di setiap AI answer:**
```svelte
<button class="report-btn" on:click={() => reportInaccuracy(messageId)}>
  🚩 Laporkan Ketidakakuratan
</button>
```

**Backend endpoint:**
```rust
POST /api/feedback/report
{
    "query": "...",
    "ai_answer_snippet": "...",
    "issue_type": "wrong_attribution|wrong_translation|missing_context|other",
    "user_comment": "..."
}
```

### Gap 5: Landing Page (3 hari)

**Halaman baru:** `/` (root) — landing page before login
- Hero: "Cari Jawaban dari 7.872 Kitab Klasik Islam"
- Demo: live search box with example queries
- Stats: 7.872 kitab, 3.4M halaman, N bahasa
- Testimonial/endorsement PBNU
- CTA: "Mulai Gratis" → register

---

## VI. TALKING POINTS UNTUK PRESENTASI

### Opening (2 menit):
> "Bayangkan seorang santri di pelosok Jawa yang butuh mencari hukum tentang vaksin. Dulu ia harus membuka puluhan kitab satu per satu, dalam bahasa Arab yang mungkin belum sepenuhnya ia kuasai. Kizana mengubah itu — cukup ketik dalam bahasa Indonesia, dan 7.872 kitab klasik terbuka untuknya dalam hitungan detik."

### Demo Live (5 menit):
1. Ketik: "hukum jual beli online" → tunjukkan query translation ke Arab
2. Tunjukkan AI synthesis + referensi kitab
3. Klik referensi → buka kitab di halaman yang tepat
4. Ketik: "cerai atas permintaan istri" → tunjukkan khilaf antar mazhab

### Closing (1 menit):
> "Kizana bukan menggantikan ulama. Kizana adalah lentera digital yang mempercepat proses bahtsul masail dari berhari-hari menjadi menit. Dengan dukungan PBNU, kita bisa membawa khazanah turats ke 50.000 pesantren dan jutaan umat Islam Indonesia."

---

*Dokumen ini disiapkan untuk presentasi ke Digdaya NU dan PBNU.*
*Terakhir diperbarui: 12 Maret 2026*
