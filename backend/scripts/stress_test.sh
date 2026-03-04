#!/bin/bash
# Kizana Search - Comprehensive Stress Test Script
# Tests all major endpoints with increasing concurrency levels

RESULTS_DIR=/tmp/stress_results
mkdir -p $RESULTS_DIR

echo '{"tests": [' > $RESULTS_DIR/all_results.json
FIRST=true

run_wrk_test() {
    local name="$1"
    local url="$2"
    local connections="$3"
    local threads="$4"
    local duration="$5"
    
    echo "=== Testing: $name (c=$connections, t=$threads, d=$duration) ==="
    
    OUTPUT=$(wrk -t$threads -c$connections -d$duration --latency "$url" 2>&1)
    
    # Parse wrk output
    REQS_SEC=$(echo "$OUTPUT" | grep 'Requests/sec' | awk '{print $2}')
    TRANSFER_SEC=$(echo "$OUTPUT" | grep 'Transfer/sec' | awk '{print $2}')
    LAT_50=$(echo "$OUTPUT" | grep '50%' | awk '{print $2}')
    LAT_75=$(echo "$OUTPUT" | grep '75%' | awk '{print $2}')
    LAT_90=$(echo "$OUTPUT" | grep '90%' | awk '{print $2}')
    LAT_99=$(echo "$OUTPUT" | grep '99%' | awk '{print $2}')
    TOTAL_REQS=$(echo "$OUTPUT" | grep 'requests in' | awk '{print $1}')
    ERRORS_SOCKET=$(echo "$OUTPUT" | grep 'Socket errors' || echo '')
    ERRORS_NON2XX=$(echo "$OUTPUT" | grep 'Non-2xx' || echo '')
    ERRORS="$ERRORS_SOCKET $ERRORS_NON2XX"
    if [ -z "$(echo "$ERRORS" | tr -d ' ')" ]; then
        ERRORS="None"
    fi
    LATENCY_LINE=$(echo "$OUTPUT" | grep 'Latency' | head -1)
    LAT_AVG=$(echo "$LATENCY_LINE" | awk '{print $2}')
    LAT_STDEV=$(echo "$LATENCY_LINE" | awk '{print $3}')
    LAT_MAX=$(echo "$LATENCY_LINE" | awk '{print $4}')
    
    if [ "$FIRST" = true ]; then
        FIRST=false
    else
        echo ',' >> $RESULTS_DIR/all_results.json
    fi
    
    RAW=$(echo "$OUTPUT" | python3 -c "import sys,json; print(json.dumps(sys.stdin.read()))" 2>/dev/null || echo '""')
    
    cat >> $RESULTS_DIR/all_results.json << JSONEND
{
  "name": "$name",
  "url": "$url",
  "connections": $connections,
  "threads": $threads,
  "duration": "$duration",
  "requests_per_sec": "$REQS_SEC",
  "transfer_per_sec": "$TRANSFER_SEC",
  "total_requests": "$TOTAL_REQS",
  "latency_avg": "$LAT_AVG",
  "latency_stdev": "$LAT_STDEV",
  "latency_max": "$LAT_MAX",
  "latency_p50": "$LAT_50",
  "latency_p75": "$LAT_75",
  "latency_p90": "$LAT_90",
  "latency_p99": "$LAT_99",
  "errors": "$ERRORS",
  "raw_output": $RAW
}
JSONEND
    
    echo "$OUTPUT"
    echo ""
}

# ===== TEST 1: Frontend Homepage =====
echo '>>> FRONTEND HOMEPAGE <<<'
run_wrk_test 'Homepage_10c' 'https://bahtsulmasail.tech/' 10 2 '15s'
run_wrk_test 'Homepage_50c' 'https://bahtsulmasail.tech/' 50 4 '15s'
run_wrk_test 'Homepage_100c' 'https://bahtsulmasail.tech/' 100 4 '15s'
run_wrk_test 'Homepage_200c' 'https://bahtsulmasail.tech/' 200 4 '15s'

# ===== TEST 2: Produk Hukum Stats API =====
echo '>>> PRODUK HUKUM STATS <<<'
run_wrk_test 'PH_Stats_10c' 'https://bahtsulmasail.tech/api/produk-hukum/stats' 10 2 '15s'
run_wrk_test 'PH_Stats_50c' 'https://bahtsulmasail.tech/api/produk-hukum/stats' 50 4 '15s'
run_wrk_test 'PH_Stats_100c' 'https://bahtsulmasail.tech/api/produk-hukum/stats' 100 4 '15s'
run_wrk_test 'PH_Stats_200c' 'https://bahtsulmasail.tech/api/produk-hukum/stats' 200 4 '15s'

# ===== TEST 3: Produk Hukum List API =====
echo '>>> PRODUK HUKUM LIST <<<'
run_wrk_test 'PH_List_10c' 'https://bahtsulmasail.tech/api/produk-hukum/list?page=1&per_page=20' 10 2 '15s'
run_wrk_test 'PH_List_50c' 'https://bahtsulmasail.tech/api/produk-hukum/list?page=1&per_page=20' 50 4 '15s'
run_wrk_test 'PH_List_100c' 'https://bahtsulmasail.tech/api/produk-hukum/list?page=1&per_page=20' 100 4 '15s'
run_wrk_test 'PH_List_200c' 'https://bahtsulmasail.tech/api/produk-hukum/list?page=1&per_page=20' 200 4 '15s'

# ===== TEST 4: Produk Hukum Search (FTS5) =====
echo '>>> PRODUK HUKUM SEARCH <<<'
run_wrk_test 'PH_Search_10c' 'https://bahtsulmasail.tech/api/produk-hukum/search?q=nikah&limit=30' 10 2 '15s'
run_wrk_test 'PH_Search_50c' 'https://bahtsulmasail.tech/api/produk-hukum/search?q=nikah&limit=30' 50 4 '15s'
run_wrk_test 'PH_Search_100c' 'https://bahtsulmasail.tech/api/produk-hukum/search?q=nikah&limit=30' 100 4 '15s'
run_wrk_test 'PH_Search_200c' 'https://bahtsulmasail.tech/api/produk-hukum/search?q=nikah&limit=30' 200 4 '15s'

# ===== TEST 5: Produk Hukum Detail =====
echo '>>> PRODUK HUKUM DETAIL <<<'
run_wrk_test 'PH_Detail_10c' 'https://bahtsulmasail.tech/api/produk-hukum/detail/1' 10 2 '15s'
run_wrk_test 'PH_Detail_50c' 'https://bahtsulmasail.tech/api/produk-hukum/detail/1' 50 4 '15s'
run_wrk_test 'PH_Detail_100c' 'https://bahtsulmasail.tech/api/produk-hukum/detail/1' 100 4 '15s'
run_wrk_test 'PH_Detail_200c' 'https://bahtsulmasail.tech/api/produk-hukum/detail/1' 200 4 '15s'

# ===== TEST 6: Static Assets (Tentang page) =====
echo '>>> TENTANG PAGE <<<'
run_wrk_test 'Tentang_50c' 'https://bahtsulmasail.tech/tentang' 50 4 '15s'
run_wrk_test 'Tentang_200c' 'https://bahtsulmasail.tech/tentang' 200 4 '15s'

# ===== TEST 7: Statistik Page =====
echo '>>> STATISTIK PAGE <<<'
run_wrk_test 'Statistik_50c' 'https://bahtsulmasail.tech/statistik' 50 4 '15s'
run_wrk_test 'Statistik_200c' 'https://bahtsulmasail.tech/statistik' 200 4 '15s'

# ===== TEST 8: Produk Hukum Page (Frontend) =====
echo '>>> PRODUK HUKUM PAGE <<<'
run_wrk_test 'PH_Page_50c' 'https://bahtsulmasail.tech/produk-hukum' 50 4 '15s'
run_wrk_test 'PH_Page_200c' 'https://bahtsulmasail.tech/produk-hukum' 200 4 '15s'

echo ']}' >> $RESULTS_DIR/all_results.json

# Get system metrics
echo '>>> SYSTEM METRICS <<<'
HOSTNAME_VAL=$(hostname)
CPU_CORES=$(nproc)
MEM_TOTAL=$(free -m | awk '/Mem:/ {print $2}')
MEM_USED=$(free -m | awk '/Mem:/ {print $3}')
DISK_TOTAL=$(df -BG / | awk 'NR==2 {print $2}' | tr -d 'G')
DISK_USED=$(df -BG / | awk 'NR==2 {print $3}' | tr -d 'G')
OS_NAME=$(lsb_release -ds 2>/dev/null || grep PRETTY_NAME /etc/os-release | cut -d= -f2 | tr -d '"')

cat > $RESULTS_DIR/system_info.json << SYSEND
{
  "hostname": "$HOSTNAME_VAL",
  "cpu_cores": $CPU_CORES,
  "memory_total_mb": $MEM_TOTAL,
  "memory_used_mb": $MEM_USED,
  "disk_total_gb": $DISK_TOTAL,
  "disk_used_gb": $DISK_USED,
  "os": "$OS_NAME"
}
SYSEND

echo '=== STRESS TESTS COMPLETE ==='
echo "Results saved to $RESULTS_DIR/"
