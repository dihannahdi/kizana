#!/usr/bin/env python3
"""
Mass evaluation runner for Kizana Search - 10K+ queries.
Submits queries in batches to the eval endpoint, collects results.
"""
import json
import time
import requests
import sys
import os
from datetime import datetime

# Always run from script directory
os.chdir(os.path.dirname(os.path.abspath(__file__)))

API_BASE = "https://bahtsulmasail.tech"
EMAIL = "eval_admin@bahtsulmasail.tech"
PASSWORD = "EvalTest2026"
BATCH_SIZE = 50  # queries per API call
MAX_RESULTS = 10  # results per query (keep low for speed)
OUTPUT_FILE = sys.argv[1] if len(sys.argv) > 1 else "eval_results_10k.json"
PROGRESS_FILE = OUTPUT_FILE.replace(".json", "_progress.json")

def login():
    """Get JWT token."""
    r = requests.post(f"{API_BASE}/api/auth/login",
                      json={"email": EMAIL, "password": PASSWORD},
                      timeout=30)
    r.raise_for_status()
    data = r.json()
    token = data.get("token")
    if not token:
        print(f"Login failed: {data}")
        sys.exit(1)
    print(f"Logged in successfully")
    return token

def load_queries():
    """Load the 10K query set."""
    with open("queries_10k_eval.json", "r", encoding="utf-8") as f:
        return json.load(f)

def load_progress():
    """Load progress state if resuming."""
    if os.path.exists(PROGRESS_FILE):
        with open(PROGRESS_FILE, "r", encoding="utf-8") as f:
            return json.load(f)
    return {"completed_batches": 0, "results": []}

def save_progress(progress):
    """Save progress for resume capability."""
    with open(PROGRESS_FILE, "w", encoding="utf-8") as f:
        json.dump(progress, f, ensure_ascii=False)

def run_batch(token, queries, config=None):
    """Submit a batch of queries."""
    payload = {
        "queries": queries,
        "config": config or {},
        "max_results": MAX_RESULTS,
    }
    headers = {"Authorization": f"Bearer {token}"}
    r = requests.post(f"{API_BASE}/api/eval/batch",
                      json=payload, headers=headers, timeout=300)
    r.raise_for_status()
    return r.json()

def main():
    print("=" * 60)
    print("KIZANA SEARCH — 10K MASS EVALUATION")
    print("=" * 60)
    
    # Login
    token = login()
    
    # Load queries
    all_queries = load_queries()
    total = len(all_queries)
    print(f"Total queries to evaluate: {total}")
    
    # Load progress
    progress = load_progress()
    start_batch = progress["completed_batches"]
    all_results = progress["results"]
    
    if start_batch > 0:
        print(f"Resuming from batch {start_batch} ({start_batch * BATCH_SIZE} queries done)")
    
    # Split into batches
    batches = []
    for i in range(0, total, BATCH_SIZE):
        batches.append(all_queries[i:i+BATCH_SIZE])
    
    total_batches = len(batches)
    print(f"Total batches: {total_batches} (size={BATCH_SIZE})")
    print("-" * 60)
    
    start_time = time.time()
    errors = 0
    
    for batch_idx in range(start_batch, total_batches):
        batch = batches[batch_idx]
        batch_start = time.time()
        
        try:
            result = run_batch(token, batch)
            batch_time = time.time() - batch_start
            
            # Extract per-query results
            for qr in result.get("results", []):
                all_results.append({
                    "query_id": qr["query_id"],
                    "query_text": qr["query_text"],
                    "translated_terms": qr.get("translated_terms", []),
                    "detected_language": qr.get("detected_language", ""),
                    "detected_domain": qr.get("detected_domain", ""),
                    "num_results": qr.get("num_results", 0),
                    "search_time_ms": qr.get("search_time_ms", 0),
                    "top_scores": [r["score"] for r in qr.get("results", [])[:5]],
                    "top_books": [r.get("book_name", "") for r in qr.get("results", [])[:3]],
                    "top_titles": [r.get("title", "") for r in qr.get("results", [])[:3]],
                })
            
            done = (batch_idx + 1) * BATCH_SIZE
            if done > total:
                done = total
            elapsed = time.time() - start_time
            rate = done / elapsed if elapsed > 0 else 0
            eta = (total - done) / rate if rate > 0 else 0
            
            print(f"  Batch {batch_idx+1}/{total_batches} — "
                  f"{done}/{total} queries — "
                  f"{batch_time:.1f}s — "
                  f"rate: {rate:.1f} q/s — "
                  f"ETA: {eta:.0f}s")
            
        except requests.exceptions.HTTPError as e:
            errors += 1
            print(f"  Batch {batch_idx+1} ERROR: {e}")
            if e.response.status_code == 401:
                print("  Re-authenticating...")
                token = login()
                # Retry this batch
                try:
                    result = run_batch(token, batch)
                    for qr in result.get("results", []):
                        all_results.append({
                            "query_id": qr["query_id"],
                            "query_text": qr["query_text"],
                            "translated_terms": qr.get("translated_terms", []),
                            "detected_language": qr.get("detected_language", ""),
                            "detected_domain": qr.get("detected_domain", ""),
                            "num_results": qr.get("num_results", 0),
                            "search_time_ms": qr.get("search_time_ms", 0),
                            "top_scores": [r["score"] for r in qr.get("results", [])[:5]],
                            "top_books": [r.get("book_name", "") for r in qr.get("results", [])[:3]],
                            "top_titles": [r.get("title", "") for r in qr.get("results", [])[:3]],
                        })
                except Exception as e2:
                    print(f"  Retry failed: {e2}")
        except Exception as e:
            errors += 1
            print(f"  Batch {batch_idx+1} EXCEPTION: {e}")
        
        # Save progress every 10 batches
        if (batch_idx + 1) % 10 == 0:
            progress["completed_batches"] = batch_idx + 1
            progress["results"] = all_results
            save_progress(progress)
    
    total_time = time.time() - start_time
    
    # Save final results
    output = {
        "metadata": {
            "total_queries": total,
            "total_results_collected": len(all_results),
            "total_time_seconds": round(total_time, 1),
            "avg_time_per_query_ms": round(total_time * 1000 / max(len(all_results), 1), 1),
            "errors": errors,
            "timestamp": datetime.now().isoformat(),
            "config": "default (all features enabled)",
        },
        "results": all_results,
    }
    
    with open(OUTPUT_FILE, "w", encoding="utf-8") as f:
        json.dump(output, f, ensure_ascii=False, indent=1)
    
    print("=" * 60)
    print(f"DONE — {len(all_results)} results in {total_time:.1f}s")
    print(f"Saved to {OUTPUT_FILE}")
    print(f"Errors: {errors}")
    
    # Quick stats
    zero_results = sum(1 for r in all_results if r["num_results"] == 0)
    avg_results = sum(r["num_results"] for r in all_results) / max(len(all_results), 1)
    avg_time = sum(r["search_time_ms"] for r in all_results) / max(len(all_results), 1)
    
    print(f"\nQuick Stats:")
    print(f"  Zero-result queries: {zero_results} ({100*zero_results/max(len(all_results),1):.1f}%)")
    print(f"  Avg results per query: {avg_results:.1f}")
    print(f"  Avg search time: {avg_time:.0f}ms")
    
    # Cleanup progress file
    if os.path.exists(PROGRESS_FILE):
        os.remove(PROGRESS_FILE)

if __name__ == "__main__":
    main()
