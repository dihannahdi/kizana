#!/usr/bin/env python3
import json
q = json.load(open("/opt/kizana/backend/queries_10k_eval.json"))
# Find exact "apa" query
for i, x in enumerate(q):
    text = x.get("text", "").strip().lower()
    if text == "apa":
        print(f"Found 'apa' at index {i} (batch {i//50 + 1})")
# Also find very short queries
short = [(i, x.get("text","")) for i, x in enumerate(q) if len(x.get("text","").strip()) <= 5]
print(f"\nTotal short queries (<= 5 chars): {len(short)}")
for i, t in short[:20]:
    print(f"  [{i}] '{t}'")
