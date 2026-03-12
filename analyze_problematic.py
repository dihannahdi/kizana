#!/usr/bin/env python3
"""
Proactive analysis: find queries likely to produce 0 results.
Looks for queries where ALL words seem unmapped or problematic.
"""
import json
import os
import requests

os.chdir("/opt/kizana/backend")

with open("queries_10k_eval.json", "r", encoding="utf-8") as f:
    all_q = json.load(f)

print(f"Total queries: {len(all_q)}")

# Find short queries (likely stopword-only or single-concept)
short = [(i, q.get("text","").strip()) for i, q in enumerate(all_q) 
         if 1 <= len(q.get("text","").strip()) <= 10]
print(f"\n=== Queries <= 10 chars ({len(short)}) ===")
for i, t in sorted(short, key=lambda x: len(x[1])):
    print(f"  [{i:4d}] '{t}'")

# Find queries with unusual characters
odd = [(i, q.get("text","").strip()) for i, q in enumerate(all_q) 
       if any(c in q.get("text","") for c in ["@", "#", "$", "%", "^", "&", "*"])]
print(f"\n=== Queries with special chars ({len(odd)}) ===")
for i, t in odd[:20]:
    print(f"  [{i:4d}] '{t}'")

# Find queries that seem to be numeric only
numeric = [(i, q.get("text","").strip()) for i, q in enumerate(all_q)
           if q.get("text","").strip().isdigit()]
print(f"\n=== Numeric-only queries ({len(numeric)}) ===")
for i, t in numeric:
    print(f"  [{i:4d}] '{t}'")

# Find queries that are purely transliterated qawa'id (likely difficult)
qawaid_patterns = ["al-", "al ", "fi al", "ala al", "min al"]
hard = [(i, q.get("text","").strip()) for i, q in enumerate(all_q)
        if any(p in q.get("text","").lower() for p in qawaid_patterns)]
print(f"\n=== Qawa'id-style queries ({len(hard)}) ===")
for i, t in hard[:30]:
    print(f"  [{i:4d}] '{t}'")
