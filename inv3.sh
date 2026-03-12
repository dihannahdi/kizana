#!/bin/bash
DB="/opt/kizana/data/kizana_all_books.sqlite"

# Card [2] from screenshot has hierarchy: ربع العبادات > كتاب أسرار الطهارة
# The snippet shows siwak text. Let's find this book.

# Known from step1: These books have كيفية التيمم:
# 107, 11036, 11430, 11446, 11468, 12109, 12227, 122392, 127703, 12888

# Check each for ربع العبادات
for bid in 107 11036 11430 11446 11468 12109 12227 122392 127703 12888; do
    has_rub=$(sqlite3 "$DB" "SELECT COUNT(*) FROM t$bid WHERE content LIKE '%ربع العبادات%'" 2>/dev/null)
    if [ "$has_rub" -gt 0 ] 2>/dev/null; then
        echo "FOUND: Book $bid has ربع العبادات"
        page=$(sqlite3 "$DB" "SELECT page FROM t$bid WHERE content LIKE '%كيفية التيمم%' LIMIT 1" 2>/dev/null)
        echo "  TOC page=$page"
        echo "  Content rows around page=$page in b$bid:"
        for i in $(seq $((page-3)) $((page+8))); do
            row=$(sqlite3 "$DB" "SELECT id,substr(content,1,200),page FROM b$bid WHERE id=$i" 2>/dev/null)
            if [ -n "$row" ]; then
                echo "    $row"
            fi
        done
    fi
done

echo ""
echo "=== Card [1]: Check book 96251 ==="
page96251=$(sqlite3 "$DB" "SELECT page FROM t96251 WHERE content LIKE '%كيفية التيمم%' LIMIT 1" 2>/dev/null)
echo "Book 96251 TOC page=$page96251"
if [ -n "$page96251" ]; then
    for i in $(seq $((page96251)) $((page96251+5))); do
        row=$(sqlite3 "$DB" "SELECT id,substr(content,1,200),page FROM b96251 WHERE id=$i" 2>/dev/null)
        if [ -n "$row" ]; then
            echo "  $row"
        fi
    done
fi

echo ""
echo "=== Card [3]: Book 107, طريقة التيمم الصحيحة ==="
page107=$(sqlite3 "$DB" "SELECT page FROM t107 WHERE content LIKE '%طريقة التيمم الصحيحة%' LIMIT 1" 2>/dev/null)
echo "Book 107 TOC page=$page107"
if [ -n "$page107" ]; then
    for i in $(seq $((page107)) $((page107+5))); do
        row=$(sqlite3 "$DB" "SELECT id,substr(content,1,200),page FROM b107 WHERE id=$i" 2>/dev/null)
        if [ -n "$row" ]; then
            echo "  $row"
        fi
    done
fi

echo ""
echo "=== Book viewer: how page is used ==="
grep -n 'fn read_book\|fn get_page\|fn book_page\|BookReadRequest\|book_id.*page' /opt/kizana/backend/src/handlers.rs | head -15

echo ""
echo "=== SearchResult page field ==="
sed -n '/pub struct SearchResult/,/}/p' /opt/kizana/backend/src/models.rs
