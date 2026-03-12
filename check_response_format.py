#!/usr/bin/env python3
"""Check actual eval batch response structure."""
import json
import requests

BASE_URL = "https://bahtsulmasail.tech"
resp = requests.post(f"{BASE_URL}/api/auth/login", 
    json={"email": "eval_admin@bahtsulmasail.tech", "password": "EvalTest2026"}, timeout=30)
token = resp.json()["token"]
headers = {"Authorization": f"Bearer {token}"}

# Test 3 queries to see full response structure
resp = requests.post(f"{BASE_URL}/api/eval/batch", headers=headers,
    json={"queries": [
        {"id": "A", "text": "cara"},
        {"id": "B", "text": "hukum saf"},
        {"id": "C", "text": "wuhu"},
    ], "config": {"use_ai": False}, "max_results": 10},
    timeout=60)
data = resp.json()
results = data.get("results", [])
for r in results:
    print(f"\n--- Query: {r.get('query_text','?')} ---")
    print(f"  Keys: {sorted(r.keys())}")
    # Print all non-results fields
    for k, v in sorted(r.items()):
        if k != "results":
            print(f"  {k}: {v}")
    # Count results
    inner = r.get("results", [])
    print(f"  Inner results count: {len(inner)}")
    if inner:
        print(f"  First result keys: {sorted(inner[0].keys())}")
        print(f"  First result: {json.dumps(inner[0])[:200]}")
