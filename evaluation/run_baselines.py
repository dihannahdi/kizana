#!/usr/bin/env python3
"""
Kizana Search — Baseline Comparison Runner
============================================
Compares the full Kizana system against baseline configurations,
organized by research question. This produces the comparison tables
required for journal submission.

Research Questions & Baselines:
  RQ1: Does cross-lingual query translation improve retrieval?
       → Full System vs. No Translation (raw Indonesian/English queries → Arabic index)
  
  RQ2: Does domain-aware scoring improve relevance?
       → Full System vs. Raw BM25 (no custom scoring at all)
  
  RQ3: How does each scoring component contribute?
       → Ablation of individual components (see run_ablation.py)
  
  RQ4: How does the system perform across languages?
       → Per-language breakdown (Indonesian vs. English vs. Mixed vs. Arabic)

Usage:
    python run_baselines.py --base-url http://localhost:8080 --token YOUR_TOKEN
"""

import argparse
import json
import time
from pathlib import Path

from evaluate import run_evaluation, load_gold_standard

# ── Baseline configurations ──
BASELINES = {
    "full_system": {
        "label": "Kizana (Full)",
        "config": {},
        "description": "Complete system with all features",
    },
    "no_translation": {
        "label": "No Translation",
        "config": {"disable_query_translation": True},
        "description": "Queries passed directly to Tantivy without Arabic expansion",
    },
    "raw_bm25": {
        "label": "Raw BM25",
        "config": {"raw_bm25_only": True},
        "description": "Translation enabled but no custom scoring adjustments",
    },
    "translation_only": {
        "label": "Translation Only",
        "config": {
            "disable_hierarchy_boost": True,
            "disable_parent_boost": True,
            "disable_book_penalty": True,
            "disable_diversity_cap": True,
        },
        "description": "Only query translation, no scoring adjustments",
    },
    "scoring_only": {
        "label": "Scoring Only",
        "config": {
            "disable_phrase_mapping": True,
            "disable_multi_variant": True,
        },
        "description": "No phrase/variant expansion, but custom scoring enabled",
    },
    "with_stemmer": {
        "label": "Full + Stemmer",
        "config": {"enable_arabic_stemmer": True},
        "description": "Full system plus Arabic morphological expansion",
    },
}


def print_comparison_table(results: dict, key_metrics: list, title: str = ""):
    """Print a formatted comparison table."""
    print(f"\n{'='*80}")
    if title:
        print(title)
        print(f"{'='*80}")
    
    try:
        from tabulate import tabulate
        headers = ["System", "Queries"] + key_metrics + ["Avg ms"]
        rows = []
        for name, r in results.items():
            row = [r["label"], r["total_queries"]]
            for m in key_metrics:
                row.append(r["metrics"].get(m, 0.0))
            row.append(r["avg_search_time_ms"])
            rows.append(row)
        print(tabulate(rows, headers=headers, floatfmt=".4f"))
    except ImportError:
        header = f"{'System':20s} | {'N':>4s} | " + " | ".join(f"{m:>8s}" for m in key_metrics) + " | {'ms':>6s}"
        print(header)
        print("-" * len(header))
        for name, r in results.items():
            values = " | ".join(f"{r['metrics'].get(m, 0.0):8.4f}" for m in key_metrics)
            print(f"{r['label']:20s} | {r['total_queries']:4d} | {values} | {r['avg_search_time_ms']:6.1f}")


def statistical_significance_note():
    """Print note about statistical significance."""
    return """
Note on Statistical Significance:
  For Q1 journal submission, pair these results with:
  - Paired t-test or Wilcoxon signed-rank test on per-query AP scores
  - Bootstrap confidence intervals (1000 resamples)
  - Effect size (Cohen's d)
  
  These tests require manual relevance judgments for highest validity.
  Automatic keyword-based judgments provide reproducible baseline metrics.
"""


def main():
    parser = argparse.ArgumentParser(description="Kizana Baseline Comparison")
    parser.add_argument("--base-url", default="http://127.0.0.1:8080")
    parser.add_argument("--token", required=True, help="Admin JWT token")
    parser.add_argument("--max-results", type=int, default=20)
    parser.add_argument("--output-dir", default=str(Path(__file__).parent / "results"))
    args = parser.parse_args()
    
    gold_queries = load_gold_standard()
    output_dir = Path(args.output_dir)
    output_dir.mkdir(exist_ok=True)
    
    key_metrics = ["MAP", "MRR", "NDCG@5", "NDCG@10", "P@5", "P@10"]
    
    print(f"\n{'='*80}")
    print(f"KIZANA SEARCH — BASELINE COMPARISON STUDY")
    print(f"{'='*80}")
    print(f"  Total queries: {len(gold_queries)}")
    print(f"  URL: {args.base_url}")
    print()
    
    all_results = {}
    per_query_data = {}  # For statistical tests
    
    for name, baseline in BASELINES.items():
        print(f"  Running: {baseline['label']:25s} ... ", end="", flush=True)
        
        start = time.time()
        result = run_evaluation(
            base_url=args.base_url,
            token=args.token,
            config=baseline["config"],
            gold_queries=gold_queries,
            max_results=args.max_results,
        )
        elapsed = time.time() - start
        
        agg = result["aggregate_metrics"]
        print(f"done ({elapsed:.1f}s) — MAP: {agg.get('MAP', 0):.3f}, MRR: {agg.get('MRR', 0):.3f}")
        
        all_results[name] = {
            "label": baseline["label"],
            "description": baseline["description"],
            "config": baseline["config"],
            "metrics": agg,
            "per_language": result["per_language_metrics"],
            "per_domain": result["per_domain_metrics"],
            "total_queries": result["total_queries"],
            "avg_search_time_ms": result["avg_search_time_ms"],
        }
        
        # Collect per-query AP for statistical tests
        per_query_data[name] = [
            pqr["metrics"].get("AP", 0.0) for pqr in result["per_query_results"]
        ]
    
    # ── RQ1: Cross-lingual Translation Impact ──
    print_comparison_table(
        {k: all_results[k] for k in ["full_system", "no_translation"] if k in all_results},
        key_metrics,
        "RQ1: Impact of Cross-Lingual Query Translation"
    )
    
    if "full_system" in all_results and "no_translation" in all_results:
        full_map = all_results["full_system"]["metrics"].get("MAP", 0)
        no_tl_map = all_results["no_translation"]["metrics"].get("MAP", 0)
        improvement = ((full_map - no_tl_map) / max(no_tl_map, 0.001)) * 100
        print(f"\n  → Translation improves MAP by {improvement:+.1f}%")
    
    # ── RQ2: Domain-Aware Scoring Impact ──
    print_comparison_table(
        {k: all_results[k] for k in ["full_system", "raw_bm25", "translation_only", "scoring_only"] if k in all_results},
        key_metrics,
        "RQ2: Impact of Domain-Aware Scoring"
    )
    
    # ── RQ4: Per-Language Performance ──
    print(f"\n{'='*80}")
    print("RQ4: Cross-Language Performance Breakdown")
    print(f"{'='*80}")
    
    for name in ["full_system", "no_translation"]:
        if name not in all_results:
            continue
        per_lang = all_results[name].get("per_language", {})
        if per_lang:
            print(f"\n  {all_results[name]['label']}:")
            for lang in sorted(per_lang.keys()):
                m = per_lang[lang]
                print(f"    {lang:8s} — MAP: {m.get('MAP', 0):.3f}, MRR: {m.get('MRR', 0):.3f}, "
                      f"NDCG@5: {m.get('NDCG@5', 0):.3f}, P@5: {m.get('P@5', 0):.3f}")
    
    # ── Statistical significance (per-query AP comparison) ──
    if "full_system" in per_query_data and "no_translation" in per_query_data:
        try:
            import numpy as np
            full_ap = np.array(per_query_data["full_system"])
            notl_ap = np.array(per_query_data["no_translation"])
            
            diff = full_ap - notl_ap
            mean_diff = np.mean(diff)
            std_diff = np.std(diff, ddof=1)
            n = len(diff)
            
            if std_diff > 0:
                t_stat = mean_diff / (std_diff / np.sqrt(n))
                print(f"\n  Paired t-test (Full vs No Translation):")
                print(f"    Mean AP difference: {mean_diff:+.4f}")
                print(f"    t-statistic: {t_stat:.3f}")
                print(f"    N: {n}")
                print(f"    Cohen's d: {mean_diff / std_diff:.3f}")
        except ImportError:
            pass
    
    print(statistical_significance_note())
    
    # ── Save all results ──
    timestamp = time.strftime("%Y%m%d_%H%M%S")
    output_file = output_dir / f"baselines_{timestamp}.json"
    
    save_data = {
        "timestamp": timestamp,
        "total_queries": len(gold_queries),
        "baselines": all_results,
        "per_query_ap": per_query_data,
    }
    
    with open(output_file, "w", encoding="utf-8") as f:
        json.dump(save_data, f, ensure_ascii=False, indent=2)
    print(f"Results saved to {output_file}")
    
    # ── Generate LaTeX tables ──
    latex_file = output_dir / f"baselines_{timestamp}.tex"
    with open(latex_file, "w", encoding="utf-8") as f:
        f.write("% Auto-generated baseline comparison for Kizana Search paper\n\n")
        
        # Table 1: Main comparison
        f.write("\\begin{table}[h]\n")
        f.write("\\centering\n")
        f.write("\\caption{Baseline Comparison Results}\n")
        f.write("\\label{tab:baselines}\n")
        f.write("\\begin{tabular}{l" + "c" * len(key_metrics) + "c}\n")
        f.write("\\toprule\n")
        f.write("System & " + " & ".join(key_metrics) + " & Avg(ms) \\\\\n")
        f.write("\\midrule\n")
        
        for name in BASELINES:
            if name not in all_results:
                continue
            r = all_results[name]
            label = r["label"].replace("_", "\\_")
            values = " & ".join(f"{r['metrics'].get(m, 0.0):.3f}" for m in key_metrics)
            ms = f"{r['avg_search_time_ms']:.1f}"
            if name == "full_system":
                f.write(f"\\textbf{{{label}}} & {values} & {ms} \\\\\n")
            else:
                f.write(f"{label} & {values} & {ms} \\\\\n")
        
        f.write("\\bottomrule\n")
        f.write("\\end{tabular}\n")
        f.write("\\end{table}\n\n")
        
        # Table 2: Per-language for full system
        if "full_system" in all_results and all_results["full_system"].get("per_language"):
            f.write("\\begin{table}[h]\n")
            f.write("\\centering\n")
            f.write("\\caption{Per-Language Performance (Full System)}\n")
            f.write("\\label{tab:per-language}\n")
            f.write("\\begin{tabular}{l" + "c" * len(key_metrics) + "}\n")
            f.write("\\toprule\n")
            f.write("Language & " + " & ".join(key_metrics) + " \\\\\n")
            f.write("\\midrule\n")
            
            for lang, m in sorted(all_results["full_system"]["per_language"].items()):
                values = " & ".join(f"{m.get(metric, 0.0):.3f}" for metric in key_metrics)
                f.write(f"{lang} & {values} \\\\\n")
            
            f.write("\\bottomrule\n")
            f.write("\\end{tabular}\n")
            f.write("\\end{table}\n")
    
    print(f"LaTeX tables saved to {latex_file}")


if __name__ == "__main__":
    main()
