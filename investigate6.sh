#!/bin/bash
DB=/opt/kizana/data/kizana_all_books.sqlite

echo '=== Find book IDs by name ==='

echo '--- Books matching فتاوى اللجنة الدائمة ---'
for tbl in $(sqlite3 "$DB" "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'b%'" | shuf | head -3000); do
    book_num=${tbl:1}
    match=$(sqlite3 "$DB" "SELECT 1 FROM \"$tbl\" WHERE content LIKE '%فتاوى اللجنة الدائمة%' AND (is_deleted='0' OR is_deleted IS NULL) LIMIT 1" 2>/dev/null)
    if [ "$match" = "1" ]; then
        echo "Book $book_num"
        break
    fi
done

echo '--- Books matching جعله الله خالصا ---'
for tbl in $(sqlite3 "$DB" "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 'b%'" | shuf | head -3000); do
    book_num=${tbl:1}
    match=$(sqlite3 "$DB" "SELECT 1 FROM \"$tbl\" WHERE content LIKE '%جعله الله خالصا%' AND (is_deleted='0' OR is_deleted IS NULL) LIMIT 1" 2>/dev/null)
    if [ "$match" = "1" ]; then
        echo "Book $book_num"
        break
    fi
done

echo ''
echo '=== Check which TOC entries have التيمم and are in top search results ==='
echo '--- Looking for TOC entries where title=كيفية التيمم but content at row is mismatched ---'

for book_num in 107 11036 11430 11446 12227 12109 12888 14594; do
    entries=$(sqlite3 "$DB" -separator '|' "SELECT id, page, substr(content,1,80) FROM t${book_num} WHERE content LIKE '%كيفية التيمم%'" 2>/dev/null)
    if [ -n "$entries" ]; then
        while IFS='|' read -r toc_id toc_page toc_title; do
            # Check if content at row has tayammum
            has_tayammum=$(sqlite3 "$DB" "SELECT CASE WHEN content LIKE '%تيمم%' THEN 'YES' ELSE 'NO' END FROM b${book_num} WHERE id=$toc_page" 2>/dev/null)
            # Get snippet of content
            content=$(sqlite3 "$DB" "SELECT substr(content,1,100) FROM b${book_num} WHERE id=$toc_page" 2>/dev/null)
            echo "Book=$book_num TOC_id=$toc_id row=$toc_page has_tayammum=$has_tayammum"
            echo "  Title: $toc_title"
            echo "  Content: $content"
            echo ""
        done <<< "$entries"
    fi
done

echo ''
echo '=== Check TOC with طريقة التيمم الصحيحة (from user screenshot result [3]) ==='
for tbl in $(sqlite3 "$DB" "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 't%'"); do
    book_num=${tbl:1}
    match=$(sqlite3 "$DB" "SELECT 1 FROM \"$tbl\" WHERE content LIKE '%طريقة التيمم الصحيحة%' LIMIT 1" 2>/dev/null)
    if [ "$match" = "1" ]; then
        echo "Found in book $book_num"
        sqlite3 "$DB" -separator '|' "SELECT id, page, substr(content,1,80) FROM t${book_num} WHERE content LIKE '%طريقة التيمم الصحيحة%'"
        # Check content
        toc_page=$(sqlite3 "$DB" "SELECT page FROM t${book_num} WHERE content LIKE '%طريقة التيمم الصحيحة%' LIMIT 1")
        content=$(sqlite3 "$DB" "SELECT substr(content,1,200) FROM b${book_num} WHERE id=$toc_page" 2>/dev/null)
        echo "  Content at row $toc_page: $content"
    fi
done 2>/dev/null
