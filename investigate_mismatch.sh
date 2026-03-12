#!/bin/bash
DB=/opt/kizana/data/kizana_all_books.sqlite

echo '=========================================='
echo '=== INVESTIGATION: TOC-Content Mismatch ==='
echo '=========================================='

echo ''
echo '--- Step 1: Find book IDs with tayammum in TOC ---'
# Get all TOC tables and search for tayammum entries
for tbl in $(sqlite3 "$DB" "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 't%' ORDER BY name" | head -200); do
    book_num=${tbl:1}
    count=$(sqlite3 "$DB" "SELECT COUNT(*) FROM \"$tbl\" WHERE content LIKE '%التيمم%'" 2>/dev/null)
    if [ "$count" != "" ] && [ "$count" -gt 0 ] 2>/dev/null; then
        name_row=$(sqlite3 "$DB" "SELECT substr(content,1,80) FROM b${book_num} WHERE (is_deleted='0' OR is_deleted IS NULL) ORDER BY id LIMIT 1" 2>/dev/null)
        echo "Book $book_num ($count entries): $name_row"
    fi
done 2>/dev/null | head -30

echo ''
echo '--- Step 2: Detailed check for book 5350 (if exists) ---'
sqlite3 "$DB" "SELECT count(*) as toc_count FROM t5350" 2>/dev/null
sqlite3 "$DB" "SELECT id, page, substr(content,1,120) FROM t5350 WHERE content LIKE '%التيمم%' LIMIT 10" 2>/dev/null

echo ''
echo '--- Step 3: For each tayammum TOC entry, check what content is at that row ---'
# Pick first tayammum book found and check the mapping
for tbl in $(sqlite3 "$DB" "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 't%'" | head -500); do
    book_num=${tbl:1}
    entries=$(sqlite3 "$DB" "SELECT id||'|'||page||'|'||substr(content,1,80) FROM \"$tbl\" WHERE content LIKE '%كيفية التيمم%' LIMIT 3" 2>/dev/null)
    if [ -n "$entries" ]; then
        echo "=== BOOK $book_num: TOC entries with 'كيفية التيمم' ==="
        echo "$entries"
        # For each entry, check the content at toc.page (=row_id in b table)
        while IFS='|' read -r toc_id toc_page toc_content; do
            echo "  --> TOC id=$toc_id, page=$toc_page (= b${book_num}.id), toc_title: $toc_content"
            row_content=$(sqlite3 "$DB" "SELECT substr(content,1,200) FROM b${book_num} WHERE id=$toc_page" 2>/dev/null)
            row_page=$(sqlite3 "$DB" "SELECT page FROM b${book_num} WHERE id=$toc_page" 2>/dev/null)
            echo "  --> b${book_num} row $toc_page: display_page=$row_page"
            echo "  --> Content: $row_content"
            echo "  ---"
        done <<< "$entries"
        echo ''
    fi
done 2>/dev/null | head -100
