#!/usr/bin/env python3
"""
Deep statistical analysis of 10K eval results.
Produces comprehensive report on search quality.
"""
import json
import sys
from collections import Counter, defaultdict
import math

def load_results(path="eval_results_10k.json"):
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)

def load_queries(path="queries_10k.json"):
    with open(path, "r", encoding="utf-8") as f:
        return json.load(f)

def main():
    input_file = sys.argv[1] if len(sys.argv) > 1 else "eval_results_10k.json"
    data = load_results(input_file)
    queries_meta = {q["id"]: q for q in load_queries()}
    results = data["results"]
    meta = data["metadata"]
    
    print("=" * 70)
    print("KIZANA SEARCH — 10K EVALUATION DEEP ANALYSIS")
    print("=" * 70)
    print(f"Total queries: {meta['total_queries']}")
    print(f"Results collected: {meta['total_results_collected']}")
    print(f"Total time: {meta['total_time_seconds']}s")
    print(f"Avg time/query: {meta['avg_time_per_query_ms']}ms")
    print()
    
    # ═══════════════════════════════════════════
    # 1. ZERO-RESULT ANALYSIS
    # ═══════════════════════════════════════════
    print("=" * 70)
    print("1. ZERO-RESULT QUERIES (Translation Failures)")
    print("=" * 70)
    
    zero = [r for r in results if r["num_results"] == 0]
    nonzero = [r for r in results if r["num_results"] > 0]
    
    print(f"Total zero-result: {len(zero)} / {len(results)} ({100*len(zero)/max(len(results),1):.2f}%)")
    
    # Zero results by category
    zero_by_cat = Counter()
    total_by_cat = Counter()
    for r in results:
        qm = queries_meta.get(r["query_id"], {})
        cat = qm.get("category", "unknown")
        total_by_cat[cat] += 1
        if r["num_results"] == 0:
            zero_by_cat[cat] += 1
    
    print("\nZero-result rate by category (worst first):")
    cat_rates = []
    for cat in total_by_cat:
        rate = zero_by_cat[cat] / total_by_cat[cat] * 100
        cat_rates.append((cat, zero_by_cat[cat], total_by_cat[cat], rate))
    cat_rates.sort(key=lambda x: -x[3])
    for cat, z, t, rate in cat_rates[:30]:
        if z > 0:
            print(f"  {cat:25s} {z:4d}/{t:4d} = {rate:5.1f}%")
    
    # Zero by language
    zero_by_lang = Counter()
    total_by_lang = Counter()
    for r in results:
        lang = r.get("detected_language", "unknown")
        total_by_lang[lang] += 1
        if r["num_results"] == 0:
            zero_by_lang[lang] += 1
    
    print("\nZero-result by detected language:")
    for lang in sorted(total_by_lang.keys()):
        z = zero_by_lang[lang]
        t = total_by_lang[lang]
        print(f"  {lang:10s} {z:4d}/{t:4d} = {100*z/t:.1f}%")
    
    # Sample zero-result queries
    print("\nSample zero-result queries (first 30):")
    for r in zero[:30]:
        terms = ", ".join(r.get("translated_terms", [])[:5])
        print(f"  [{r['query_id']}] {r['query_text'][:60]:60s} -> terms: {terms}")
    
    # ═══════════════════════════════════════════
    # 2. SCORE DISTRIBUTION
    # ═══════════════════════════════════════════
    print("\n" + "=" * 70)
    print("2. SCORE DISTRIBUTION")
    print("=" * 70)
    
    top1_scores = [r["top_scores"][0] for r in results if r["top_scores"]]
    if top1_scores:
        avg_top1 = sum(top1_scores) / len(top1_scores)
        median_top1 = sorted(top1_scores)[len(top1_scores)//2]
        min_top1 = min(top1_scores)
        max_top1 = max(top1_scores)
        std_top1 = math.sqrt(sum((s - avg_top1)**2 for s in top1_scores) / len(top1_scores))
        
        print(f"Top-1 score statistics (n={len(top1_scores)}):")
        print(f"  Mean:   {avg_top1:.2f}")
        print(f"  Median: {median_top1:.2f}")
        print(f"  Std:    {std_top1:.2f}")
        print(f"  Min:    {min_top1:.2f}")
        print(f"  Max:    {max_top1:.2f}")
        
        # Score bands
        bands = [(0, 10), (10, 20), (20, 30), (30, 40), (40, 50),
                 (50, 60), (60, 70), (70, 80), (80, 90), (90, 101)]
        print("\nScore distribution (top-1):")
        for lo, hi in bands:
            count = sum(1 for s in top1_scores if lo <= s < hi)
            bar = "#" * (count * 50 // max(len(top1_scores), 1))
            print(f"  {lo:3d}-{hi-1:3d}: {count:5d} ({100*count/len(top1_scores):5.1f}%) {bar}")
    
    # Score by category
    print("\nAvg top-1 score by category (lowest first):")
    scores_by_cat = defaultdict(list)
    for r in results:
        if r["top_scores"]:
            qm = queries_meta.get(r["query_id"], {})
            cat = qm.get("category", "unknown")
            scores_by_cat[cat].append(r["top_scores"][0])
    
    cat_avgs = [(cat, sum(s)/len(s), len(s)) for cat, s in scores_by_cat.items() if s]
    cat_avgs.sort(key=lambda x: x[1])
    for cat, avg, n in cat_avgs[:30]:
        print(f"  {cat:25s} avg={avg:5.1f} (n={n})")
    
    # ═══════════════════════════════════════════
    # 3. SEARCH TIME ANALYSIS
    # ═══════════════════════════════════════════
    print("\n" + "=" * 70)
    print("3. SEARCH TIME ANALYSIS")
    print("=" * 70)
    
    times = [r["search_time_ms"] for r in results]
    if times:
        avg_t = sum(times) / len(times)
        med_t = sorted(times)[len(times)//2]
        p95_t = sorted(times)[int(len(times)*0.95)]
        p99_t = sorted(times)[int(len(times)*0.99)]
        max_t = max(times)
        
        print(f"Search latency (n={len(times)}):")
        print(f"  Mean:   {avg_t:.0f}ms")
        print(f"  Median: {med_t:.0f}ms")
        print(f"  P95:    {p95_t:.0f}ms")
        print(f"  P99:    {p99_t:.0f}ms")
        print(f"  Max:    {max_t:.0f}ms")
        
        # Slow queries
        slow = [(r["query_id"], r["query_text"], r["search_time_ms"]) 
                for r in results if r["search_time_ms"] > p95_t]
        slow.sort(key=lambda x: -x[2])
        print(f"\nSlowest queries (>{p95_t}ms):")
        for qid, text, ms in slow[:20]:
            print(f"  [{qid}] {text[:55]:55s} {ms}ms")
    
    # ═══════════════════════════════════════════
    # 4. LANGUAGE DETECTION
    # ═══════════════════════════════════════════
    print("\n" + "=" * 70)
    print("4. LANGUAGE DETECTION ACCURACY")
    print("=" * 70)
    
    lang_matrix = defaultdict(lambda: Counter())
    for r in results:
        qm = queries_meta.get(r["query_id"], {})
        expected = qm.get("lang", "id")
        detected = r.get("detected_language", "unknown")
        lang_matrix[expected][detected] += 1
    
    print("Expected -> Detected confusion matrix:")
    all_langs = sorted(set(l for c in lang_matrix.values() for l in c.keys()))
    header = f"  {'Expected':10s} " + " ".join(f"{l:8s}" for l in all_langs) + "  Total"
    print(header)
    for exp in sorted(lang_matrix.keys()):
        row_total = sum(lang_matrix[exp].values())
        counts = " ".join(f"{lang_matrix[exp].get(l, 0):8d}" for l in all_langs)
        correct = lang_matrix[exp].get(exp, 0)
        acc = 100 * correct / max(row_total, 1)
        print(f"  {exp:10s} {counts}  {row_total:5d} ({acc:.1f}% correct)")
    
    # ═══════════════════════════════════════════
    # 5. DOMAIN DETECTION
    # ═══════════════════════════════════════════
    print("\n" + "=" * 70)
    print("5. DOMAIN DETECTION DISTRIBUTION")
    print("=" * 70)
    
    domain_counts = Counter()
    for r in results:
        domain_counts[r.get("detected_domain", "unknown")] += 1
    
    print("Detected domains:")
    for domain, count in domain_counts.most_common():
        pct = 100 * count / len(results)
        print(f"  {domain:30s} {count:5d} ({pct:5.1f}%)")
    
    # ═══════════════════════════════════════════
    # 6. TRANSLATION QUALITY
    # ═══════════════════════════════════════════
    print("\n" + "=" * 70)
    print("6. QUERY TRANSLATION QUALITY")
    print("=" * 70)
    
    term_counts = [len(r.get("translated_terms", [])) for r in results]
    if term_counts:
        avg_terms = sum(term_counts) / len(term_counts)
        zero_terms = sum(1 for t in term_counts if t == 0)
        one_term = sum(1 for t in term_counts if t == 1)
        
        print(f"Translated term statistics:")
        print(f"  Avg terms per query: {avg_terms:.1f}")
        print(f"  Zero terms (no translation): {zero_terms} ({100*zero_terms/len(term_counts):.1f}%)")
        print(f"  Only 1 term: {one_term} ({100*one_term/len(term_counts):.1f}%)")
        
        # Term count distribution
        print("\nTerm count distribution:")
        for n in range(0, 15):
            count = sum(1 for t in term_counts if t == n)
            if count > 0:
                bar = "#" * (count * 40 // max(len(term_counts), 1))
                print(f"  {n:2d} terms: {count:5d} ({100*count/len(term_counts):5.1f}%) {bar}")
    
    # Queries with few terms but high score (good translation efficiency)
    # vs queries with many terms but low score (bad translation)
    print("\nLow term count + zero results (translation failures):")
    trans_fails = [(r["query_id"], r["query_text"], len(r.get("translated_terms", [])), 
                    r.get("translated_terms", []))
                   for r in results 
                   if r["num_results"] == 0 and len(r.get("translated_terms", [])) <= 2]
    for qid, text, tc, terms in trans_fails[:20]:
        print(f"  [{qid}] {text[:50]:50s} -> {tc} terms: {terms}")
    
    # ═══════════════════════════════════════════
    # 7. RESULT DIVERSITY
    # ═══════════════════════════════════════════
    print("\n" + "=" * 70)
    print("7. BOOK DIVERSITY IN RESULTS")
    print("=" * 70)
    
    book_freq = Counter()
    for r in results:
        for book in r.get("top_books", []):
            if book:
                book_freq[book] += 1
    
    print(f"Unique books appearing in results: {len(book_freq)}")
    print("\nMost frequently appearing books (top 30):")
    for book, count in book_freq.most_common(30):
        print(f"  {count:5d}x {book[:60]}")
    
    # ═══════════════════════════════════════════
    # 8. OVERALL QUALITY METRICS
    # ═══════════════════════════════════════════
    print("\n" + "=" * 70)
    print("8. OVERALL QUALITY METRICS")
    print("=" * 70)
    
    total_q = len(results)
    has_results = sum(1 for r in results if r["num_results"] > 0)
    high_score = sum(1 for r in results if r["top_scores"] and r["top_scores"][0] >= 70)
    mid_score = sum(1 for r in results if r["top_scores"] and 40 <= r["top_scores"][0] < 70)
    low_score = sum(1 for r in results if r["top_scores"] and r["top_scores"][0] < 40)
    
    print(f"Result rate:      {has_results}/{total_q} = {100*has_results/total_q:.1f}%")
    print(f"High score (>=70): {high_score}/{total_q} = {100*high_score/total_q:.1f}%")
    print(f"Mid score (40-69): {mid_score}/{total_q} = {100*mid_score/total_q:.1f}%")
    print(f"Low score (<40):   {low_score}/{total_q} = {100*low_score/total_q:.1f}%")
    print(f"Zero results:      {total_q-has_results}/{total_q} = {100*(total_q-has_results)/total_q:.1f}%")
    
    if times:
        under_500 = sum(1 for t in times if t < 500)
        under_1000 = sum(1 for t in times if t < 1000)
        over_2000 = sum(1 for t in times if t > 2000)
        print(f"\nLatency:")
        print(f"  <500ms:  {under_500}/{total_q} = {100*under_500/total_q:.1f}%")
        print(f"  <1000ms: {under_1000}/{total_q} = {100*under_1000/total_q:.1f}%")
        print(f"  >2000ms: {over_2000}/{total_q} = {100*over_2000/total_q:.1f}%")
    
    # ═══════════════════════════════════════════
    # 9. CATEGORY DEEP DIVE
    # ═══════════════════════════════════════════
    print("\n" + "=" * 70)
    print("9. CATEGORY PERFORMANCE SUMMARY")
    print("=" * 70)
    
    cat_stats = defaultdict(lambda: {"count": 0, "has_results": 0, "scores": [], "times": []})
    for r in results:
        qm = queries_meta.get(r["query_id"], {})
        cat = qm.get("category", "unknown")
        cat_stats[cat]["count"] += 1
        if r["num_results"] > 0:
            cat_stats[cat]["has_results"] += 1
        if r["top_scores"]:
            cat_stats[cat]["scores"].append(r["top_scores"][0])
        cat_stats[cat]["times"].append(r["search_time_ms"])
    
    print(f"{'Category':25s} {'Count':>5s} {'Results%':>8s} {'AvgScore':>8s} {'AvgMs':>6s}")
    print("-" * 55)
    for cat in sorted(cat_stats.keys()):
        s = cat_stats[cat]
        res_pct = 100 * s["has_results"] / max(s["count"], 1)
        avg_s = sum(s["scores"]) / max(len(s["scores"]), 1) if s["scores"] else 0
        avg_ms = sum(s["times"]) / max(len(s["times"]), 1)
        print(f"  {cat:25s} {s['count']:5d} {res_pct:7.1f}% {avg_s:8.1f} {avg_ms:6.0f}")
    
    # ═══════════════════════════════════════════
    # 10. ACTIONABLE RECOMMENDATIONS
    # ═══════════════════════════════════════════
    print("\n" + "=" * 70)
    print("10. ACTIONABLE RECOMMENDATIONS")
    print("=" * 70)
    
    # Find worst categories
    worst_cats = [(cat, s) for cat, s in cat_stats.items() 
                  if s["count"] >= 10 and (not s["scores"] or sum(s["scores"])/len(s["scores"]) < 40)]
    if worst_cats:
        print("\nCategories needing immediate attention (avg score < 40, n>=10):")
        for cat, s in sorted(worst_cats, key=lambda x: sum(x[1]["scores"])/max(len(x[1]["scores"]),1)):
            avg = sum(s["scores"])/max(len(s["scores"]),1)
            print(f"  {cat}: avg score = {avg:.1f}, result rate = {100*s['has_results']/s['count']:.0f}%")
    
    # Find most common untranslated patterns
    no_trans = Counter()
    for r in results:
        if r["num_results"] == 0:
            words = r["query_text"].lower().split()
            for w in words:
                if len(w) > 3:
                    no_trans[w] += 1
    
    if no_trans:
        print("\nMost common words in zero-result queries (add to term dictionary):")
        for word, count in no_trans.most_common(30):
            print(f"  {word:20s} appears {count}x in zero-result queries")
    
    print("\n" + "=" * 70)
    print("ANALYSIS COMPLETE")
    print("=" * 70)

if __name__ == "__main__":
    main()
