#!/usr/bin/env python3
"""Analyze zero-result queries from eval to find translation gaps."""
import json
import sys
from collections import Counter

def main():
    fname = sys.argv[1] if len(sys.argv) > 1 else 'eval_results_v15.json'
    with open(fname, encoding='utf-8') as f:
        data = json.load(f)
    results = data['results']
    
    zeros = [r for r in results if r['num_results'] == 0]
    print(f'Zero results: {len(zeros)} / {len(results)} ({len(zeros)/len(results)*100:.1f}%)')
    
    # Split by zero-terms vs has-terms-but-no-result
    zero_term = [r for r in zeros if not r.get('translated_terms')]
    has_term_no_result = [r for r in zeros if r.get('translated_terms')]
    
    print(f'\n=== ZERO TERM (translation failure: {len(zero_term)}) ===')
    for r in zero_term[:100]:
        print(f'  {r["query_text"][:70]}')
    
    print(f'\n=== HAS TERMS BUT NO RESULT (index miss: {len(has_term_no_result)}) ===')
    for r in has_term_no_result[:50]:
        terms = r.get('translated_terms', [])[:4]
        print(f'  {r["query_text"][:50]:<52} -> {terms}')
    
    # Word frequency in zero-result queries
    print('\n=== MOST COMMON WORDS IN ZERO-RESULT QUERIES ===')
    stop = {'dan','atau','yang','di','ke','dari','untuk','dengan','pada','dalam',
            'adalah','ini','itu','juga','tidak','bisa','ada','sudah','akan',
            'the','a','an','of','to','in','is','and','for','on','with','what',
            'how','why','when','where','who'}
    word_count = Counter()
    for r in zeros:
        words = r['query_text'].lower().split()
        for w in words:
            w = w.strip('?,.')
            if len(w) > 2 and w not in stop:
                word_count[w] += 1
    for word, count in word_count.most_common(60):
        print(f'  {word:<30} {count}x')

if __name__ == '__main__':
    main()
