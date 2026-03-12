import json

with open('d:/nahdi/bahtsulmasail/backend/eval_progress.json', encoding='utf-8') as f:
    d = json.load(f)

batches_done = d.get('completed_batches', 0)
results_count = len(d.get('results', []))
total_batches = 201
total_queries = 10030
pct = 100 * batches_done / total_batches
queries_done = batches_done * 50
remaining = total_queries - queries_done
print(f"Batches: {batches_done}/{total_batches} ({pct:.1f}%)")
print(f"Queries: {queries_done}/{total_queries}")
print(f"Results collected: {results_count}")
print(f"Remaining: {remaining} queries")
