#!/usr/bin/env python3
"""
Comprehensive quality scan of ALL queries 3000-10030.
Reports zeros and very-low (1-2 results).
"""
import json, requests, os, sys

BASE_URL = "https://bahtsulmasail.tech"
resp = requests.post(f"{BASE_URL}/api/auth/login",
    json={"email": "eval_admin@bahtsulmasail.tech", "password": "EvalTest2026"}, timeout=30)
token = resp.json()["token"]
headers = {"Authorization": f"Bearer {token}"}

os.chdir("/opt/kizana/backend")
with open("queries_10k_eval.json", "r", encoding="utf-8") as f:
    all_q = json.load(f)

# Test ALL queries from 3000 to 10030
start_idx = int(sys.argv[1]) if len(sys.argv) > 1 else 3000
end_idx = int(sys.argv[2]) if len(sys.argv) > 2 else 10030

queries = [{"id": str(i), "text": all_q[i].get("text", "")} for i in range(start_idx, end_idx)]
print(f"Testing {len(queries)} queries ({start_idx}-{end_idx})...")

zeros = []
low_results = []  # 1-4 results
BATCH_SIZE = 50

for i in range(0, len(queries), BATCH_SIZE):
    batch = queries[i:i+BATCH_SIZE]
    try:
        resp = requests.post(f"{BASE_URL}/api/eval/batch", headers=headers,
            json={"queries": batch, "config": {"use_ai": False}, "max_results": 10},
            timeout=120)
        for r in resp.json().get("results", []):
            q = r.get("query_text", "?")
            count = r.get("num_results", -1)
            terms = r.get("translated_terms", [])
            idx = r.get("query_id", "?")
            if count == 0:
                zeros.append((idx, q, terms))
            elif 0 < count < 5:
                low_results.append((idx, q, count, terms))
    except Exception as e:
        print(f"Error at batch {i}: {e}", file=sys.stderr)
    
    if (i // BATCH_SIZE) % 10 == 0:
        print(f"  Progress: {i+BATCH_SIZE}/{len(queries)}, zeros={len(zeros)}, low={len(low_results)}")
        sys.stdout.flush()

print(f"\n=== FINAL RESULTS ({start_idx}-{end_idx}) ===")
print(f"Zeros ({len(zeros)}):")
for idx, q, t in zeros:
    print(f"  [{idx}] '{q}' T:{t[:3]}")
print(f"\nLow 1-4 results ({len(low_results)}):")
for idx, q, n, t in sorted(low_results, key=lambda x: x[2]):
    print(f"  {n:2d} | [{idx}] '{q}' T:{t[:3]}")
