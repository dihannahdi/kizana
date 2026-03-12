#!/bin/bash
DB=/opt/kizana/data/kizana_all_books.sqlite

echo '============================================================'
echo '=== DEEP INVESTIGATION: TOC → Content Mapping Verification ==='
echo '============================================================'

# The user's search for "Tata cara tayammum" returned these books (from the screenshot):
# 1. فتاوى اللجنة الدائمة - hal 384
# 2. جعله الله خالصا - hal 133
# 3. مجموع فتاوى العلامة عبد العزيز بن باز - hal 187
# 4. صلاة المؤمن - hal 707
# 5. كتاب 12227 - hal 150
# 6. الموسوعة الفقهية الميسرة - hal 220

echo ''
echo '=== Step 1: Find book IDs for known books ==='
# Search by matching first page content with book names
# Book 107 = مجموع فتاوى ابن باز (already found)

# Check all t-tables for "كيفية التيمم" entries
echo 'Scanning TOC tables for كيفية التيمم...'
FOUND_BOOKS=""
for tbl in $(sqlite3 "$DB" "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 't%'"); do
    book_num=${tbl:1}
    has_match=$(sqlite3 "$DB" "SELECT 1 FROM \"$tbl\" WHERE content LIKE '%كيفية التيمم%' LIMIT 1" 2>/dev/null)
    if [ "$has_match" = "1" ]; then
        FOUND_BOOKS="$FOUND_BOOKS $book_num"
    fi
done

echo "Books with 'كيفية التيمم' in TOC: $FOUND_BOOKS"

echo ''
echo '=== Step 2: For each book, verify TOC→Content mapping ==='
for book_num in $FOUND_BOOKS; do
    echo ''
    echo "---------- BOOK $book_num ----------"
    
    # Get book name
    book_name=$(sqlite3 "$DB" "SELECT substr(content,1,120) FROM b${book_num} WHERE (is_deleted='0' OR is_deleted IS NULL) ORDER BY id LIMIT 1" 2>/dev/null)
    echo "Book name (page 1): $book_name"
    
    # Get all tayammum TOC entries
    echo "TOC entries:"
    sqlite3 "$DB" "SELECT id, page, substr(content,1,120) FROM t${book_num} WHERE content LIKE '%كيفية التيمم%'" 2>/dev/null | while IFS='|' read -r toc_id toc_page toc_content; do
        echo "  TOC: id=$toc_id, page_field=$toc_page"
        echo "  TOC title: $toc_content"
        
        # Now check what's actually at that row ID in b-table
        actual_content=$(sqlite3 "$DB" "SELECT substr(content,1,250) FROM b${book_num} WHERE id=$toc_page" 2>/dev/null)
        actual_page=$(sqlite3 "$DB" "SELECT page FROM b${book_num} WHERE id=$toc_page" 2>/dev/null)
        
        echo "  b${book_num} row $toc_page: display_page=$actual_page"
        echo "  Content at row: $actual_content"
        
        # Also check surrounding rows (what snippet extraction will get)
        echo "  --- Bidirectional window (row-3 to row+9): ---"
        start_id=$((toc_page - 3))
        if [ $start_id -lt 1 ]; then start_id=1; fi
        end_id=$((toc_page + 9))
        
        sqlite3 "$DB" "SELECT id, page, substr(content,1,100) FROM b${book_num} WHERE id >= $start_id AND id <= $end_id AND (is_deleted='0' OR is_deleted IS NULL) ORDER BY id" 2>/dev/null | while IFS='|' read -r row_id row_page row_content; do
            marker=""
            if [ "$row_id" = "$toc_page" ]; then marker=" <<<< TOC POINTS HERE"; fi
            echo "    row $row_id (page $row_page): ${row_content}${marker}"
        done
        
        # Check if the content mentions tayammum at all
        has_tayammum=$(sqlite3 "$DB" "SELECT COUNT(*) FROM b${book_num} WHERE id >= $start_id AND id <= $end_id AND content LIKE '%تيمم%'" 2>/dev/null)
        echo "  Rows mentioning تيمم in window: $has_tayammum"
        echo "  ..."
    done
done 2>/dev/null
