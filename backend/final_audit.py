#!/usr/bin/env python3
"""Final v12 audit with correct query IDs."""
import json

d = json.load(open('eval_results_v14.json', 'r', encoding='utf-8'))
print('=' * 70)
print('FINAL v12 AUDIT')
print('=' * 70)
print(f'Total: {d["total_queries"]} queries')
print(f'With results: {d["queries_with_results"]}')
print(f'Zero results: {d["queries_no_results"]}')
print(f'Avg time: {d["avg_search_time_ms"]:.0f}ms')
print(f'Avg top score: {d["avg_top_score"]:.1f}')
print(f'Score dist: {d["score_distribution"]}')
print(f'Translation: {d["translation_coverage"]}')
print(f'Snippet: {d["snippet_quality"]}')
print(f'Diversity: {d["diversity"]}')
print()

checks = {
    'pu01': ('membatalkan puasa', ['مبطلات','مفسدات','نواقض','صيام'], 'عبادات'),
    'hj03': ('ihram dari miqat', ['إحرام','ميقات'], 'عبادات'),
    'aq06': ('qadha qadar', ['القضاء','القدر'], 'عقيدة'),
    'aq08': ('sifat dua puluh', ['صفات','العشرون'], None),
    'ts05': ('adab murid', ['آداب','متعلم','طالب'], 'أخلاق'),
    'kt08': ('operasi plastik', ['جراحة','تجميل'], None),
    'cm01': ('celana pendek', ['عورة','ستر','لباس'], 'عبادات'),
    'cm02': ('nikah beda agama', ['الدين','كتابية','اختلاف'], None),
    'ib13': ('imam perempuan', ['إمامة','المرأة'], 'عبادات'),
    'zk04': ('zakat emas', ['زكاة','ذهب'], 'عبادات'),
}

print('=' * 70)
print('KEY QUERY AUDIT (Previously Failing)')
print('=' * 70)
passed = 0
failed = 0

for qr in d['by_query']:
    if qr['id'] in checks:
        desc, expected, exp_domain = checks[qr['id']]
        terms = qr.get('translated_terms', [])
        found = [e for e in expected if any(e in t for t in terms)]
        n = qr.get('num_results', 0)
        domain = qr.get('domain', '')
        top = qr['top3'][0] if qr.get('top3') else {}
        top_title = top.get('title', 'N/A')[:60]
        top_score = top.get('score', 0)
        
        term_ok = n > 0 and len(found) > 0
        domain_ok = exp_domain is None or exp_domain in domain
        
        if term_ok and domain_ok:
            status = '✅ PASS'
            passed += 1
        else:
            status = '❌ FAIL'
            failed += 1
        
        print(f'\n{status} [{qr["id"]}] "{qr["text"]}"')
        print(f'  Domain: {domain} {"✓" if domain_ok else "✗ expected " + str(exp_domain)}')
        print(f'  Results: {n} | Terms: {len(terms)}')
        print(f'  Expected terms found: {found}/{expected}')
        print(f'  Top result: [{top_score:.0f}] {top_title}')

print(f'\n{"=" * 70}')
print(f'KEY QUERIES: {passed}/{passed+failed} PASSED')
print(f'{"=" * 70}')

# Full audit - check ALL queries
print()
print('=' * 70)
print('FULL QUERY AUDIT')
print('=' * 70)
total = len(d['by_query'])
zero = sum(1 for q in d['by_query'] if q.get('num_results',0) == 0)
low_terms = sum(1 for q in d['by_query'] if len(q.get('translated_terms',[])) < 2 and q.get('num_results',0) > 0)
no_trans = sum(1 for q in d['by_query'] if len(q.get('translated_terms',[])) == 0)

print(f'Total queries: {total}')
print(f'Zero results: {zero}')
print(f'No translation: {no_trans}')
print(f'Low terms (<2): {low_terms}')

# Show any remaining issues
issues = []
for q in d['by_query']:
    n = q.get('num_results', 0)
    terms = q.get('translated_terms', [])
    if n == 0:
        issues.append(f'  ❌ ZERO RESULTS: [{q["id"]}] "{q["text"]}"')
    if len(terms) == 0:
        issues.append(f'  ⚠️  NO TRANSLATION: [{q["id"]}] "{q["text"]}"')
    elif len(terms) < 2:
        issues.append(f'  ⚠️  LOW TERMS ({len(terms)}): [{q["id"]}] "{q["text"]}"')

if issues:
    print(f'\nRemaining issues ({len(issues)}):')
    for i in issues:
        print(i)
else:
    print('\n🎉 No remaining issues!')

# Score analysis
scores = []
for q in d['by_query']:
    if q.get('top3'):
        scores.append(q['top3'][0].get('score', 0))

if scores:
    avg = sum(scores)/len(scores)
    print(f'\nTop-1 Score: avg={avg:.1f}, min={min(scores):.1f}, max={max(scores):.1f}')

# Domain detection accuracy
print(f'\nDomain distribution: {d["domain_distribution"]}')
