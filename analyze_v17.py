#!/usr/bin/env python3
"""Analyze V17b eval results for insights beyond just zeros."""
import json
import os

os.chdir("/opt/kizana/backend")

with open("eval_results_v17.json", "r", encoding="utf-8") as f:
    v17 = json.load(f)

results = v17.get("results", [])
print(f"Total: {len(results)} queries")
print()

# Distribution of result counts
counts = [r.get("num_results", 0) for r in results]
from collections import Counter
dist = Counter(counts)
print("Result count distribution:")
for k in sorted(dist.keys()):
    pct = 100 * dist[k] / len(results)
    bar = "#" * int(pct / 2)
    print(f"  {k:2} results: {dist[k]:4} ({pct:5.1f}%) {bar}")

print()

# Low result queries (1-2 results) 
low = [r for r in results if r.get("num_results", 0) in (1, 2)]
print(f"Low-result queries (1-2 results): {len(low)}")
for r in low[:15]:
    qt = r.get("query_text", "?")
    n = r.get("num_results", 0)
    tt = r.get("translated_terms", [])[:3]
    print(f"  {n} result(s): '{qt}' T:{tt}")

print()
# Translation stats
zero_t = [r for r in results if len(r.get("translated_terms", [])) == 0]
low_t = [r for r in results if 0 < len(r.get("translated_terms", [])) <= 2]
good_t = [r for r in results if len(r.get("translated_terms", [])) > 2]
print(f"Translation stats:")
print(f"  Zero Arabic terms: {len(zero_t)} ({100*len(zero_t)/len(results):.1f}%)")
print(f"  1-2 Arabic terms: {len(low_t)} ({100*len(low_t)/len(results):.1f}%)")
print(f"  3+ Arabic terms: {len(good_t)} ({100*len(good_t)/len(results):.1f}%)")
