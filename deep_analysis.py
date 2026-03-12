#!/usr/bin/env python3
"""Full analysis of all low-result queries with tokenization simulation."""
import json
import os

os.chdir("/opt/kizana/backend")

with open("eval_results_v17.json", "r", encoding="utf-8") as f:
    v17 = json.load(f)

results = v17.get("results", [])

# All queries with <= 5 results
print("=== ALL QUERIES WITH <= 5 RESULTS (V17b) ===")
low = [(r.get("num_results",0), r.get("query_text",""), r.get("translated_terms",[]))
       for r in results if r.get("num_results",0) <= 5]
low.sort()
for n, q, t in low:
    print(f"  {n:2} | '{q}'")
    if t:
        print(f"       T:{t[:4]}")

print(f"\nTotal: {len(low)}")

# Summary by pattern
print("\n=== PATTERN ANALYSIS ===")
patterns = {}
for n, q, t in low:
    # Detect pattern
    ql = q.lower()
    if "pandangan islam tentang" in ql:
        p = "pandangan tentang X"
    elif "what does islam say" in ql or "what islam says" in ql:
        p = "what Islam says about X"
    elif "dalam islam" in ql:
        p = "X dalam islam"
    elif ql.startswith("al ") or ql.startswith("al-"):
        p = "transliterated Arabic maxim"
    else:
        p = "other"
    patterns.setdefault(p, []).append(q)

for p, qs in sorted(patterns.items()):
    print(f"\n  {p} ({len(qs)} queries):")
    for q in qs:
        print(f"    - {q}")
