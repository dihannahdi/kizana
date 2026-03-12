import json

qs = json.load(open('d:/nahdi/bahtsulmasail/backend/queries_10k.json', encoding='utf-8'))

for cat in ['edge', 'misspell', 'kontemporer', 'tech', 'en_natural']:
    print(f"\n=== {cat} (sample 5) ===")
    samples = [q for q in qs if q['category'] == cat][:5]
    for q in samples:
        print(f"  {q['id']:15s} {q['text'][:70]}")
