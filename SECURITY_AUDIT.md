# 🔒 Laporan Audit Keamanan Siber — Bahtsul Masail

**Tanggal:** Juni 2025  
**Cakupan:** Backend Actix-Web (Rust) + Frontend SvelteKit  
**Total File Diaudit:** 11 file sumber + konfigurasi nginx  

---

## Ringkasan Eksekutif

Audit ini mengidentifikasi **28 temuan keamanan** di seluruh stack Bahtsul Masail:
- **2 CRITICAL** — harus segera diperbaiki
- **6 HIGH** — diperbaiki minggu ini
- **12 MEDIUM** — diperbaiki sprint berikutnya
- **8 LOW** — backlog

---

## Temuan CRITICAL

### 1.1 — JWT Secret Hardcoded Default
**Severity:** CRITICAL | **File:** `config.rs`

JWT secret memiliki default hardcoded: `"kizana_secret_key_change_in_production_2026"`. Jika env var `JWT_SECRET` tidak diset, semua token ditandatangani dengan kunci yang diketahui publik. Attacker bisa memalsukan JWT arbitrary.

**Remediasi:** Force env var, minimum 32 karakter:
```rust
jwt_secret: std::env::var("JWT_SECRET")
    .expect("FATAL: JWT_SECRET environment variable must be set"),
```

### 5.1 — Production Nginx Tanpa HTTPS
**Severity:** CRITICAL | **File:** `nginx_kizana.conf`

Konfigurasi nginx produksi hanya listen port 80. Semua traffic (JWT token, password, API keys) terkirim plaintext.

**Remediasi:** Deploy SSL via Let's Encrypt + redirect HTTP → HTTPS.

---

## Temuan HIGH

### 1.4 — Token Tidak Diinvalidasi Saat Ganti Password
JWT tetap valid 7 hari setelah password diubah. Token curian masih bisa dipakai.

### 1.5 — Ubah Email Tanpa Konfirmasi Password
`update_profile` membolehkan ganti email tanpa password saat ini. Attacker XSS bisa ambil alih akun.

### 2.2 — XSS via Markdown Rendering
`{@html marked.parse(text)}` tanpa sanitasi. Output AI dan database bisa mengandung `<script>` tags.

**Remediasi:** Install `dompurify`, sanitasi semua output `{@html}`.

### 3.1 — Rate Limit Bypass via X-Forwarded-For Spoofing
Client bisa mengirim header `X-Forwarded-For` palsu untuk bypass rate limiting.

### 3.6 — Nginx CORS Reflects Any Origin
`add_header Access-Control-Allow-Origin $http_origin` merefleksi origin apapun, mem-bypass CORS Actix.

### 4.1 — JWT di localStorage (XSS-accessible)
Token disimpan di localStorage yang bisa diakses oleh JavaScript (termasuk XSS payload).

---

## Temuan MEDIUM

| # | Temuan | Kategori |
|---|---|---|
| 1.2 | JWT 7 hari tanpa mekanisme revokasi | Auth |
| 2.1 | Dynamic SQL table names (mitigasi: i64 type) | Injection |
| 2.3 | XSS di fungsi highlightContent | Injection |
| 2.4 | XSS di export PDF via document.write | Injection |
| 3.2 | CORS izinkan localhost di produksi | API |
| 3.3 | Tidak ada limit panjang query | API |
| 3.4 | Error message bocorkan informasi internal | API |
| 3.5 | Tidak ada explicit JSON payload limit | API |
| 4.3 | Email tercatat plaintext di log | Data |
| 5.2 | Tidak ada Content-Security-Policy | Infra |
| 6.1 | Tidak ada rate limit per user | Bisnis |
| 6.2 | Per-key rate limit tidak dienforce | Bisnis |
| 6.5 | Bcrypt per-request (DoS vector) | Bisnis |
| 7.1 | Tidak ada cargo audit untuk dependency | Deps |

---

## Temuan LOW

| # | Temuan | Kategori |
|---|---|---|
| 1.3 | JWT algorithm tidak di-pin eksplisit | Auth |
| 1.6 | API key bcrypt cost hardcoded 10 | Auth |
| 4.2 | AI API key plaintext di memori | Data |
| 5.3 | Missing HSTS, Referrer-Policy headers | Infra |
| 6.3 | Session count per user tidak dibatasi | Bisnis |
| 6.4 | Tidak ada limit jumlah API key per user | Bisnis |

---

## Prioritas Remediasi

1. **Segera:** Fix 1.1 (JWT secret) + 5.1 (HTTPS)
2. **Minggu ini:** Fix 2.2 (XSS), 4.1 (localStorage), 3.6 (CORS nginx), 3.1 (IP spoofing)
3. **Sprint berikutnya:** Fix 1.4, 1.5, 3.3, 3.4, dan temuan MEDIUM lainnya
4. **Backlog:** Temuan LOW

---

*Ditulis sebagai bagian dari audit keamanan komprehensif Bahtsul Masail.*
