import json

qs = json.load(open('d:/nahdi/bahtsulmasail/backend/queries_10k.json', encoding='utf-8'))

for cat in ['usul_fiqh', 'khilaf', 'mazhab_issue', 'mazhab', 'comparative']:
    samples = [q for q in qs if q['category'] == cat]
    print(f"\n=== {cat} ({len(samples)} queries) ===")
    for q in samples[:8]:
        print(f"  {q['text'][:80]}")
