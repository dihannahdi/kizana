#!/usr/bin/env python3
import json
q = json.load(open('/opt/kizana/backend/queries_10k_eval.json'))
for i in range(9840, 9900):
    print(f"[{i}] {q[i]['text']}")
print("...")
for i in range(9980, 10030):
    print(f"[{i}] {q[i]['text']}")
