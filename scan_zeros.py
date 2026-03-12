#!/usr/bin/env python3
"""
Proactively scan all 10030 eval queries and simulate translation
to find any that would produce zero Arabic terms — before eval completes.
"""
import json
import re
import sys
import os

os.chdir("/opt/kizana/backend")

# Load all queries
with open("queries_10k_eval.json", "r", encoding="utf-8") as f:
    all_queries = json.load(f)

print(f"Total queries: {len(all_queries)}")

# Minimal simulation: check if query has any mapped term
# We use the service log to check translated_terms for a few hundred queries
# OR we parse the v17b results to find T:[] patterns

# Load v17b results to see what was flagged
try:
    with open("eval_results_v17.json", "r", encoding="utf-8") as f:
        v17 = json.load(f)
    
    v17_results = v17.get("results", [])
    zeros_v17 = [(r.get("query_text",""), r.get("translated_terms",[])) 
                 for r in v17_results 
                 if r.get("num_results", 1) == 0]
    
    print(f"\nAll V17b zeros ({len(zeros_v17)} total):")
    for q, t in zeros_v17:
        print(f"  '{q}' T:{t}")
        
except Exception as e:
    print(f"Could not load v17 results: {e}")

# Also check if progress file has any zeros
try:
    with open("eval_results_v19_progress.json", "r", encoding="utf-8") as f:
        v19p = json.load(f)
    
    v19_results = v19p.get("results", [])
    zeros_v19 = [(r.get("query_text",""), r.get("translated_terms",[])) 
                 for r in v19_results 
                 if r.get("num_results", 1) == 0]
    
    print(f"\nV19 zeros so far ({len(v19_results)} processed): {len(zeros_v19)}")
    for q, t in zeros_v19:
        print(f"  '{q}' T:{t}")
        
except Exception as e:
    print(f"V19 progress: {e}")
