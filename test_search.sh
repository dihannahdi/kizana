#!/bin/bash
# Test the actual search API to see what results we get
echo '=== Testing search API for tayammum ==='
curl -s 'http://localhost:8080/search' \
  -H 'Content-Type: application/json' \
  -d '{"query":"تيمم","limit":25}' | python3 -m json.tool 2>/dev/null | head -200

echo ''
echo '=== Also test with Arabic expanded query ==='
curl -s 'http://localhost:8080/search' \
  -H 'Content-Type: application/json' \
  -d '{"query":"كيفية التيمم","limit":25}' | python3 -m json.tool 2>/dev/null | head -200
