#!/bin/bash
DB=/opt/kizana/backend/books.db

echo '=== TABLES SAMPLE ==='
sqlite3 "$DB" '.tables' | tr ' ' '\n' | grep -v '^$' | head -20

echo '=== Check books metadata ==='
sqlite3 "$DB" "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'b%' AND name NOT LIKE 't%' ORDER BY name" 2>/dev/null

echo '=== Check t5350 count ==='
sqlite3 "$DB" 'SELECT count(*) FROM t5350' 2>/dev/null

echo '=== Check b5350 count ==='
sqlite3 "$DB" 'SELECT count(*) FROM b5350' 2>/dev/null
