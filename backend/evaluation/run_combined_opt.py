#!/usr/bin/env python3
"""Test combined optimization configs."""
from evaluate import run_evaluation, load_gold_standard
import json, time

gold = load_gold_standard()
token = open('/tmp/admin_token.txt').read().strip()

configs = {
    'no_multivar_no_diversity': {
        'disable_multi_variant': True,
        'disable_diversity_cap': True,
    },
    'no_multivar_no_bookpenalty': {
        'disable_multi_variant': True,
        'disable_book_penalty': True,
    },
    'kitchen_sink': {
        'disable_multi_variant': True,
        'disable_diversity_cap': True,
        'disable_book_penalty': True,
    },
}

results = {}
for name, cfg in configs.items():
    print(f'Running {name}...')
    r = run_evaluation(
        base_url='http://127.0.0.1:8080',
        token=token,
        config=cfg,
        gold_queries=gold,
        max_results=20,
    )
    agg = r['aggregate_metrics']
    results[name] = {
        'aggregate': agg,
        'per_language': r.get('per_language_metrics', {}),
    }
    print(f"  MAP={agg['MAP']:.3f} MRR={agg['MRR']:.3f} "
          f"NDCG@5={agg['NDCG@5']:.3f} NDCG@10={agg['NDCG@10']:.3f}")

ts = time.strftime('%Y%m%d_%H%M%S')
with open(f'results/combined_opt_{ts}.json', 'w') as f:
    json.dump(results, f, indent=2)
print(f'All done. Saved to results/combined_opt_{ts}.json')
