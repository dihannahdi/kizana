#!/usr/bin/env python3
"""
Compare two eval result sets (e.g., v14 baseline vs v15 improved).
Produces side-by-side analysis showing what improved, regressed, or stayed same.
"""
import json
import sys
import os
from collections import Counter, defaultdict

os.chdir(os.path.dirname(os.path.abspath(__file__)))

def load_results(path):
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)

def load_queries(path="queries_10k.json"):
    with open(path, "r", encoding="utf-8") as f:
        return {q["id"]: q for q in json.load(f)}

def main():
    if len(sys.argv) < 3:
        print("Usage: compare_eval.py <baseline.json> <improved.json>")
        print("Example: compare_eval.py eval_results_10k.json eval_results_v15.json")
        sys.exit(1)
    
    baseline_path = sys.argv[1]
    improved_path = sys.argv[2]
    
    base = load_results(baseline_path)
    imp = load_results(improved_path)
    queries_meta = load_queries()
    
    base_results = {r["query_id"]: r for r in base["results"]}
    imp_results = {r["query_id"]: r for r in imp["results"]}
    
    common_ids = set(base_results.keys()) & set(imp_results.keys())
    
    print("=" * 80)
    print("KIZANA SEARCH — EVALUATION COMPARISON REPORT")
    print("=" * 80)
    print(f"Baseline: {baseline_path} ({len(base_results)} queries)")
    print(f"Improved: {improved_path} ({len(imp_results)} queries)")
    print(f"Common queries: {len(common_ids)}")
    print()
    
    # ═══ 1. HIGH-LEVEL METRICS ═══
    print("=" * 80)
    print("1. HIGH-LEVEL METRICS COMPARISON")
    print("=" * 80)
    
    base_zero = sum(1 for qid in common_ids if base_results[qid]["num_results"] == 0)
    imp_zero = sum(1 for qid in common_ids if imp_results[qid]["num_results"] == 0)
    
    base_scores = [base_results[qid]["top_scores"][0] for qid in common_ids if base_results[qid]["top_scores"]]
    imp_scores = [imp_results[qid]["top_scores"][0] for qid in common_ids if imp_results[qid]["top_scores"]]
    
    base_times = [base_results[qid]["search_time_ms"] for qid in common_ids]
    imp_times = [imp_results[qid]["search_time_ms"] for qid in common_ids]
    
    def safe_avg(lst):
        return sum(lst) / max(len(lst), 1)
    
    print(f"{'Metric':35s} {'Baseline':>10s} {'Improved':>10s} {'Delta':>10s}")
    print("-" * 70)
    print(f"{'Zero-result queries':35s} {base_zero:10d} {imp_zero:10d} {imp_zero - base_zero:+10d}")
    print(f"{'Zero-result rate':35s} {100*base_zero/len(common_ids):9.1f}% {100*imp_zero/len(common_ids):9.1f}% {100*(imp_zero-base_zero)/len(common_ids):+9.1f}%")
    print(f"{'Avg top-1 score':35s} {safe_avg(base_scores):10.2f} {safe_avg(imp_scores):10.2f} {safe_avg(imp_scores)-safe_avg(base_scores):+10.2f}")
    print(f"{'Avg search time (ms)':35s} {safe_avg(base_times):10.0f} {safe_avg(imp_times):10.0f} {safe_avg(imp_times)-safe_avg(base_times):+10.0f}")
    
    # Score bands
    bands = [(0, 40), (40, 70), (70, 90), (90, 101)]
    band_labels = ["Low (<40)", "Medium (40-69)", "High (70-89)", "Top (90-100)"]
    print(f"\nScore distribution (top-1):")
    for (lo, hi), label in zip(bands, band_labels):
        b_count = sum(1 for s in base_scores if lo <= s < hi)
        i_count = sum(1 for s in imp_scores if lo <= s < hi)
        delta = i_count - b_count
        print(f"  {label:20s} {b_count:6d} → {i_count:6d} ({delta:+6d})")
    
    # ═══ 2. QUERY-LEVEL CHANGES ═══
    print("\n" + "=" * 80)
    print("2. QUERY-LEVEL CHANGES")
    print("=" * 80)
    
    improved_queries = []
    regressed_queries = []
    fixed_zero = []  # was zero, now has results
    new_zero = []    # had results, now zero
    
    for qid in sorted(common_ids):
        b = base_results[qid]
        i = imp_results[qid]
        
        b_score = b["top_scores"][0] if b["top_scores"] else 0
        i_score = i["top_scores"][0] if i["top_scores"] else 0
        delta = i_score - b_score
        
        if b["num_results"] == 0 and i["num_results"] > 0:
            fixed_zero.append((qid, i_score))
        elif b["num_results"] > 0 and i["num_results"] == 0:
            new_zero.append((qid, b_score))
        elif delta > 5:
            improved_queries.append((qid, b_score, i_score, delta))
        elif delta < -5:
            regressed_queries.append((qid, b_score, i_score, delta))
    
    print(f"\nFixed zero-result queries: {len(fixed_zero)}")
    for qid, score in sorted(fixed_zero, key=lambda x: -x[1])[:20]:
        qm = queries_meta.get(qid, {})
        cat = qm.get("category", "?")
        text = base_results[qid]["query_text"]
        print(f"  [{qid}] ({cat}) {text[:55]:55s} → score {score:.1f}")
    
    print(f"\nNew zero-result queries (REGRESSIONS): {len(new_zero)}")
    for qid, score in sorted(new_zero, key=lambda x: -x[1])[:20]:
        qm = queries_meta.get(qid, {})
        cat = qm.get("category", "?")
        text = base_results[qid]["query_text"]
        print(f"  [{qid}] ({cat}) {text[:55]:55s} was {score:.1f}")
    
    print(f"\nMost improved queries (delta > +5): {len(improved_queries)}")
    improved_queries.sort(key=lambda x: -x[3])
    for qid, b_s, i_s, d in improved_queries[:20]:
        text = base_results[qid]["query_text"]
        print(f"  [{qid}] {text[:50]:50s} {b_s:.0f} → {i_s:.0f} ({d:+.0f})")
    
    print(f"\nMost regressed queries (delta < -5): {len(regressed_queries)}")
    regressed_queries.sort(key=lambda x: x[3])
    for qid, b_s, i_s, d in regressed_queries[:20]:
        text = base_results[qid]["query_text"]
        print(f"  [{qid}] {text[:50]:50s} {b_s:.0f} → {i_s:.0f} ({d:+.0f})")
    
    # ═══ 3. CATEGORY COMPARISON ═══
    print("\n" + "=" * 80)
    print("3. CATEGORY-LEVEL COMPARISON")
    print("=" * 80)
    
    cat_base = defaultdict(list)
    cat_imp = defaultdict(list)
    cat_base_zero = Counter()
    cat_imp_zero = Counter()
    cat_total = Counter()
    
    for qid in common_ids:
        qm = queries_meta.get(qid, {})
        cat = qm.get("category", "unknown")
        cat_total[cat] += 1
        
        b = base_results[qid]
        i = imp_results[qid]
        
        if b["top_scores"]:
            cat_base[cat].append(b["top_scores"][0])
        if b["num_results"] == 0:
            cat_base_zero[cat] += 1
            
        if i["top_scores"]:
            cat_imp[cat].append(i["top_scores"][0])
        if i["num_results"] == 0:
            cat_imp_zero[cat] += 1
    
    print(f"{'Category':25s} {'N':>4s} {'Base Avg':>8s} {'Imp Avg':>8s} {'Delta':>7s} {'Base 0%':>7s} {'Imp 0%':>7s}")
    print("-" * 75)
    
    cat_deltas = []
    for cat in sorted(cat_total.keys()):
        n = cat_total[cat]
        b_avg = safe_avg(cat_base[cat]) if cat_base[cat] else 0
        i_avg = safe_avg(cat_imp[cat]) if cat_imp[cat] else 0
        delta = i_avg - b_avg
        b_zero_pct = 100 * cat_base_zero[cat] / n
        i_zero_pct = 100 * cat_imp_zero[cat] / n
        cat_deltas.append((cat, n, b_avg, i_avg, delta, b_zero_pct, i_zero_pct))
    
    # Sort by delta (most improved first)
    cat_deltas.sort(key=lambda x: -x[4])
    for cat, n, b_avg, i_avg, delta, b_zero, i_zero in cat_deltas:
        marker = "⬆" if delta > 2 else ("⬇" if delta < -2 else " ")
        print(f"  {cat:25s} {n:4d} {b_avg:8.1f} {i_avg:8.1f} {delta:+6.1f}{marker} {b_zero:6.1f}% {i_zero:6.1f}%")
    
    # ═══ 4. SUMMARY ═══
    print("\n" + "=" * 80)
    print("4. EXECUTIVE SUMMARY")
    print("=" * 80)
    
    total_improved = len(improved_queries) + len(fixed_zero)
    total_regressed = len(regressed_queries) + len(new_zero)
    unchanged = len(common_ids) - total_improved - total_regressed
    
    print(f"Total improved:   {total_improved:5d} ({100*total_improved/len(common_ids):.1f}%)")
    print(f"Total regressed:  {total_regressed:5d} ({100*total_regressed/len(common_ids):.1f}%)")
    print(f"Unchanged (±5):   {unchanged:5d} ({100*unchanged/len(common_ids):.1f}%)")
    print(f"Net improvement:  {total_improved - total_regressed:+5d}")
    print()
    
    score_delta = safe_avg(imp_scores) - safe_avg(base_scores)
    zero_delta = imp_zero - base_zero
    print(f"Score delta:      {score_delta:+.2f} (avg top-1)")
    print(f"Zero-result delta: {zero_delta:+d}")
    
    if score_delta > 0 and zero_delta <= 0:
        print("\n✅ OVERALL: IMPROVEMENT — scores up, zero-results same or down")
    elif score_delta > 0:
        print("\n⚠️  OVERALL: MIXED — scores up but zero-results also up")
    elif score_delta < 0:
        print("\n❌ OVERALL: REGRESSION — scores decreased")
    else:
        print("\n  OVERALL: NO SIGNIFICANT CHANGE")
    
    print("\n" + "=" * 80)
    print("COMPARISON COMPLETE")
    print("=" * 80)

if __name__ == "__main__":
    main()
