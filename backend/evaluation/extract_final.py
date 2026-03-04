#!/usr/bin/env python3
"""Final: extract ALL metrics for paper."""
import json

# === INTERNAL BASELINES ===
print('=== INTERNAL BASELINES ===')
with open('/opt/kizana/evaluation/results/baselines_20260303_163020.json') as f:
    bas = json.load(f)
for name, data in bas['baselines'].items():
    m = data['metrics']
    print(f"{name} ({data['label']}): MAP={m['MAP']:.3f} MRR={m['MRR']:.3f} NDCG@5={m['NDCG@5']:.3f} NDCG@10={m['NDCG@10']:.3f} P@5={m['P@5']:.3f} P@10={m['P@10']:.3f}")

# === EXTERNAL BASELINE ===
print('\n=== EXTERNAL BASELINE ===')
with open('/opt/kizana/evaluation/results/external_baseline_20260303_170824.json') as f:
    ext = json.load(f)
for name, data in ext.items():
    if not isinstance(data, dict) or 'metrics' not in data:
        continue
    m = data['metrics']
    print(f"{name} ({data['label']}): MAP={m['MAP']:.3f} MRR={m['MRR']:.3f} NDCG@5={m['NDCG@5']:.3f} NDCG@10={m['NDCG@10']:.3f} P@5={m['P@5']:.3f} P@10={m['P@10']:.3f}")
    pl = data.get('per_language', {})
    for lang, lm in sorted(pl.items()):
        print(f"    {lang}: MAP={lm['MAP']:.3f} MRR={lm['MRR']:.3f} NDCG@5={lm['NDCG@5']:.3f}")

# Statistical tests
if 'statistical_tests' in ext:
    print('\n  Statistical Tests:')
    for tname, td in ext['statistical_tests'].items():
        if isinstance(td, dict):
            parts = []
            for k, v in td.items():
                if isinstance(v, float):
                    parts.append(f"{k}={v:.4f}")
                else:
                    parts.append(f"{k}={v}")
            print(f"    {tname}: {', '.join(parts)}")

# === INTERNAL BASELINES PER LANGUAGE ===
print('\n=== INTERNAL BASELINES PER LANGUAGE ===')
for name, data in bas['baselines'].items():
    pl = data.get('per_language', {})
    if pl:
        print(f"{name}:")
        for lang, lm in sorted(pl.items()):
            print(f"    {lang}: MAP={lm['MAP']:.3f} NDCG@5={lm['NDCG@5']:.3f}")

# === STAT TESTS FROM BASELINES ===
if 'statistical_tests' in bas:
    print('\n=== STATISTICAL TESTS (BASELINES) ===')
    st = bas['statistical_tests']
    for tname, td in st.items():
        if isinstance(td, dict):
            parts = [f"{k}={v:.4f}" if isinstance(v, float) else f"{k}={v}" for k, v in td.items()]
            print(f"  {tname}: {', '.join(parts)}")
