#!/usr/bin/env python3
"""Extract all evaluation results for paper writing."""
import json, glob, os

os.chdir('/opt/kizana/evaluation/results')

# 1. Default eval
print('=== DEFAULT EVALUATION ===')
with open('eval_default_20260303_160911.json') as f:
    default = json.load(f)
agg = default['aggregate_metrics']
print(f"MAP={agg['MAP']:.3f} MRR={agg['MRR']:.3f} NDCG@5={agg['NDCG@5']:.3f} NDCG@10={agg['NDCG@10']:.3f}")
print(f"P@5={agg.get('P@5',0):.3f} P@10={agg.get('P@10',0):.3f} R@5={agg.get('R@5',0):.3f} R@10={agg.get('R@10',0):.3f}")
print('Per language:')
for lang, m in sorted(default.get('per_language_metrics', {}).items()):
    print(f"  {lang}: MAP={m['MAP']:.3f} MRR={m['MRR']:.3f} NDCG@5={m['NDCG@5']:.3f} NDCG@10={m['NDCG@10']:.3f}")
print('Per domain:')
for dom, m in sorted(default.get('per_domain_metrics', {}).items()):
    print(f"  {dom}: MAP={m['MAP']:.3f} MRR={m['MRR']:.3f} NDCG@5={m['NDCG@5']:.3f}")

# 2. Ablation 
print('\n=== ABLATION STUDY ===')
with open('ablation_20260303_162215.json') as f:
    abl = json.load(f)
for cfg_name, data in abl.items():
    a = data.get('aggregate_metrics', data.get('aggregate', data.get('metrics', {})))
    if not a:
        # Try the data itself as metrics
        if 'MAP' in data:
            a = data
        else:
            print(f"{cfg_name}: keys={list(data.keys())}")
            continue
    print(f"{cfg_name}: MAP={a['MAP']:.3f} MRR={a['MRR']:.3f} NDCG@5={a['NDCG@5']:.3f} NDCG@10={a['NDCG@10']:.3f}")

# 3. Baselines  
print('\n=== INTERNAL BASELINES ===')
with open('baselines_20260303_163020.json') as f:
    bas = json.load(f)
for cfg_name, data in bas.items():
    if not isinstance(data, dict):
        continue
    a = data.get('aggregate_metrics', data.get('aggregate', data.get('metrics', {})))
    if not a:
        if 'MAP' in data:
            a = data
        else:
            print(f"{cfg_name}: keys={list(data.keys())}")
            continue
    print(f"{cfg_name}: MAP={a['MAP']:.3f} MRR={a['MRR']:.3f} NDCG@5={a['NDCG@5']:.3f} NDCG@10={a['NDCG@10']:.3f}")

# 4. External baseline
print('\n=== EXTERNAL BASELINE (vs Google Translate) ===')
with open('external_baseline_20260303_170824.json') as f:
    ext = json.load(f)
for cfg_name, data in ext.items():
    if cfg_name == 'statistical_tests':
        continue
    if not isinstance(data, dict):
        continue
    a = data.get('aggregate_metrics', data.get('aggregate', {}))
    if not a or 'MAP' not in a:
        continue
    print(f"{cfg_name}: MAP={a['MAP']:.3f} MRR={a['MRR']:.3f} NDCG@5={a.get('NDCG@5',0):.3f} NDCG@10={a.get('NDCG@10',0):.3f}")
    pl = data.get('per_language_metrics', data.get('per_language', {}))
    if pl:
        for lang, m in sorted(pl.items()):
            print(f"    {lang}: MAP={m['MAP']:.3f} MRR={m['MRR']:.3f} NDCG@5={m.get('NDCG@5',0):.3f}")

if 'statistical_tests' in ext:
    print('\nStatistical tests:')
    for test_name, td in ext['statistical_tests'].items():
        print(f"  {test_name}: {json.dumps(td, indent=2)}")

# 5. Optimized configs
print('\n=== OPTIMIZED CONFIGS ===')
with open('optimized_20260303_171253.json') as f:
    opt = json.load(f)
a = opt['aggregate']
print(f"no_multi_variant: MAP={a['MAP']:.3f} MRR={a['MRR']:.3f} NDCG@5={a['NDCG@5']:.3f} NDCG@10={a['NDCG@10']:.3f}")
for lang, m in sorted(opt.get('per_language', {}).items()):
    print(f"    {lang}: MAP={m['MAP']:.3f} NDCG@5={m['NDCG@5']:.3f}")

print()
with open('combined_opt_20260303_171608.json') as f:
    copt = json.load(f)
for cfg_name, data in copt.items():
    a = data['aggregate']
    print(f"{cfg_name}: MAP={a['MAP']:.3f} MRR={a['MRR']:.3f} NDCG@5={a['NDCG@5']:.3f} NDCG@10={a['NDCG@10']:.3f}")
    for lang, m in sorted(data.get('per_language', {}).items()):
        print(f"    {lang}: MAP={m['MAP']:.3f} NDCG@5={m['NDCG@5']:.3f}")

# 6. Gold standard validation
print('\n=== GOLD STANDARD VALIDATION ===')
with open('gold_standard_validation_20260303_170204.json') as f:
    gs = json.load(f)
for k, v in gs.items():
    print(f"  {k}: {v}")

# 7. Annotation protocol
print('\n=== ANNOTATION PROTOCOL (first 50 lines) ===')
with open('annotation_protocol.txt') as f:
    for i, line in enumerate(f):
        if i >= 50:
            break
        print(line.rstrip())

# 8. Gold standard stats
print('\n=== GOLD STANDARD DATASET STATS ===')
import sys
sys.path.insert(0, '/opt/kizana/evaluation')
from evaluate import load_gold_standard
gold = load_gold_standard()
print(f"Total queries: {len(gold)}")
langs = {}
doms = {}
for q in gold:
    l = q.get('language', 'unknown')
    d = q.get('domain', 'unknown')
    langs[l] = langs.get(l, 0) + 1
    doms[d] = doms.get(d, 0) + 1
print(f"Languages: {json.dumps(langs)}")
print(f"Domains: {json.dumps(doms)}")
avg_kw = sum(len(q.get('relevant_keywords', [])) for q in gold) / len(gold)
print(f"Avg keywords per query: {avg_kw:.1f}")
