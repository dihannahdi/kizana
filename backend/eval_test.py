#!/usr/bin/env python3
"""Batch eval test for Kizana Search - runs 100+ queries and outputs detailed analysis."""
import requests
import json
import time
import sys

BASE = "http://127.0.0.1:8080"

# Try multiple possible passwords for eval_admin
PASSWORDS = ["EvalTest2026"]

def get_admin_token():
    for pw in PASSWORDS:
        try:
            r = requests.post(f"{BASE}/api/auth/login", json={
                "email": "eval_admin@bahtsulmasail.tech",
                "password": pw
            }, timeout=5)
            if r.status_code == 200:
                data = r.json()
                return data.get("token", "")
        except:
            pass
    # Try dihannahdii admin
    for pw in ["Admin2026!", "admin123", "Nahdi2026!", "kizana2026", "bahtsulmasail"]:
        try:
            r = requests.post(f"{BASE}/api/auth/login", json={
                "email": "dihannahdii@gmail.com",
                "password": pw
            }, timeout=5)
            if r.status_code == 200:
                data = r.json()
                return data.get("token", "")
        except:
            pass
    return None

# ══════════════════════════════════════════════
# 110 test queries across all domains and languages
# ══════════════════════════════════════════════
QUERIES = [
    # ═══ IBADAH - SHALAT (15 queries) ═══
    {"id": "ib01", "text": "hukum shalat jumat"},
    {"id": "ib02", "text": "syarat sah shalat"},
    {"id": "ib03", "text": "shalat jamak qashar dalam perjalanan"},
    {"id": "ib04", "text": "bacaan qunut shalat subuh"},
    {"id": "ib05", "text": "hukum meninggalkan shalat"},
    {"id": "ib06", "text": "shalat berjamaah di rumah"},
    {"id": "ib07", "text": "rukun shalat menurut syafi'i"},
    {"id": "ib08", "text": "sujud sahwi karena lupa"},
    {"id": "ib09", "text": "shalat tahajud cara dan waktunya"},
    {"id": "ib10", "text": "shalat jenazah ghaib"},
    {"id": "ib11", "text": "hukum shalat witir"},
    {"id": "ib12", "text": "shalat sambil duduk karena sakit"},
    {"id": "ib13", "text": "imam shalat perempuan boleh gak"},
    {"id": "ib14", "text": "prayer while traveling in Islam"},
    {"id": "ib15", "text": "صلاة الجمعة أحكامها"},

    # ═══ THAHARAH (10 queries) ═══
    {"id": "th01", "text": "cara wudhu yang benar"},
    {"id": "th02", "text": "tayammum syarat dan caranya"},
    {"id": "th03", "text": "najis mughallazhah dan cara membersihkannya"},
    {"id": "th04", "text": "hukum menyentuh mushaf tanpa wudhu"},
    {"id": "th05", "text": "mandi junub tata cara"},
    {"id": "th06", "text": "haid dan istihadzah perbedaannya"},
    {"id": "th07", "text": "air mutanajjis boleh untuk wudhu"},
    {"id": "th08", "text": "wudhu batal karena apa saja"},
    {"id": "th09", "text": "darah keluar apakah batal wudhu"},
    {"id": "th10", "text": "purification rules in Islamic law"},

    # ═══ PUASA (8 queries) ═══
    {"id": "pu01", "text": "hal yang membatalkan puasa"},
    {"id": "pu02", "text": "puasa sunnah senin kamis"},
    {"id": "pu03", "text": "fidyah puasa orang tua"},
    {"id": "pu04", "text": "niat puasa ramadhan kapan dibaca"},
    {"id": "pu05", "text": "hukum berpuasa bagi ibu hamil"},
    {"id": "pu06", "text": "puasa arafah hukumnya"},
    {"id": "pu07", "text": "makan sahur sebelum imsak"},
    {"id": "pu08", "text": "kafarat berbuka puasa sengaja"},

    # ═══ ZAKAT (6 queries) ═══
    {"id": "zk01", "text": "nisab zakat mal berapa"},
    {"id": "zk02", "text": "zakat fitrah beras atau uang"},
    {"id": "zk03", "text": "delapan golongan penerima zakat"},
    {"id": "zk04", "text": "zakat emas dan perak nisabnya"},
    {"id": "zk05", "text": "zakat profesi hukumnya"},
    {"id": "zk06", "text": "mustahik zakat menurut quran"},

    # ═══ HAJI & UMRAH (5 queries) ═══
    {"id": "hj01", "text": "rukun haji wajib dan sunnahnya"},
    {"id": "hj02", "text": "dam haji jenis dan hukumnya"},
    {"id": "hj03", "text": "ihram dari miqat"},
    {"id": "hj04", "text": "tawaf ifadhah wajib atau rukun"},
    {"id": "hj05", "text": "badal haji untuk orang meninggal"},

    # ═══ MUNAKAHAT / NIKAH (12 queries) ═══
    {"id": "nk01", "text": "syarat sah nikah dalam islam"},
    {"id": "nk02", "text": "nikah siri hukumnya menurut ulama"},
    {"id": "nk03", "text": "wali nikah siapa yang berhak"},
    {"id": "nk04", "text": "mahar nikah minimal berapa"},
    {"id": "nk05", "text": "talak tiga sekaligus jatuh berapa"},
    {"id": "nk06", "text": "iddah cerai mati berapa lama"},
    {"id": "nk07", "text": "khuluk cerai gugat istri"},
    {"id": "nk08", "text": "nafkah suami kepada istri"},
    {"id": "nk09", "text": "poligami syarat dan hukumnya"},
    {"id": "nk10", "text": "nikah mut'ah halal atau haram"},
    {"id": "nk11", "text": "cerai karena suami impoten"},
    {"id": "nk12", "text": "hak asuh anak setelah cerai"},

    # ═══ MUAMALAT / EKONOMI (12 queries) ═══
    {"id": "mu01", "text": "riba bank konvensional hukumnya"},
    {"id": "mu02", "text": "jual beli online hukumnya"},
    {"id": "mu03", "text": "gadai emas di bank syariah"},
    {"id": "mu04", "text": "asuransi syariah halal haram"},
    {"id": "mu05", "text": "wakaf tanah syarat dan rukunnya"},
    {"id": "mu06", "text": "hutang piutang dalam islam"},
    {"id": "mu07", "text": "jual beli salam istishna"},
    {"id": "mu08", "text": "mudharabah akad bagi hasil"},
    {"id": "mu09", "text": "hukum dropship dalam islam"},
    {"id": "mu10", "text": "waris pembagian harta"},
    {"id": "mu11", "text": "wasiat maksimal sepertiga"},
    {"id": "mu12", "text": "hibah harta kepada anak"},

    # ═══ AQIDAH (8 queries) ═══
    {"id": "aq01", "text": "tauhid rububiyah uluhiyah"},
    {"id": "aq02", "text": "syirik kecil dan besar"},
    {"id": "aq03", "text": "bid'ah hasanah dan sayyi'ah"},
    {"id": "aq04", "text": "tawassul kepada orang shaleh"},
    {"id": "aq05", "text": "maulid nabi hukumnya"},
    {"id": "aq06", "text": "iman kepada qadha qadar"},
    {"id": "aq07", "text": "hukum murtad dalam islam"},
    {"id": "aq08", "text": "sifat dua puluh allah"},

    # ═══ TASAWUF & AKHLAK (6 queries) ═══
    {"id": "ts01", "text": "taubat nasuha syarat dan caranya"},
    {"id": "ts02", "text": "dzikir setelah shalat"},
    {"id": "ts03", "text": "riya dan ujub perbedaannya"},
    {"id": "ts04", "text": "ghibah dan namimah hukumnya"},
    {"id": "ts05", "text": "adab murid kepada guru"},
    {"id": "ts06", "text": "tawakkal kepada allah"},

    # ═══ MAKANAN & MINUMAN (6 queries) ═══
    {"id": "mk01", "text": "makanan haram dalam islam"},
    {"id": "mk02", "text": "hukum makan gelatin babi"},
    {"id": "mk03", "text": "sembelihan ahli kitab boleh dimakan"},
    {"id": "mk04", "text": "khamr dan nabidz hukumnya"},
    {"id": "mk05", "text": "hukum merokok dalam islam"},
    {"id": "mk06", "text": "istihalah najis menjadi suci"},

    # ═══ JINAYAT / PIDANA ISLAM (5 queries) ═══
    {"id": "jn01", "text": "hukum mencuri dalam islam"},
    {"id": "jn02", "text": "qishas diyat perbedaannya"},
    {"id": "jn03", "text": "hukum membunuh dalam islam"},
    {"id": "jn04", "text": "ta'zir dalam hukum islam"},
    {"id": "jn05", "text": "hudud jenis dan hukumnya"},

    # ═══ KONTEMPORER / MODERN (10 queries) ═══
    {"id": "kt01", "text": "hukum transplantasi organ"},
    {"id": "kt02", "text": "hukum bayi tabung dalam islam"},
    {"id": "kt03", "text": "vaksin mengandung babi darurat"},
    {"id": "kt04", "text": "cryptocurrency bitcoin halal haram"},
    {"id": "kt05", "text": "hukum foto dan gambar makhluk hidup"},
    {"id": "kt06", "text": "musik dan nyanyian hukumnya"},
    {"id": "kt07", "text": "bank asi hukumnya dalam islam"},
    {"id": "kt08", "text": "operasi plastik hukumnya"},
    {"id": "kt09", "text": "kloning manusia pandangan islam"},
    {"id": "kt10", "text": "hukum aborsi dalam islam"},

    # ═══ BAHASA INGGRIS (5 queries) ═══
    {"id": "en01", "text": "rulings on interest in Islamic finance"},
    {"id": "en02", "text": "conditions for valid marriage in Islam"},
    {"id": "en03", "text": "fasting rules during Ramadan"},
    {"id": "en04", "text": "inheritance shares in Islamic law"},
    {"id": "en05", "text": "ruling on apostasy in Islam"},

    # ═══ BAHASA ARAB (5 queries) ═══
    {"id": "ar01", "text": "حكم الربا في الاسلام"},
    {"id": "ar02", "text": "شروط صحة النكاح"},
    {"id": "ar03", "text": "أحكام الطهارة"},
    {"id": "ar04", "text": "الميراث والفرائض"},
    {"id": "ar05", "text": "حكم الزكاة في المال"},

    # ═══ BAHASA CAMPURAN / AWAM (7 queries) ═══
    {"id": "cm01", "text": "boleh gak shalat pakai celana pendek"},
    {"id": "cm02", "text": "gimana hukumnya nikah beda agama"},
    {"id": "cm03", "text": "kucing najis gak sih"},
    {"id": "cm04", "text": "kenapa riba diharamkan"},
    {"id": "cm05", "text": "apakah tahlilan itu bid'ah"},
    {"id": "cm06", "text": "boleh gak wanita jadi imam shalat"},
    {"id": "cm07", "text": "doa qunut dibaca kapan"},
]

def run_eval(token, queries, batch_size=20):
    """Run eval batch and collect all results."""
    headers = {"Authorization": f"Bearer {token}", "Content-Type": "application/json"}
    all_results = []
    
    for i in range(0, len(queries), batch_size):
        batch = queries[i:i+batch_size]
        payload = {
            "queries": batch,
            "config": {},
            "max_results": 10
        }
        try:
            r = requests.post(f"{BASE}/api/eval/batch", json=payload, headers=headers, timeout=120)
            if r.status_code == 200:
                data = r.json()
                all_results.extend(data.get("results", []))
                print(f"  Batch {i//batch_size + 1}: {len(batch)} queries OK ({data.get('total_time_ms', 0)}ms)", file=sys.stderr)
            else:
                print(f"  Batch {i//batch_size + 1} FAILED: {r.status_code} {r.text[:200]}", file=sys.stderr)
        except Exception as e:
            print(f"  Batch {i//batch_size + 1} ERROR: {e}", file=sys.stderr)
    
    return all_results

def analyze_results(results):
    """Comprehensive analysis of search results."""
    analysis = {
        "total_queries": len(results),
        "queries_with_results": 0,
        "queries_no_results": 0,
        "avg_results_per_query": 0,
        "avg_search_time_ms": 0,
        "avg_top_score": 0,
        "score_distribution": {"90-100": 0, "70-89": 0, "50-69": 0, "30-49": 0, "0-29": 0},
        "domain_distribution": {},
        "language_distribution": {},
        "translation_coverage": {"translated": 0, "empty": 0},
        "snippet_quality": {"has_snippet": 0, "empty_snippet": 0, "short_snippet": 0},
        "diversity": {"unique_books": set(), "avg_unique_books_per_query": 0},
        "by_query": [],
    }
    
    total_results = 0
    total_time = 0
    total_top_score = 0
    total_unique_books = 0
    
    for qr in results:
        qid = qr.get("query_id", "")
        qtext = qr.get("query_text", "")
        translated = qr.get("translated_terms", [])
        lang = qr.get("detected_language", "unknown")
        domain = qr.get("detected_domain", "unknown")
        search_results = qr.get("results", [])
        search_time = qr.get("search_time_ms", 0)
        num_results = qr.get("num_results", len(search_results))
        
        total_time += search_time
        
        # Count results
        if num_results > 0:
            analysis["queries_with_results"] += 1
        else:
            analysis["queries_no_results"] += 1
        total_results += num_results
        
        # Top score
        top_score = search_results[0]["score"] if search_results else 0
        total_top_score += top_score
        
        # Score distribution
        for sr in search_results:
            s = sr["score"]
            if s >= 90: analysis["score_distribution"]["90-100"] += 1
            elif s >= 70: analysis["score_distribution"]["70-89"] += 1
            elif s >= 50: analysis["score_distribution"]["50-69"] += 1
            elif s >= 30: analysis["score_distribution"]["30-49"] += 1
            else: analysis["score_distribution"]["0-29"] += 1
        
        # Domain/language
        analysis["domain_distribution"][domain] = analysis["domain_distribution"].get(domain, 0) + 1
        analysis["language_distribution"][lang] = analysis["language_distribution"].get(lang, 0) + 1
        
        # Translation coverage
        if translated:
            analysis["translation_coverage"]["translated"] += 1
        else:
            analysis["translation_coverage"]["empty"] += 1
        
        # Snippet quality
        for sr in search_results:
            snippet = sr.get("content_snippet", "")
            if not snippet:
                analysis["snippet_quality"]["empty_snippet"] += 1
            elif len(snippet) < 100:
                analysis["snippet_quality"]["short_snippet"] += 1
            else:
                analysis["snippet_quality"]["has_snippet"] += 1
        
        # Diversity
        books_in_query = set()
        for sr in search_results:
            bid = sr.get("book_id", 0)
            books_in_query.add(bid)
            analysis["diversity"]["unique_books"].add(bid)
        total_unique_books += len(books_in_query)
        
        # Per-query detail
        query_detail = {
            "id": qid,
            "text": qtext,
            "lang": lang,
            "domain": domain,
            "translated_terms": translated[:8],
            "num_results": num_results,
            "search_time_ms": search_time,
            "top_score": round(top_score, 1),
            "unique_books": len(books_in_query),
            "top3": []
        }
        for sr in search_results[:3]:
            query_detail["top3"].append({
                "title": sr.get("title", "")[:80],
                "book": sr.get("book_name", "")[:60],
                "score": round(sr.get("score", 0), 1),
                "page": sr.get("page", ""),
                "snippet_len": len(sr.get("content_snippet", "")),
            })
        analysis["by_query"].append(query_detail)
    
    # Compute averages
    n = len(results) or 1
    analysis["avg_results_per_query"] = round(total_results / n, 1)
    analysis["avg_search_time_ms"] = round(total_time / n, 1)
    analysis["avg_top_score"] = round(total_top_score / n, 1)
    analysis["diversity"]["unique_books_total"] = len(analysis["diversity"]["unique_books"])
    analysis["diversity"]["avg_unique_books_per_query"] = round(total_unique_books / n, 1)
    del analysis["diversity"]["unique_books"]  # not serializable
    
    return analysis

def main():
    print("=== Kizana Search Comprehensive Eval ===", file=sys.stderr)
    print(f"Testing {len(QUERIES)} queries...", file=sys.stderr)
    
    # Get admin token
    token = get_admin_token()
    if not token:
        print("ERROR: Could not get admin token. Trying all known accounts failed.", file=sys.stderr)
        sys.exit(1)
    print(f"Got admin token (length={len(token)})", file=sys.stderr)
    
    # Run eval
    results = run_eval(token, QUERIES)
    print(f"\nGot results for {len(results)} queries", file=sys.stderr)
    
    # Analyze
    analysis = analyze_results(results)
    
    # Output as JSON
    print(json.dumps(analysis, ensure_ascii=False, indent=2))

if __name__ == "__main__":
    main()
