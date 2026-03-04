#!/usr/bin/env python3
"""Extract baselines and external baseline data."""
import json

# Baselines
with open('/opt/kizana/evaluation/results/baselines_20260303_163020.json') as f:
    bas = json.load(f)

print('=== INTERNAL BASELINES ===')
bl = bas.get('baselines', bas)
for k, v in bl.items():
    if not isinstance(v, dict):
        continue
    a = v.get('aggregate_metrics', v.get('aggregate', {}))
    if not a or 'MAP' not in a:
        continue
    print(f"{k}: MAP={a['MAP']:.3f} MRR={a['MRR']:.3f} NDCG@5={a['NDCG@5']:.3f} NDCG@10={a['NDCG@10']:.3f}")

# Statistical tests from baselines
if 'statistical_tests' in bas:
    print('\nStatistical tests (baselines):')
    for t, d in bas['statistical_tests'].items():
        if isinstance(d, dict):
            print(f"  {t}: t={d.get('t_stat',0):.3f} p={d.get('p_value',0):.4f} d={d.get('effect_size',0):.3f}")
        else:
            print(f"  {t}: {d}")

# External baseline
print('\n=== EXTERNAL BASELINE ===')
with open('/opt/kizana/evaluation/results/external_baseline_20260303_170824.json') as f:
    ext = json.load(f)

for k, v in ext.items():
    if k == 'statistical_tests':
        continue
    if not isinstance(v, dict):
        continue
    a = v.get('aggregate_metrics', v.get('aggregate', {}))
    if not a or 'MAP' not in a:
        continue
    print(f"{k}: MAP={a['MAP']:.3f} MRR={a['MRR']:.3f} NDCG@5={a.get('NDCG@5',0):.3f} NDCG@10={a.get('NDCG@10',0):.3f}")
    pl = v.get('per_language_metrics', v.get('per_language', {}))
    if pl:
        for lang, m in sorted(pl.items()):
            print(f"    {lang}: MAP={m['MAP']:.3f} NDCG@5={m.get('NDCG@5',0):.3f}")

if 'statistical_tests' in ext:
    print('\nStatistical tests (external):')
    for t, d in ext['statistical_tests'].items():
        if isinstance(d, dict):
            print(f"  {t}:")
            for kk, vv in d.items():
                print(f"    {kk}: {vv}")
        else:
            print(f"  {t}: {d}")
