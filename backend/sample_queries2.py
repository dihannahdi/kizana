import json

qs = json.load(open('d:/nahdi/bahtsulmasail/backend/queries_10k.json', encoding='utf-8'))

# Sample from multiple interesting categories
for cat in ['kontemporer', 'awam', 'skenario', 'conditional', 'food', 'medical', 'environment', 'women_fiqh', 'nikah_deep', 'shalat_deep', 'usul_fiqh']:
    samples = [q for q in qs if q['category'] == cat][:5]
    print(f"\n=== {cat} ({len([q for q in qs if q['category']==cat])} queries, sample 5) ===")
    for q in samples:
        print(f"  {q['text'][:80]}")
