#!/bin/bash
# Test actual search API
echo '=== Testing search API for tayammum ==='
RESULTS=$(curl -s 'http://127.0.0.1:8080/search' \
  -H 'Content-Type: application/json' \
  -d '{"query":"Tata cara tayammum","limit":25}')

echo "$RESULTS" | python3 -c "
import json, sys
data = json.load(sys.stdin)
results = data.get('results', [])
print(f'Total results: {len(results)}')
print()
for i, r in enumerate(results):
    print(f'[{i+1}] book_id={r[\"book_id\"]}, toc_id={r[\"toc_id\"]}, page={r[\"page\"]}, score={r[\"score\"]:.2f}')
    print(f'    book_name: {r[\"book_name\"]}')
    print(f'    title: {r[\"title\"][:100]}')
    print(f'    snippet: {r[\"content_snippet\"][:150]}')
    print()
" 2>/dev/null
