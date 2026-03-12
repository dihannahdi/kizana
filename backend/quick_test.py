#!/usr/bin/env python3
import requests, json

BASE = "http://localhost:8080"

# Login first
login = requests.post(f"{BASE}/api/auth/login", json={
    "email": "eval_admin@bahtsulmasail.tech",
    "password": "EvalTest2026"
})
token = login.json().get("token", "")
headers = {"Authorization": f"Bearer {token}"}
print(f"Auth: {'OK' if token else 'FAILED'}")

queries = [
    "ihram dari miqat",
    "adab murid kepada guru",
    "operasi plastik hukumnya",
    "imam shalat perempuan boleh gak",
    "zakat emas dan perak nisabnya",
    "iman kepada qadha qadar",
    "sifat dua puluh allah",
    "boleh gak shalat pakai celana pendek",
]

for q in queries:
    r = requests.post(f"{BASE}/api/query", json={"query": q}, headers=headers)
    d = r.json()
    terms = d.get("translated_terms", [])
    domain = d.get("domain", "")
    n = d.get("total_results", 0)
    top = d.get("results", [{}])[0].get("title", "N/A")[:50] if d.get("results") else "N/A"
    print(f'[{n:2d}] [{domain:8s}] "{q}"')
    print(f'     Terms({len(terms)}): {", ".join(terms[:5])}')
    print(f'     Top: {top}')
    print()
