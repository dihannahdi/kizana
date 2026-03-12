#!/bin/bash
DB="/opt/kizana/data/kizana_all_books.sqlite"

echo "=== Finding book with 'أسرار الطهارة' in TOC ==="
for tname in $(sqlite3 "$DB" "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 't%' AND name != 'toc_index'" 2>/dev/null); do
    bid=$(echo "$tname" | sed 's/^t//')
    result=$(sqlite3 "$DB" "SELECT id, substr(content,1,100) FROM \"$tname\" WHERE content LIKE '%أسرار الطهارة%' LIMIT 1" 2>/dev/null)
    if [ -n "$result" ]; then
        echo "  Book $bid: $result"
        toc_entry=$(sqlite3 "$DB" "SELECT id, page, parent FROM \"$tname\" WHERE content LIKE '%كيفية التيمم%' LIMIT 1" 2>/dev/null)
        echo "    TOC tayammum: $toc_entry"
    fi
done

echo ""
echo "=== Card [2] book detailed check ==="
echo "Looking for book with 'ربع العبادات' AND 'كيفية التيمم'..."
for tname in $(sqlite3 "$DB" "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 't%' AND name != 'toc_index'" 2>/dev/null); do
    bid=$(echo "$tname" | sed 's/^t//')
    result=$(sqlite3 "$DB" "SELECT COUNT(*) FROM \"$tname\" WHERE content LIKE '%ربع العبادات%'" 2>/dev/null)
    if [ "$result" -gt 0 ] 2>/dev/null; then
        tay=$(sqlite3 "$DB" "SELECT id, page FROM \"$tname\" WHERE content LIKE '%كيفية التيمم%' LIMIT 1" 2>/dev/null)
        if [ -n "$tay" ]; then
            echo "  Book $bid: TOC entry: $tay"
            page=$(echo "$tay" | cut -d'|' -f2)
            echo "    Content b$bid rows $((page-3)) to $((page+8)):"
            sqlite3 "$DB" "SELECT id, substr(content,1,250), page FROM b$bid WHERE id >= $((page-3)) AND id <= $((page+8)) ORDER BY id" 2>/dev/null
            echo "    Checking tayammum vs siwak keywords:"
            sqlite3 "$DB" "SELECT id, CASE WHEN content LIKE '%التيمم%' OR content LIKE '%تيمم%' THEN 'TAYAMMUM' ELSE '-' END, CASE WHEN content LIKE '%السواك%' OR content LIKE '%سواك%' THEN 'SIWAK' ELSE '-' END FROM b$bid WHERE id >= $((page-3)) AND id <= $((page+8)) ORDER BY id" 2>/dev/null
        fi
    fi
done

echo ""
echo "=== Card [1]: فتاوى اللجنة الدائمة ==="
echo "Check TOC table for book with 'فتاوى اللجنة الدائمة'..."
for tname in $(sqlite3 "$DB" "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 't%' AND name != 'toc_index'" 2>/dev/null); do
    bid=$(echo "$tname" | sed 's/^t//')
    result=$(sqlite3 "$DB" "SELECT id FROM \"$tname\" WHERE content LIKE '%كيفية التيمم%' LIMIT 1" 2>/dev/null)
    if [ -n "$result" ]; then
        meta=$(sqlite3 "$DB" "SELECT book_name, author_name FROM books_metadata WHERE book_id=$bid" 2>/dev/null)
        if echo "$meta" | grep -q "الدائمة"; then
            echo "  Book $bid: $meta"
            page=$(sqlite3 "$DB" "SELECT page FROM \"$tname\" WHERE content LIKE '%كيفية التيمم%' LIMIT 1" 2>/dev/null)
            echo "    TOC page=$page, content rows:"
            sqlite3 "$DB" "SELECT id, substr(content,1,250), page FROM b$bid WHERE id >= $((page-2)) AND id <= $((page+6)) ORDER BY id" 2>/dev/null
        fi 
    fi
done

echo ""
echo "=== Card [3]: مجموع فتاوى ابن باز - طريقة التيمم الصحيحة ==="
echo "Book 107, TOC 'طريقة التيمم الصحيحة':"
sqlite3 "$DB" "SELECT id, page, parent FROM t107 WHERE content LIKE '%طريقة التيمم الصحيحة%' LIMIT 3" 2>/dev/null
echo "Content at that page (expecting page around 2261 area):"
page=$(sqlite3 "$DB" "SELECT page FROM t107 WHERE content LIKE '%طريقة التيمم الصحيحة%' LIMIT 1" 2>/dev/null)
echo "Page field: $page"
sqlite3 "$DB" "SELECT id, substr(content,1,250), page FROM b107 WHERE id >= $((page-1)) AND id <= $((page+5)) ORDER BY id" 2>/dev/null
echo "Checking tayammum keyword:"
sqlite3 "$DB" "SELECT id, CASE WHEN content LIKE '%التيمم%' OR content LIKE '%تيمم%' THEN 'TAYAMMUM' ELSE '-' END FROM b107 WHERE id >= $((page-1)) AND id <= $((page+5)) ORDER BY id" 2>/dev/null

echo ""
echo "=== Check handlers.rs book reading ==="
grep -n 'fn read_book\|fn get_book\|book_id.*page\|display_page' /opt/kizana/backend/src/handlers.rs | head -20

echo ""
echo "=== Check models.rs SearchResult ==="
grep -n 'page\|snippet\|content' /opt/kizana/backend/src/models.rs | head -30

echo ""
echo "=== Check how book viewer receives page ==="
grep -n 'fn.*book\|page.*content\|BookPage\|BookRead' /opt/kizana/backend/src/handlers.rs | head -20

echo ""
echo "DONE"
