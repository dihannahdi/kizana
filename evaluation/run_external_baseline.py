#!/usr/bin/env python3
"""
Kizana Search — External Baseline Comparison
=============================================
Compares Kizana's domain-specific cross-lingual retrieval against
standard industry baselines:

  1. Google Translate + BM25: Machine-translate queries to Arabic using
     Google Translate, then search with raw BM25 (no custom scoring).
     This represents the "naive" approach a researcher would use.

  2. Google Translate + Full Scoring: Machine-translated queries using
     Kizana's scoring adjustments (hierarchy, diversity, etc.)

  3. Kizana Full System: Our domain-specific query translation + scoring.

This answers the key Q1 reviewer question:
  "How does your system compare to simply using Google Translate + BM25?"

Usage:
    pip install deep-translator
    python run_external_baseline.py --base-url http://localhost:8080 --token YOUR_TOKEN
"""

import argparse
import json
import sys
import time
from pathlib import Path
from typing import Dict, List, Optional

import requests

from evaluate import load_gold_standard, judge_result
from metrics import compute_all_metrics, compute_batch_metrics

# Try importing deep_translator
try:
    from deep_translator import GoogleTranslator
    HAS_TRANSLATOR = True
except ImportError:
    HAS_TRANSLATOR = False
    print("WARNING: deep-translator not installed. Install with: pip install deep-translator")
    print("         Will use cached translations if available.\n")


# ── Cache file for translations (avoid repeated API calls) ──
TRANSLATION_CACHE_FILE = Path(__file__).parent / "translation_cache.json"


def load_translation_cache() -> Dict[str, str]:
    """Load cached query translations from disk."""
    if TRANSLATION_CACHE_FILE.exists():
        with open(TRANSLATION_CACHE_FILE, "r", encoding="utf-8") as f:
            return json.load(f)
    return {}


def save_translation_cache(cache: Dict[str, str]):
    """Save translation cache to disk."""
    with open(TRANSLATION_CACHE_FILE, "w", encoding="utf-8") as f:
        json.dump(cache, f, ensure_ascii=False, indent=2)


def translate_query_google(query: str, source_lang: str, cache: Dict[str, str]) -> str:
    """
    Translate a query to Arabic using Google Translate.
    Uses cache to avoid redundant API calls.
    """
    cache_key = f"{source_lang}:{query}"
    if cache_key in cache:
        return cache[cache_key]
    
    if not HAS_TRANSLATOR:
        return query  # Fallback: return original (will produce poor results)
    
    # Map our language codes to Google Translate codes
    lang_map = {
        "id": "id",       # Indonesian
        "en": "en",       # English
        "ar": "ar",       # Arabic (no translation needed)
        "mixed": "id",    # Mixed → treat as Indonesian
    }
    src = lang_map.get(source_lang, "auto")
    
    if src == "ar":
        # No translation needed for Arabic queries
        cache[cache_key] = query
        return query
    
    try:
        translator = GoogleTranslator(source=src, target="ar")
        translated = translator.translate(query)
        cache[cache_key] = translated
        return translated
    except Exception as e:
        print(f"    Translation error for '{query}': {e}")
        cache[cache_key] = query  # Cache failure to avoid retrying
        return query


def search_with_query(base_url: str, token: str, query: str, config: Dict,
                      max_results: int = 20, query_id: str = "ext_q") -> Dict:
    """Send a single query to the eval endpoint."""
    headers = {
        "Authorization": f"Bearer {token}",
        "Content-Type": "application/json",
    }
    payload = {
        "queries": [{"id": query_id, "text": query}],
        "config": config,
        "max_results": max_results,
    }
    
    resp = requests.post(
        f"{base_url}/api/eval/batch",
        json=payload,
        headers=headers,
        timeout=60,
    )
    resp.raise_for_status()
    data = resp.json()
    results = data.get("results", [{}])
    return results[0] if results else {}


def run_external_evaluation(
    base_url: str,
    token: str,
    gold_queries: List[Dict],
    translate_fn,
    config: Dict,
    config_name: str,
    max_results: int = 20,
    k_values: List[int] = [1, 3, 5, 10, 20],
    verbose: bool = False,
) -> Dict:
    """
    Run evaluation with externally-translated queries.
    
    For external baselines, we translate the query FIRST using Google Translate,
    then send the translated Arabic text directly (with query translation disabled).
    """
    all_relevance_scores = []
    per_query_results = []
    total_search_time = 0.0
    
    per_language = {}
    per_domain = {}
    
    # First, translate all queries
    translated_queries = []
    for i, gq in enumerate(gold_queries):
        query_text = gq["text"]
        lang = gq.get("lang", "unknown")
        domain = gq.get("domain", "unknown")
        relevant_keywords = gq.get("relevant_keywords", [])
        
        # Translate query if needed
        if translate_fn:
            translated_text = translate_fn(query_text, lang)
        else:
            translated_text = query_text
        
        translated_queries.append({
            "id": gq["id"],
            "text": translated_text,
            "lang": lang,
            "domain": domain,
            "relevant_keywords": relevant_keywords,
        })
    
    # Process in batches of 20
    BATCH_SIZE = 20
    for batch_start in range(0, len(translated_queries), BATCH_SIZE):
        batch = translated_queries[batch_start:batch_start + BATCH_SIZE]
        
        payload = {
            "queries": [{"id": tq["id"], "text": tq["text"]} for tq in batch],
            "config": config,
            "max_results": max_results,
        }
        
        headers = {
            "Authorization": f"Bearer {token}",
            "Content-Type": "application/json",
        }
        
        try:
            resp = requests.post(
                f"{base_url}/api/eval/batch",
                json=payload,
                headers=headers,
                timeout=120,
            )
            resp.raise_for_status()
            data = resp.json()
        except Exception as e:
            if verbose:
                print(f"    Batch error at {batch_start}: {e}")
            for tq in batch:
                all_relevance_scores.append([0] * max_results)
            continue
        
        total_search_time += data.get("total_time_ms", 0)
        
        for result in data.get("results", []):
            query_id = result["query_id"]
            tq = next((q for q in batch if q["id"] == query_id), None)
            if not tq:
                continue
            
            search_results = result.get("results", [])
            relevant_keywords = tq["relevant_keywords"]
            lang = tq["lang"]
            domain = tq["domain"]
            
            relevance_scores = []
            for sr in search_results[:max_results]:
                rel = judge_result(sr, relevant_keywords)
                relevance_scores.append(rel)
            
            while len(relevance_scores) < max_results:
                relevance_scores.append(0)
            
            all_relevance_scores.append(relevance_scores)
            
            if lang not in per_language:
                per_language[lang] = []
            per_language[lang].append(relevance_scores)
            
            if domain not in per_domain:
                per_domain[domain] = []
            per_domain[domain].append(relevance_scores)
    
    # Compute aggregate metrics
    aggregate = compute_batch_metrics(all_relevance_scores, k_values)
    
    # Compute per-language metrics
    per_language_metrics = {}
    for lang, scores in per_language.items():
        per_language_metrics[lang] = compute_batch_metrics(scores, k_values)
    
    # Compute per-domain metrics
    per_domain_metrics = {}
    for domain, scores in per_domain.items():
        per_domain_metrics[domain] = compute_batch_metrics(scores, k_values)
    
    avg_time = total_search_time / len(gold_queries) if gold_queries else 0
    
    return {
        "config_name": config_name,
        "config": config,
        "total_queries": len(gold_queries),
        "aggregate_metrics": aggregate,
        "per_language_metrics": per_language_metrics,
        "per_domain_metrics": per_domain_metrics,
        "avg_search_time_ms": avg_time,
        "all_relevance_scores": all_relevance_scores,
    }


def compute_paired_ttest(scores_a: List[List[int]], scores_b: List[List[int]],
                          k_values: List[int] = [5, 10]) -> Dict:
    """Compute paired t-test between two systems using AP scores."""
    try:
        import numpy as np
    except ImportError:
        return {}
    
    results = {}
    
    # Compute AP for each query
    from metrics import average_precision
    
    aps_a = [average_precision(s) for s in scores_a]
    aps_b = [average_precision(s) for s in scores_b]
    
    diffs = np.array(aps_a) - np.array(aps_b)
    n = len(diffs)
    
    if n < 2:
        return {}
    
    mean_diff = np.mean(diffs)
    std_diff = np.std(diffs, ddof=1)
    
    if std_diff == 0:
        return {"t_statistic": float('inf'), "p_value": 0.0, "cohens_d": float('inf')}
    
    t_stat = mean_diff / (std_diff / np.sqrt(n))
    
    # Two-tailed p-value (approximate using normal for large n)
    from math import erfc, sqrt
    p_value = erfc(abs(t_stat) / sqrt(2))
    
    # Cohen's d
    pooled_std = np.sqrt((np.std(aps_a, ddof=1)**2 + np.std(aps_b, ddof=1)**2) / 2)
    cohens_d = mean_diff / pooled_std if pooled_std > 0 else 0
    
    results = {
        "t_statistic": round(float(t_stat), 4),
        "p_value": round(float(p_value), 6),
        "cohens_d": round(float(cohens_d), 4),
        "mean_diff": round(float(mean_diff), 4),
        "n": n,
    }
    
    return results


def main():
    parser = argparse.ArgumentParser(
        description="Kizana External Baseline Comparison (vs Google Translate + BM25)"
    )
    parser.add_argument("--base-url", default="http://127.0.0.1:8080")
    parser.add_argument("--token", required=True, help="Admin JWT token")
    parser.add_argument("--max-results", type=int, default=20)
    parser.add_argument("--output-dir", default=str(Path(__file__).parent / "results"))
    parser.add_argument("--verbose", action="store_true")
    parser.add_argument("--skip-translate", action="store_true",
                        help="Skip Google Translate baseline (use cached only)")
    args = parser.parse_args()
    
    gold_queries = load_gold_standard()
    output_dir = Path(args.output_dir)
    output_dir.mkdir(exist_ok=True)
    
    # Load translation cache
    cache = load_translation_cache()
    
    print(f"\n{'='*70}")
    print(f"KIZANA SEARCH — EXTERNAL BASELINE COMPARISON")
    print(f"{'='*70}")
    print(f"  Queries:           {len(gold_queries)}")
    print(f"  URL:               {args.base_url}")
    print(f"  Google Translate:  {'available' if HAS_TRANSLATOR else 'NOT INSTALLED'}")
    print(f"  Cached translations: {len(cache)}")
    print()
    
    all_results = {}
    key_metrics = ["MAP", "MRR", "NDCG@5", "NDCG@10", "P@5", "P@10"]
    
    # ── Baseline 1: Google Translate + Raw BM25 ──
    print("  [1/4] Google Translate + BM25 ... ", end="", flush=True)
    
    def gt_translate(query, lang):
        return translate_query_google(query, lang, cache)
    
    start = time.time()
    gt_bm25 = run_external_evaluation(
        base_url=args.base_url,
        token=args.token,
        gold_queries=gold_queries,
        translate_fn=gt_translate,
        config={"disable_query_translation": True, "raw_bm25_only": True},
        config_name="gt_bm25",
        max_results=args.max_results,
        verbose=args.verbose,
    )
    elapsed = time.time() - start
    agg = gt_bm25["aggregate_metrics"]
    print(f"done ({elapsed:.1f}s) — MAP: {agg.get('MAP', 0):.3f}, MRR: {agg.get('MRR', 0):.3f}")
    all_results["gt_bm25"] = {
        "label": "Google Translate + BM25",
        "metrics": agg,
        "per_language": gt_bm25["per_language_metrics"],
        "per_domain": gt_bm25["per_domain_metrics"],
        "total_queries": gt_bm25["total_queries"],
        "avg_search_time_ms": gt_bm25["avg_search_time_ms"],
        "all_relevance_scores": gt_bm25["all_relevance_scores"],
    }
    
    # Save translation cache after each baseline
    save_translation_cache(cache)
    
    # ── Baseline 2: Google Translate + Kizana Scoring ──
    print("  [2/4] Google Translate + Kizana Scoring ... ", end="", flush=True)
    
    start = time.time()
    gt_scoring = run_external_evaluation(
        base_url=args.base_url,
        token=args.token,
        gold_queries=gold_queries,
        translate_fn=gt_translate,
        config={"disable_query_translation": True},  # Use GT translation but keep scoring
        config_name="gt_scoring",
        max_results=args.max_results,
        verbose=args.verbose,
    )
    elapsed = time.time() - start
    agg = gt_scoring["aggregate_metrics"]
    print(f"done ({elapsed:.1f}s) — MAP: {agg.get('MAP', 0):.3f}, MRR: {agg.get('MRR', 0):.3f}")
    all_results["gt_scoring"] = {
        "label": "Google Translate + Scoring",
        "metrics": agg,
        "per_language": gt_scoring["per_language_metrics"],
        "per_domain": gt_scoring["per_domain_metrics"],
        "total_queries": gt_scoring["total_queries"],
        "avg_search_time_ms": gt_scoring["avg_search_time_ms"],
        "all_relevance_scores": gt_scoring["all_relevance_scores"],
    }
    
    # ── Baseline 3: Kizana Full System (no translation → raw query) ──
    print("  [3/4] No Translation (raw query → Arabic index) ... ", end="", flush=True)
    
    start = time.time()
    no_trans = run_external_evaluation(
        base_url=args.base_url,
        token=args.token,
        gold_queries=gold_queries,
        translate_fn=None,  # No translation at all
        config={"disable_query_translation": True},
        config_name="no_translation",
        max_results=args.max_results,
        verbose=args.verbose,
    )
    elapsed = time.time() - start
    agg = no_trans["aggregate_metrics"]
    print(f"done ({elapsed:.1f}s) — MAP: {agg.get('MAP', 0):.3f}, MRR: {agg.get('MRR', 0):.3f}")
    all_results["no_translation"] = {
        "label": "No Translation",
        "metrics": agg,
        "per_language": no_trans["per_language_metrics"],
        "per_domain": no_trans["per_domain_metrics"],
        "total_queries": no_trans["total_queries"],
        "avg_search_time_ms": no_trans["avg_search_time_ms"],
        "all_relevance_scores": no_trans["all_relevance_scores"],
    }
    
    # ── System 4: Kizana Full System ──
    print("  [4/4] Kizana Full System ... ", end="", flush=True)
    
    start = time.time()
    kizana_full = run_external_evaluation(
        base_url=args.base_url,
        token=args.token,
        gold_queries=gold_queries,
        translate_fn=None,  # Kizana handles translation internally
        config={},  # Full system, all features enabled
        config_name="kizana_full",
        max_results=args.max_results,
        verbose=args.verbose,
    )
    elapsed = time.time() - start
    agg = kizana_full["aggregate_metrics"]
    print(f"done ({elapsed:.1f}s) — MAP: {agg.get('MAP', 0):.3f}, MRR: {agg.get('MRR', 0):.3f}")
    all_results["kizana_full"] = {
        "label": "Kizana (Full)",
        "metrics": agg,
        "per_language": kizana_full["per_language_metrics"],
        "per_domain": kizana_full["per_domain_metrics"],
        "total_queries": kizana_full["total_queries"],
        "avg_search_time_ms": kizana_full["avg_search_time_ms"],
        "all_relevance_scores": kizana_full["all_relevance_scores"],
    }
    
    # ════════════════════════════════════════════════
    # RESULTS
    # ════════════════════════════════════════════════
    
    print(f"\n{'='*80}")
    print("EXTERNAL BASELINE COMPARISON — Aggregate Results")
    print(f"{'='*80}")
    
    # Order: No Translation → GT + BM25 → GT + Scoring → Kizana Full
    display_order = ["no_translation", "gt_bm25", "gt_scoring", "kizana_full"]
    
    try:
        from tabulate import tabulate
        headers = ["System", "MAP", "MRR", "NDCG@5", "NDCG@10", "P@5", "P@10", "Avg ms"]
        rows = []
        for name in display_order:
            if name not in all_results:
                continue
            r = all_results[name]
            row = [r["label"]]
            for m in key_metrics:
                row.append(r["metrics"].get(m, 0.0))
            row.append(r["avg_search_time_ms"])
            rows.append(row)
        print(tabulate(rows, headers=headers, floatfmt=".4f"))
    except ImportError:
        header = f"{'System':30s} | " + " | ".join(f"{m:>8s}" for m in key_metrics)
        print(header)
        print("-" * len(header))
        for name in display_order:
            if name not in all_results:
                continue
            r = all_results[name]
            values = " | ".join(f"{r['metrics'].get(m, 0.0):8.4f}" for m in key_metrics)
            print(f"{r['label']:30s} | {values}")
    
    # ── Improvement over Google Translate ──
    if "gt_bm25" in all_results and "kizana_full" in all_results:
        print(f"\n{'='*80}")
        print("IMPROVEMENT: Kizana vs. Google Translate + BM25")
        print(f"{'='*80}")
        
        gt = all_results["gt_bm25"]["metrics"]
        kz = all_results["kizana_full"]["metrics"]
        
        for m in key_metrics:
            gt_val = gt.get(m, 0.0)
            kz_val = kz.get(m, 0.0)
            abs_diff = kz_val - gt_val
            rel_pct = (abs_diff / gt_val * 100) if gt_val > 0 else 0
            print(f"  {m:>10s}: {gt_val:.4f} → {kz_val:.4f}  ({abs_diff:+.4f}, {rel_pct:+.1f}%)")
        
        # Statistical significance
        print(f"\n  Statistical Significance (paired t-test on AP scores):")
        stats = compute_paired_ttest(
            all_results["kizana_full"]["all_relevance_scores"],
            all_results["gt_bm25"]["all_relevance_scores"],
        )
        if stats:
            sig = "***" if stats["p_value"] < 0.001 else "**" if stats["p_value"] < 0.01 else "*" if stats["p_value"] < 0.05 else "n.s."
            print(f"    t({stats['n']-1}) = {stats['t_statistic']:.3f}, p = {stats['p_value']:.6f} {sig}")
            print(f"    Cohen's d = {stats['cohens_d']:.3f} ({'large' if abs(stats['cohens_d']) > 0.8 else 'medium' if abs(stats['cohens_d']) > 0.5 else 'small'})")
    
    # ── Per-language comparison: Kizana vs GT+BM25 ──
    if "gt_bm25" in all_results and "kizana_full" in all_results:
        print(f"\n{'='*80}")
        print("PER-LANGUAGE: Kizana Full vs. Google Translate + BM25")
        print(f"{'='*80}")
        
        for lang in sorted(set(
            list(all_results["kizana_full"].get("per_language", {}).keys()) +
            list(all_results["gt_bm25"].get("per_language", {}).keys())
        )):
            kz_lang = all_results["kizana_full"]["per_language"].get(lang, {})
            gt_lang = all_results["gt_bm25"]["per_language"].get(lang, {})
            
            print(f"\n  Language: {lang}")
            print(f"    {'Metric':>10s} | {'Kizana':>8s} | {'GT+BM25':>8s} | {'Δ':>8s} | {'%Δ':>8s}")
            print(f"    {'-'*52}")
            for m in key_metrics:
                kz_val = kz_lang.get(m, 0.0)
                gt_val = gt_lang.get(m, 0.0)
                delta = kz_val - gt_val
                pct = (delta / gt_val * 100) if gt_val > 0 else 0
                print(f"    {m:>10s} | {kz_val:8.4f} | {gt_val:8.4f} | {delta:+8.4f} | {pct:+7.1f}%")
    
    # ── Save results (without raw scores for JSON) ──
    save_results = {}
    for name, r in all_results.items():
        save_results[name] = {k: v for k, v in r.items() if k != "all_relevance_scores"}
    
    timestamp = time.strftime("%Y%m%d_%H%M%S")
    output_file = output_dir / f"external_baseline_{timestamp}.json"
    with open(output_file, "w", encoding="utf-8") as f:
        json.dump(save_results, f, ensure_ascii=False, indent=2)
    print(f"\nResults saved to {output_file}")
    
    # ── Generate LaTeX table ──
    latex_file = output_dir / f"external_baseline_{timestamp}.tex"
    with open(latex_file, "w", encoding="utf-8") as f:
        f.write("% Auto-generated external baseline comparison table\n")
        f.write("\\begin{table}[t]\n")
        f.write("\\centering\n")
        f.write("\\caption{Cross-lingual retrieval comparison: Kizana vs.\\ external baselines. ")
        f.write("Statistical significance determined by paired $t$-test on per-query AP scores.}\n")
        f.write("\\label{tab:external-baseline}\n")
        f.write("\\begin{tabular}{l" + "c" * len(key_metrics) + "}\n")
        f.write("\\toprule\n")
        f.write("System & " + " & ".join(key_metrics) + " \\\\\n")
        f.write("\\midrule\n")
        
        for name in display_order:
            if name not in all_results:
                continue
            r = all_results[name]
            label = r["label"].replace("_", "\\_")
            values = " & ".join(f"{r['metrics'].get(m, 0.0):.3f}" for m in key_metrics)
            if name == "kizana_full":
                bold_vals = " & ".join("\\textbf{" + f"{r['metrics'].get(m, 0.0):.3f}" + "}" for m in key_metrics)
                f.write("\\textbf{" + label + "} & " + bold_vals + " \\\\\n")
            else:
                f.write(f"{label} & {values} \\\\\n")
        
        f.write("\\bottomrule\n")
        f.write("\\end{tabular}\n")
        f.write("\\end{table}\n")
        
        # Per-language table
        f.write("\n% Per-language comparison: Kizana vs Google Translate + BM25\n")
        f.write("\\begin{table}[t]\n")
        f.write("\\centering\n")
        f.write("\\caption{Per-language MAP comparison across systems.}\n")
        f.write("\\label{tab:external-per-lang}\n")
        
        languages = sorted(all_results.get("kizana_full", {}).get("per_language", {}).keys())
        f.write("\\begin{tabular}{l" + "c" * len(languages) + "c}\n")
        f.write("\\toprule\n")
        f.write("System & " + " & ".join(lang.capitalize() for lang in languages) + " & Overall \\\\\n")
        f.write("\\midrule\n")
        
        for name in display_order:
            if name not in all_results:
                continue
            r = all_results[name]
            label = r["label"].replace("_", "\\_")
            vals = []
            for lang in languages:
                lang_data = r.get("per_language", {}).get(lang, {})
                vals.append(f"{lang_data.get('MAP', 0.0):.3f}")
            vals.append(f"{r['metrics'].get('MAP', 0.0):.3f}")
            
            if name == "kizana_full":
                f.write(f"\\textbf{{{label}}} & " + " & ".join(f"\\textbf{{{v}}}" for v in vals) + " \\\\\n")
            else:
                f.write(f"{label} & " + " & ".join(vals) + " \\\\\n")
        
        f.write("\\bottomrule\n")
        f.write("\\end{tabular}\n")
        f.write("\\end{table}\n")
    
    print(f"LaTeX tables saved to {latex_file}")
    
    # ── Print Google Translate output samples ──
    if cache:
        print(f"\n{'='*80}")
        print("SAMPLE GOOGLE TRANSLATIONS (for paper appendix)")
        print(f"{'='*80}")
        
        samples = list(cache.items())[:10]
        for key, translated in samples:
            lang, query = key.split(":", 1) if ":" in key else ("?", key)
            print(f"  [{lang}] {query}")
            print(f"   → {translated}")
            print()


if __name__ == "__main__":
    main()
