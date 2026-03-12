import json

with open('eval_results.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

# Detailed look at specific queries to assess RELEVANCE quality
checks = [
    # Core fiqh queries
    'ib01', 'ib05', 'ib07', 'ib08', 'ib13',
    # Thaharah
    'th03', 'th06', 'th09',
    # Puasa 
    'pu01', 'pu05', 'pu08',
    # Nikah
    'nk02', 'nk05', 'nk07', 'nk11',
    # Muamalat
    'mu01', 'mu09', 'mu10',
    # Aqidah
    'aq03', 'aq04', 'aq08',
    # Contemporary
    'kt01', 'kt03', 'kt05', 'kt08', 'kt09',
    # English
    'en01', 'en04',
    # Arabic
    'ar01', 'ar03',
    # Casual/mixed
    'cm01', 'cm02', 'cm03', 'cm05',
    # Zero results
    'hj03', 'ts05',
    # Low translation
    'ib08', 'mu08', 'jn02',
]

for q in data['by_query']:
    if q['id'] in checks:
        print(f"\n{'='*80}")
        print(f"[{q['id']}] {q['text']}")
        print(f"  Lang={q['lang']} | Domain={q['domain']} | Results={q['num_results']} | Time={q['search_time_ms']}ms")
        print(f"  Translated: {q['translated_terms']}")
        for i, r in enumerate(q['top3']):
            print(f"  #{i+1}: score={r['score']} | {r['title']}")
            print(f"       Book: {r['book']} | Page: {r['page']} | Snippet: {r['snippet_len']} chars")
