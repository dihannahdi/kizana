import json

with open('eval_results.json', 'r', encoding='utf-8') as f:
    data = json.load(f)

# 1. Queries with NO results
print('=== QUERIES WITH NO RESULTS ===')
for q in data['by_query']:
    if q['num_results'] == 0:
        print(f"  [{q['id']}] {q['text']} (lang={q['lang']}, domain={q['domain']}, terms={q['translated_terms']})")

print()
print('=== QUERIES WITH LOW TOP SCORE (<80) ===')
for q in data['by_query']:
    if q['top_score'] < 80 and q['num_results'] > 0:
        print(f"  [{q['id']}] {q['text']} (score={q['top_score']}, results={q['num_results']}, terms={q['translated_terms'][:5]})")
        for r in q['top3'][:2]:
            print(f"    -> {r['title'][:60]} | {r['book'][:40]} | score={r['score']}")

print()
print('=== QUERIES WITH FEW RESULTS (<5) ===')
for q in data['by_query']:
    if 0 < q['num_results'] < 5:
        print(f"  [{q['id']}] {q['text']} (results={q['num_results']}, score={q['top_score']})")

print()
print('=== SLOW QUERIES (>1000ms) ===')
for q in data['by_query']:
    if q['search_time_ms'] > 1000:
        print(f"  [{q['id']}] {q['text']} ({q['search_time_ms']}ms)")

print()
print('=== LOW DIVERSITY (unique_books<=3) ===')
for q in data['by_query']:
    if q['unique_books'] <= 3 and q['num_results'] >= 5:
        print(f"  [{q['id']}] {q['text']} (books={q['unique_books']}, results={q['num_results']})")

print()
print('=== DOMAIN DETECTION ISSUES ===')
# Queries that should be munakahat but detected as something else, etc.
expected_domains = {
    'nk': 'مناكحات', 'mu': 'معاملات', 'ib': 'عبادات', 'th': 'طهارة',
    'pu': 'عبادات', 'zk': 'عبادات', 'hj': 'عبادات', 'aq': 'عقيدة',
    'ts': 'تصوف', 'jn': 'جنايات', 'mk': 'عبادات', 'kt': 'عام',
    'en': 'عام', 'ar': 'عام', 'cm': 'عام'
}
for q in data['by_query']:
    prefix = q['id'][:2]
    expected = expected_domains.get(prefix, 'عام')
    if q['domain'] != expected and q['domain'] != 'عام' and expected != 'عام':
        print(f"  [{q['id']}] {q['text']}: detected={q['domain']}, expected={expected}")

print()
print('=== TRANSLATION COVERAGE GAPS ===')
for q in data['by_query']:
    if len(q['translated_terms']) <= 2:
        print(f"  [{q['id']}] {q['text']} -> only {len(q['translated_terms'])} terms: {q['translated_terms']}")

print()
print('=== SUMMARY STATISTICS ===')
scores = [q['top_score'] for q in data['by_query'] if q['num_results'] > 0]
times = [q['search_time_ms'] for q in data['by_query']]
results_counts = [q['num_results'] for q in data['by_query']]
books_counts = [q['unique_books'] for q in data['by_query'] if q['num_results'] > 0]

print(f"  Total queries: {data['total_queries']}")
print(f"  With results: {data['queries_with_results']} ({data['queries_with_results']/data['total_queries']*100:.1f}%)")
print(f"  No results: {data['queries_no_results']}")
print(f"  Avg results/query: {data['avg_results_per_query']}")
print(f"  Avg top score: {sum(scores)/len(scores):.1f}")
print(f"  Median top score: {sorted(scores)[len(scores)//2]:.1f}")
print(f"  Min top score: {min(scores):.1f}")
print(f"  Avg search time: {sum(times)/len(times):.0f}ms")
print(f"  Max search time: {max(times)}ms")
print(f"  Avg unique books/query: {sum(books_counts)/len(books_counts):.1f}")
print(f"  Total unique books: {data['diversity']['unique_books_total']}")
print(f"  Score distribution: {data['score_distribution']}")
print(f"  Domain distribution: {data['domain_distribution']}")
print(f"  Language detection: {data['language_distribution']}")
