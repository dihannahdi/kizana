#!/usr/bin/env python3
"""Get full list of low-result and zero-translated-term queries from V17b."""
import json
import os

os.chdir("/opt/kizana/backend")

with open("eval_results_v17.json", "r", encoding="utf-8") as f:
    v17 = json.load(f)

results = v17.get("results", [])

# All 1-result queries
print("=== ALL 1-RESULT QUERIES (V17b) ===")
low_one = [(r.get("query_text",""), r.get("translated_terms",[])) 
           for r in results if r.get("num_results",0) == 1]
for q, t in low_one:
    print(f"  '{q}'  T:{t[:4]}")

print(f"\nTotal 1-result: {len(low_one)}")

print("\n=== QUERIES WITH ZERO ARABIC TERMS AND >0 RESULTS (V17b) ===")
zero_t_nonzero_r = [(r.get("query_text",""), r.get("num_results",0)) 
                    for r in results 
                    if len(r.get("translated_terms",[])) == 0 and r.get("num_results",0) > 0]
for q, n in zero_t_nonzero_r[:20]:
    print(f"  {n} results: '{q}'")
print(f"\nTotal zero-Arabic-terms with results: {len(zero_t_nonzero_r)}")
