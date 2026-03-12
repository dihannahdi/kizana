#!/bin/bash
DB="/opt/kizana/data/kizana_all_books.sqlite"

echo "=========================================="
echo "INVESTIGATION: Snippet relevance for tayammum search"
echo "=========================================="

# From screenshot, Card [2] shows "كيفية التيمم" from "كتاب أسرار الطهارة" "ربع العبادات"
# with content about siwak (مطهرة للفم ومرضاة للرب), which is IRRELEVANT
# Let's find which book this is

echo ""
echo "=== Step 1: Find books with TOC matching 'كيفية التيمم' ==="
sqlite3 "$DB" "
SELECT 'book_' || substr(name,2), name 
FROM sqlite_master 
WHERE type='table' AND name LIKE 't%' AND name != 'toc_index'
" | while IFS='|' read book_label tname; do
    bid=$(echo "$tname" | sed 's/^t//')
    result=$(sqlite3 "$DB" "SELECT id, substr(content,1,150), page, parent FROM \"$tname\" WHERE content LIKE '%كيفية التيمم%' LIMIT 1" 2>/dev/null)
    if [ -n "$result" ]; then
        echo "  Book $bid: $result"
    fi
done

echo ""
echo "=== Step 2: Check card [2] - Book with 'كتاب أسرار الطهارة' in TOC ==="
# The screenshot shows the hierarchy: كتاب أسرار الطهارة > ربع العبادات > القسم الثاني...
# Let's find which book has this
sqlite3 "$DB" "
SELECT name FROM sqlite_master 
WHERE type='table' AND name LIKE 't%' AND name != 'toc_index'
" | while read tname; do
    bid=$(echo "$tname" | sed 's/^t//')
    result=$(sqlite3 "$DB" "SELECT id, substr(content,1,100) FROM \"$tname\" WHERE content LIKE '%أسرار الطهارة%' LIMIT 1" 2>/dev/null)
    if [ -n "$result" ]; then
        echo "  Book $bid ($tname): $result"
        # Now find what that book's كيفية التيمم TOC entry points to
        toc_info=$(sqlite3 "$DB" "SELECT id, page, parent FROM \"$tname\" WHERE content LIKE '%كيفية التيمم%'" 2>/dev/null)
        echo "    TOC 'كيفية التيمم' entries: $toc_info"
        if [ -n "$toc_info" ]; then
            page=$(echo "$toc_info" | head -1 | cut -d'|' -f2)
            echo "    Content at page=$page (row $page in b$bid):"
            sqlite3 "$DB" "SELECT id, substr(content,1,200), page FROM b$bid WHERE id >= $page AND id <= $((page+5)) LIMIT 6" 2>/dev/null
        fi
    fi
done

echo ""
echo "=== Step 3: Card [1] - فتاوى اللجنة الدائمة ==="
# Card [1] shows كيفية التيمم from المجلد الخامس (الفقه - الطهارة)
# Author: اللجنة الدائمة للبحوث العلمية والإفتاء
sqlite3 "$DB" "
SELECT name FROM sqlite_master 
WHERE type='table' AND name LIKE 't%' AND name != 'toc_index'
" | while read tname; do
    bid=$(echo "$tname" | sed 's/^t//')
    result=$(sqlite3 "$DB" "SELECT id FROM \"$tname\" WHERE content LIKE '%اللجنة الدائمة%' AND content LIKE '%الطهارة%' LIMIT 1" 2>/dev/null)
    if [ -n "$result" ]; then
        toc_entry=$(sqlite3 "$DB" "SELECT id, page FROM \"$tname\" WHERE content LIKE '%كيفية التيمم%' LIMIT 1" 2>/dev/null)
        if [ -n "$toc_entry" ]; then
            page=$(echo "$toc_entry" | cut -d'|' -f2)
            echo "  Book $bid: TOC entry $toc_entry"
            echo "    Content at page=$page:"
            sqlite3 "$DB" "SELECT id, substr(content,1,300) FROM b$bid WHERE id >= $page AND id <= $((page+3)) LIMIT 4" 2>/dev/null
        fi
    fi
done

echo ""
echo "=== Step 4: Card [3] - مجموع فتاوى العلامة عبد العزيز بن باز ==="
# Shows طريقة التيمم الصحيحة, page 187
sqlite3 "$DB" "
SELECT name FROM sqlite_master 
WHERE type='table' AND name LIKE 't%' AND name != 'toc_index'
" | while read tname; do
    bid=$(echo "$tname" | sed 's/^t//')
    result=$(sqlite3 "$DB" "SELECT id, page FROM \"$tname\" WHERE content LIKE '%طريقة التيمم الصحيحة%' LIMIT 1" 2>/dev/null)
    if [ -n "$result" ]; then
        page=$(echo "$result" | cut -d'|' -f2)
        echo "  Book $bid: TOC entry $result"
        echo "    Content at page=$page:"
        sqlite3 "$DB" "SELECT id, substr(content,1,300) FROM b$bid WHERE id >= $page AND id <= $((page+3)) LIMIT 4" 2>/dev/null
    fi
done

echo ""
echo "=== Step 5: Verify snippet extraction logic ==="
echo "Testing the sliding window on card [2] example..."
# Let's check what the bidirectional fetch gets

echo ""
echo "=== Step 6: Check the search terms that would be generated for 'كيفية التيمم' ==="
echo "The TOC content field contains 'كيفية التيمم' as the title."
echo "When searching, translated.arabic_terms would be the Arabic terms from the search query."
echo "If user searched 'tata cara tayammum' or 'كيفية التيمم', the terms should include تيمم."

echo ""
echo "=== Step 7: Check book viewer - what does the /book endpoint return? ==="
echo "Card [1] displays page 384. Let's check what b{book_id} has at the row matching page field."
# For card [3]: page 187 from book مجموع فتاوى
# Let's check if book viewer uses display page or row_id

echo ""
echo "=== DONE ==="
