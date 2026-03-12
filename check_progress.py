#!/usr/bin/env python3
import json, os, sys

for fname in ["eval_results_v20_progress.json", "eval_results_v19_progress.json"]:
    f = "/opt/kizana/backend/" + fname
    if os.path.exists(f):
        break
else:
    print("No progress file yet")
    sys.exit()

d = json.load(open(f))
results = d.get("results", [])
zeros = [r for r in results if r.get("num_results", 1) == 0]
print(f"Processed: {len(results)}, zeros: {len(zeros)} ({100*len(zeros)/max(len(results),1):.2f}%)")
for z in zeros[:20]:
    qt = z.get("query_text", "?")
    tt = z.get("translated_terms", [])
    print(f"  ZERO: '{qt}' T:{tt}")
