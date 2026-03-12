#!/bin/bash
DB=/opt/kizana/data/kizana_all_books.sqlite

echo '=== Find ALL books with tayammum TOC entries ==='
# Search all TOC tables for tayammum
for tbl in $(sqlite3 "$DB" "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 't%'"); do
    book_num=${tbl:1}
    count=$(sqlite3 "$DB" "SELECT COUNT(*) FROM \"$tbl\" WHERE content LIKE '%التيمم%'" 2>/dev/null)
    if [ "$count" != "" ] && [ "$count" -gt 0 ] 2>/dev/null; then
        # Get book name from first page
        first_content=$(sqlite3 "$DB" "SELECT substr(content,1,100) FROM b${book_num} WHERE (is_deleted='0' OR is_deleted IS NULL) ORDER BY id LIMIT 1" 2>/dev/null)
        echo "BOOK_${book_num}|${count}|${first_content}"
    fi
done 2>/dev/null

echo ''
echo '=== Specific checks for reported mismatched books ==='

echo ''
echo '--- Check book name matching "فتاوى اللجنة الدائمة" ---'
for tbl in $(sqlite3 "$DB" "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'b%'" | head -8000); do
    book_num=${tbl:1}
    match=$(sqlite3 "$DB" "SELECT 1 FROM \"$tbl\" WHERE content LIKE '%فتاوى اللجنة الدائمة%' LIMIT 1" 2>/dev/null)
    if [ "$match" = "1" ]; then
        echo "Found 'فتاوى اللجنة الدائمة' in book $book_num"
        # now check TOC entries
        sqlite3 "$DB" "SELECT id, page, substr(content,1,100) FROM t${book_num} WHERE content LIKE '%التيمم%' LIMIT 5" 2>/dev/null | while read line; do
            echo "  TOC: $line"
        done
    fi
done 2>/dev/null | head -30
