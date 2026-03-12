#!/usr/bin/env python3
import json
d = json.load(open('eval_results_v14.json'))
print('Total:', d['total_queries'])
print('Results:', d['queries_with_results'])
print('Zero:', d['queries_no_results'])
print('Trans:', d['translation_coverage'])
print('Domains:', d['domain_distribution'])
print('AvgTime:', d['avg_search_time_ms'])

# Check the 10 previously wrong domains
checks = {
    'th10': 'طهارة', 'mu08': 'معاملات', 'mu09': 'معاملات',
    'mu11': 'معاملات', 'mu12': 'معاملات', 'aq04': 'عقيدة',
    'aq05': 'عقيدة', 'aq08': 'عقيدة', 'jn02': 'جنايات', 'jn04': 'جنايات'
}
print('\nDomain Fix Check:')
for q in d['by_query']:
    if q['id'] in checks:
        exp = checks[q['id']]
        actual = q.get('domain', '')
        ok = exp in actual
        print(f'  {"V" if ok else "X"} [{q["id"]}] "{q["text"][:40]}" → {actual} (expected: {exp})')
