#!/bin/bash
DB=/opt/kizana/data/kizana_all_books.sqlite

echo '=== NON-BOOK TABLES ==='
sqlite3 "$DB" "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'b%' AND name NOT LIKE 't%' ORDER BY name"

echo '=== BOOKS TABLE SCHEMA ==='
sqlite3 "$DB" ".schema books" 2>/dev/null

echo '=== SAMPLE BOOKS ==='
sqlite3 "$DB" "SELECT * FROM books LIMIT 5" 2>/dev/null

echo '=== CHECK CONFIG ==='
sqlite3 "$DB" "SELECT name FROM sqlite_master WHERE type='table'" | grep -v '^[bt][0-9]' | head -20
