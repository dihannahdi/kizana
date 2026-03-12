#!/bin/bash
DB=/opt/kizana/data/kizana_all_books.sqlite

echo '=== FAST INVESTIGATION ==='

# Get all t-tables that have كيفية التيمم
echo '--- Finding tables with كيفية التيمم ---'
# Use a smarter approach: query master to get table list, then do SQL union
tables=$(sqlite3 "$DB" "SELECT name FROM sqlite_master WHERE type='table' AND name LIKE 't%'" 2>/dev/null)

# Build a quick check by creating a temp script
FOUND=""
for t in $tables; do
    bnum=${t:1}
    result=$(sqlite3 "$DB" "SELECT '$bnum' FROM $t WHERE content LIKE '%كيفية التيمم%' LIMIT 1" 2>/dev/null)
    if [ -n "$result" ]; then
        FOUND="$FOUND $result"
        echo "  Found in book $result"
    fi
done

echo ''
echo "All books with كيفية التيمم: $FOUND"
echo ''

# Now for EACH found book, do the detailed check
for book_num in $FOUND; do
    echo "=============================="
    echo "BOOK $book_num"
    echo "=============================="
    
    # Book name
    sqlite3 "$DB" "SELECT substr(content,1,150) FROM b${book_num} WHERE (is_deleted='0' OR is_deleted IS NULL) ORDER BY id LIMIT 1" 2>/dev/null
    
    echo ''
    # TOC entries
    echo "TOC entries with كيفية التيمم:"
    sqlite3 "$DB" -separator '|||' "SELECT id, page, content FROM t${book_num} WHERE content LIKE '%كيفية التيمم%'" 2>/dev/null | while IFS='|||' read -r toc_id toc_page toc_content; do
        echo "  TOC_ID=$toc_id PAGE_FIELD=$toc_page"
        echo "  TITLE: $toc_content"
        
        # Content at the pointed row
        actual=$(sqlite3 "$DB" "SELECT page || '::' || substr(content,1,250) FROM b${book_num} WHERE id=$toc_page" 2>/dev/null)
        echo "  CONTENT_AT_ROW: $actual"
        
        # Check if tayammum text is in the window
        start=$((toc_page - 3))
        [ $start -lt 1 ] && start=1
        end=$((toc_page + 9))
        tayammum_count=$(sqlite3 "$DB" "SELECT COUNT(*) FROM b${book_num} WHERE id >= $start AND id <= $end AND content LIKE '%تيمم%'" 2>/dev/null)
        echo "  TAYAMMUM_ROWS_IN_WINDOW: $tayammum_count (from row $start to $end)"
        echo ''
    done
done
