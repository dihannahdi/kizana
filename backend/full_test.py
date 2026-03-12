#!/usr/bin/env python3
"""Comprehensive test of all key queries with correct API field names."""
import requests, json

BASE = "http://localhost:8080"
login = requests.post(f"{BASE}/api/auth/login", json={
    "email": "eval_admin@bahtsulmasail.tech",
    "password": "EvalTest2026"
})
token = login.json().get("token", "")
headers = {"Authorization": f"Bearer {token}"}
print(f"Auth: {'OK' if token else 'FAILED'}\n")

queries = [
    ("ihram dari miqat", ["إحرام","ميقات"], "عبادات"),
    ("adab murid kepada guru", ["آداب","متعلم"], "أخلاق"),
    ("operasi plastik hukumnya", ["جراحة","تجميل"], None),
    ("imam shalat perempuan boleh gak", ["إمامة","المرأة"], "عبادات"),
    ("zakat emas dan perak nisabnya", ["زكاة","ذهب"], "عبادات"),
    ("iman kepada qadha qadar", ["القضاء","القدر"], "عقيدة"),
    ("sifat dua puluh allah", ["صفات","العشرون"], None),
    ("boleh gak shalat pakai celana pendek", ["عورة","ستر"], "عبادات"),
    ("gimana hukumnya nikah beda agama", ["اختلاف","كتابية"], None),
    ("hal yang membatalkan puasa", ["مبطلات","صيام","مفسدات"], "عبادات"),
    ("hal yang membatalkan wudhu", ["نواقض","وضوء"], "طهارة"),
    ("hukum nikah siri", ["نكاح","سري","ولي"], "مناكحات"),
    ("hukum riba dalam islam", ["ربا","فائدة"], "معاملات"),
    ("shalat jamak qasar", ["الجمع","قصر"], "عبادات"),
    ("cara tayammum yang benar", ["تيمم","طهارة"], "طهارة"),
    ("hukum asuransi syariah", ["تأمين","تكافل"], "معاملات"),
    ("syarat sah nikah", ["شروط","نكاح"], "مناكحات"),
    ("hukum donor organ tubuh", ["التبرع","عضو"], None),
    ("boleh gak foto makhluk hidup", ["تصوير","الأرواح"], None),
    ("kloning manusia pandangan islam", ["استنساخ"], None),
]

passed = 0
failed = 0
for q, expected_terms, exp_domain in queries:
    r = requests.post(f"{BASE}/api/query", json={"query": q}, headers=headers)
    d = r.json()
    terms = d.get("translated_terms", [])
    domain = d.get("detected_domain", "")
    results = d.get("results", [])
    n = len(results)
    top_title = results[0].get("title", "N/A")[:60] if results else "N/A"
    top_score = results[0].get("score", 0) if results else 0
    
    term_ok = any(any(e in t for t in terms) for e in expected_terms)
    domain_ok = exp_domain is None or exp_domain in domain
    has_results = n > 0
    
    ok = term_ok and domain_ok and has_results
    if ok:
        status = "PASS"
        passed += 1
    else:
        status = "FAIL"
        failed += 1
    
    issues = []
    if not has_results: issues.append("NO RESULTS")
    if not term_ok: issues.append(f"TERMS missing {expected_terms}")
    if not domain_ok: issues.append(f"DOMAIN {domain} != {exp_domain}")
    
    issue_str = " | ".join(issues) if issues else ""
    print(f'{"V" if ok else "X"} [{n:2d}] [{domain:8s}] "{q}"')
    if not ok:
        print(f'     ISSUES: {issue_str}')
    print(f'     Terms({len(terms)}): {", ".join(terms[:5])}')
    print(f'     Top: [{top_score:.0f}] {top_title}')
    print()

print(f"{'='*60}")
print(f"RESULT: {passed}/{passed+failed} PASSED ({failed} failed)")
print(f"{'='*60}")
