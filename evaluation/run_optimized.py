#!/usr/bin/env python3
"""Quick run: optimized config (no multi-variant)."""
from evaluate import run_evaluation, load_gold_standard
import json, time

gold = load_gold_standard()
token = open('/tmp/admin_token.txt').read().strip()

print('Running optimized (no multi-variant)...')
r = run_evaluation(base_url='http://127.0.0.1:8080', token=token, 
                   config={'disable_multi_variant': True}, gold_queries=gold, max_results=20)
agg = r['aggregate_metrics']
print(f"MAP={agg['MAP']:.3f} MRR={agg['MRR']:.3f} NDCG@5={agg['NDCG@5']:.3f} NDCG@10={agg['NDCG@10']:.3f} P@5={agg['P@5']:.3f} P@10={agg['P@10']:.3f}")

# Per-language
for lang, metrics in sorted(r.get('per_language_metrics', {}).items()):
    print(f"  {lang}: MAP={metrics['MAP']:.3f} NDCG@5={metrics['NDCG@5']:.3f}")

ts = time.strftime('%Y%m%d_%H%M%S')
with open(f'results/optimized_{ts}.json', 'w') as f:
    json.dump({'aggregate': agg, 'per_language': r.get('per_language_metrics', {}), 
               'per_domain': r.get('per_domain_metrics', {})}, f, indent=2)
print(f'Saved to results/optimized_{ts}.json')
