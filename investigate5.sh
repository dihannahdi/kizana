#!/bin/bash
DB=/opt/kizana/data/kizana_all_books.sqlite

# Check specific books found: 107, 11036, 11430, 11446, 12227
for book_num in 107 11036 11430 11446 12227; do
    echo "=============================="
    echo "BOOK $book_num"
    echo "=============================="
    sqlite3 "$DB" "SELECT substr(content,1,100) FROM b${book_num} WHERE (is_deleted='0' OR is_deleted IS NULL) ORDER BY id LIMIT 1"
    echo "---TOC entries---"
    sqlite3 "$DB" -separator '|' "SELECT id, page, substr(content,1,100) FROM t${book_num} WHERE content LIKE '%التيمم%' LIMIT 5"
    echo ""
done

echo "=============================="
echo "DEEP CHECK BOOK 107"
echo "=============================="
# Get TOC entry for كيفية التيمم
TOC_PAGE=$(sqlite3 "$DB" "SELECT page FROM t107 WHERE content LIKE '%كيفية التيمم%' LIMIT 1")
echo "TOC page field (=row_id in b107): $TOC_PAGE"

# Show what's at that row and surrounding rows
echo "---Content at row $TOC_PAGE and surroundings---"
START=$((TOC_PAGE - 3))
END=$((TOC_PAGE + 9))
sqlite3 "$DB" -separator '|' "SELECT id, page, substr(content,1,150) FROM b107 WHERE id >= $START AND id <= $END AND (is_deleted='0' OR is_deleted IS NULL) ORDER BY id"

echo ""
echo "=============================="
echo "DEEP CHECK BOOK 11430"
echo "=============================="
TOC_PAGE2=$(sqlite3 "$DB" "SELECT page FROM t11430 WHERE content LIKE '%كيفية التيمم%' LIMIT 1")
echo "TOC page field: $TOC_PAGE2"
START2=$((TOC_PAGE2 - 3))
END2=$((TOC_PAGE2 + 9))
sqlite3 "$DB" -separator '|' "SELECT id, page, substr(content,1,150) FROM b11430 WHERE id >= $START2 AND id <= $END2 AND (is_deleted='0' OR is_deleted IS NULL) ORDER BY id"

echo ""
echo "=============================="
echo "DEEP CHECK BOOK 12227"
echo "=============================="
TOC_PAGE3=$(sqlite3 "$DB" "SELECT page FROM t12227 WHERE content LIKE '%كيفية التيمم%' LIMIT 1")
echo "TOC page field: $TOC_PAGE3"
START3=$((TOC_PAGE3 - 3))
END3=$((TOC_PAGE3 + 9))
sqlite3 "$DB" -separator '|' "SELECT id, page, substr(content,1,150) FROM b12227 WHERE id >= $START3 AND id <= $END3 AND (is_deleted='0' OR is_deleted IS NULL) ORDER BY id"

echo ""
echo "=============================="  
echo "CHECK WHAT B107 ROW 2261 CONTENT LOOKS LIKE (full)"
echo "=============================="
sqlite3 "$DB" "SELECT content FROM b107 WHERE id = 2261"
