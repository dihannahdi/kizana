#!/usr/bin/env python3
"""Deep audit: domain accuracy, per-category analysis, relevance quality."""
import json

d = json.load(open('eval_results_v14.json', 'r', encoding='utf-8'))

# Expected domain mapping for each query category
EXPECTED_DOMAINS = {
    'ib': 'عبادات',   # Ibadah/Shalat
    'th': 'طهارة',    # Thaharah
    'pu': 'عبادات',   # Puasa
    'zk': 'عبادات',   # Zakat
    'hj': 'عبادات',   # Haji
    'nk': 'مناكحات',  # Munakahat
    'mu': 'معاملات',  # Muamalat
    'aq': 'عقيدة',    # Aqidah
    'ts': None,       # Tasawuf/Akhlak - mixed
    'mk': None,       # Makanan - mixed domains
    'jn': 'جنايات',   # Jinayat
    'kt': None,       # Kontemporer - mixed
    'en': None,       # English - mixed
    'ar': None,       # Arabic - mixed
    'cm': None,       # Campuran - mixed
}

print('=' * 70)
print('DEEP AUDIT v13 — COMPREHENSIVE ANALYSIS')
print('=' * 70)

# ─── 1. OVERALL METRICS ─────────────────────────────────────
print('\n1. OVERALL METRICS')
print(f'   Total queries: {d["total_queries"]}')
print(f'   Queries with results: {d["queries_with_results"]} ({d["queries_with_results"]/d["total_queries"]*100:.0f}%)')
print(f'   Zero results: {d["queries_no_results"]}')
print(f'   Avg search time: {d["avg_search_time_ms"]:.0f}ms')
print(f'   Translation coverage: {d["translation_coverage"]["translated"]}/{d["total_queries"]} ({d["translation_coverage"]["translated"]/d["total_queries"]*100:.0f}%)')

# ─── 2. SCORE DISTRIBUTION ──────────────────────────────────
print('\n2. SCORE DISTRIBUTION (all results)')
sd = d['score_distribution']
total_results = sum(sd.values())
for band, count in sd.items():
    pct = count / total_results * 100 if total_results else 0
    bar = '#' * int(pct / 2)
    print(f'   {band:8s}: {count:4d} ({pct:5.1f}%) {bar}')

# ─── 3. PER-CATEGORY ANALYSIS ───────────────────────────────
print('\n3. PER-CATEGORY ANALYSIS')
categories = {}
for q in d['by_query']:
    cat = q['id'][:2]
    if cat not in categories:
        categories[cat] = {'queries': [], 'results_count': 0, 'terms_count': 0, 'domains': {}}
    categories[cat]['queries'].append(q)
    categories[cat]['results_count'] += q.get('num_results', 0)
    categories[cat]['terms_count'] += len(q.get('translated_terms', []))
    dom = q.get('domain', 'unknown')
    categories[cat]['domains'][dom] = categories[cat]['domains'].get(dom, 0) + 1

cat_names = {
    'ib': 'Ibadah/Shalat', 'th': 'Thaharah', 'pu': 'Puasa', 'zk': 'Zakat',
    'hj': 'Haji/Umrah', 'nk': 'Munakahat', 'mu': 'Muamalat', 'aq': 'Aqidah',
    'ts': 'Tasawuf/Akhlak', 'mk': 'Makanan', 'jn': 'Jinayat', 'kt': 'Kontemporer',
    'en': 'English', 'ar': 'Arabic', 'cm': 'Campuran'
}

for cat, info in sorted(categories.items()):
    n = len(info['queries'])
    avg_results = info['results_count'] / n
    avg_terms = info['terms_count'] / n
    domains_str = ', '.join(f'{d}:{c}' for d, c in sorted(info['domains'].items(), key=lambda x: -x[1]))
    exp = EXPECTED_DOMAINS.get(cat)
    domain_ok = exp is None or all(q.get('domain') == exp for q in info['queries'])
    marker = 'V' if domain_ok else '!'
    name = cat_names.get(cat, cat)
    print(f'   {marker} [{cat}] {name:15s} | {n:2d} queries | avg_results: {avg_results:.0f} | avg_terms: {avg_terms:.1f} | domains: {domains_str}')

# ─── 4. DOMAIN ACCURACY ─────────────────────────────────────
print('\n4. DOMAIN DETECTION ACCURACY')
correct = 0
wrong = 0
skipped = 0
wrong_list = []

for q in d['by_query']:
    cat = q['id'][:2]
    exp = EXPECTED_DOMAINS.get(cat)
    actual = q.get('domain', '')
    if exp is None:
        skipped += 1
    elif actual == exp:
        correct += 1
    else:
        wrong += 1
        wrong_list.append((q['id'], q['text'][:40], actual, exp))

total_checkable = correct + wrong
acc = correct / total_checkable * 100 if total_checkable else 0
print(f'   Checkable: {total_checkable} queries')
print(f'   Correct: {correct} ({acc:.1f}%)')
print(f'   Wrong: {wrong}')
print(f'   Skipped (mixed domain): {skipped}')

if wrong_list:
    print(f'\n   Wrong detections:')
    for qid, text, actual, expected in wrong_list:
        print(f'   X [{qid}] "{text}" -> {actual} (expected: {expected})')

# ─── 5. SEARCH TIME ANALYSIS ────────────────────────────────
print('\n5. SEARCH TIME ANALYSIS')
times = [q.get('search_time_ms', 0) for q in d['by_query']]
avg_t = sum(times) / len(times)
max_t = max(times)
min_t = min(times)
slow = [(q['id'], q['text'][:40], q['search_time_ms']) for q in d['by_query'] if q.get('search_time_ms', 0) > 1000]
print(f'   Avg: {avg_t:.0f}ms | Min: {min_t}ms | Max: {max_t}ms')
print(f'   Slow queries (>1s): {len(slow)}')
for qid, text, t in sorted(slow, key=lambda x: -x[2])[:5]:
    print(f'   > [{qid}] "{text}" -> {t}ms')

# ─── 6. TOP RESULT RELEVANCE SPOT CHECK ─────────────────────
print('\n6. TOP RESULT RELEVANCE (all 120)')
for q in d['by_query']:
    terms = q.get('translated_terms', [])
    top3 = q.get('top3', [])
    top_title = top3[0].get('title', 'N/A')[:55] if top3 else 'N/A'
    top_score = top3[0].get('score', 0) if top3 else 0
    dom = q.get('domain', '')
    term_str = ', '.join(terms[:3])[:40]
    print(f'   [{q["id"]:4s}] [{dom:6s}] [{top_score:3.0f}] {q["text"][:35]:35s} → {top_title}')

# ─── 7. DIVERSITY ANALYSIS ──────────────────────────────────
print('\n7. DIVERSITY')
print(f'   Unique books total: {d["diversity"]["unique_books_total"]}')
print(f'   Avg unique books/query: {d["diversity"]["avg_unique_books_per_query"]}')
books_per_q = [q.get('unique_books', 0) for q in d['by_query']]
low_div = [(q['id'], q['text'][:35], q.get('unique_books',0)) for q in d['by_query'] if q.get('unique_books',0) < 3]
print(f'   Low diversity (<3 books): {len(low_div)}')
for qid, text, nb in low_div[:5]:
    print(f'   ! [{qid}] "{text}" -> {nb} books')

# ─── 8. SUMMARY SCORECARD ───────────────────────────────────
print('\n' + '=' * 70)
print('SCORECARD SUMMARY')
print('=' * 70)
metrics = [
    ('Result Coverage', f'{d["queries_with_results"]}/{d["total_queries"]}', d["queries_with_results"]/d["total_queries"]*100),
    ('Translation Coverage', f'{d["translation_coverage"]["translated"]}/{d["total_queries"]}', d["translation_coverage"]["translated"]/d["total_queries"]*100),
    ('Domain Accuracy', f'{correct}/{total_checkable}', acc),
    ('Avg Search Time', f'{avg_t:.0f}ms', max(0, 100 - avg_t/10)),
    ('Snippet Quality', f'{d["snippet_quality"]["has_snippet"]}/{sum(d["snippet_quality"].values())}', d["snippet_quality"]["has_snippet"]/max(1,sum(d["snippet_quality"].values()))*100),
    ('Score Quality (90-100)', f'{sd["90-100"]}/{total_results}', sd["90-100"]/total_results*100),
    ('Book Diversity', f'{d["diversity"]["avg_unique_books_per_query"]:.1f}/10', d["diversity"]["avg_unique_books_per_query"]/10*100),
]
total_score = 0
for name, val, pct in metrics:
    bar = '#' * int(pct / 5)
    print(f'   {name:22s}: {val:12s} ({pct:5.1f}%) {bar}')
    total_score += pct
avg_score = total_score / len(metrics)
print(f'\n   OVERALL SYSTEM SCORE: {avg_score:.1f}%')
