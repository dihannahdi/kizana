#!/usr/bin/env python3
import requests, json

BASE = "http://localhost:8080"
login = requests.post(f"{BASE}/api/auth/login", json={
    "email": "eval_admin@bahtsulmasail.tech",
    "password": "EvalTest2026"
})
token = login.json().get("token", "")
headers = {"Authorization": f"Bearer {token}"}

r = requests.post(f"{BASE}/api/query", json={"query": "ihram dari miqat"}, headers=headers)
d = r.json()
print("Keys:", list(d.keys()))
print()
for k, v in d.items():
    if isinstance(v, list):
        print(f"{k}: [{len(v)} items]")
        if v and isinstance(v[0], dict):
            print(f"  First item keys: {list(v[0].keys())}")
            print(f"  First item: {json.dumps(v[0], ensure_ascii=False)[:200]}")
    elif isinstance(v, dict):
        print(f"{k}: {json.dumps(v, ensure_ascii=False)[:200]}")
    else:
        print(f"{k}: {v}")
