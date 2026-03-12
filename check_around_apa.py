#!/usr/bin/env python3
"""Check queries around index 1640-1660 where 'apa' lives."""
import json
import requests

os.chdir("/opt/kizana/backend")
import os

with open("queries_10k_eval.json", "r", encoding="utf-8") as f:
    all_q = json.load(f)

# Show queries around apa
print("=== Queries 1630-1660 ===")
for i in range(1630, 1660):
    print(f"  [{i}] {all_q[i].get('text','')}")

# Find unusual/problematic single-word queries
print("\n=== All queries <= 6 chars ===")
short = [(i, q.get("text","")) for i, q in enumerate(all_q) if len(q.get("text","").strip()) <= 6]
for i, t in short:
    print(f"  [{i}] '{t}'")
