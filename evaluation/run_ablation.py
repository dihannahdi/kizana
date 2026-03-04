#!/usr/bin/env python3
"""
Kizana Search — Ablation Study Runner
=======================================
Runs the evaluation with different configurations to measure the contribution
of each system component. This produces the ablation table required for
Q1/Q2 journal publication.

Ablation Configurations:
  1. Full System (all features enabled)
  2. -Query Translation (disable cross-lingual mapping entirely)
  3. -Phrase Mapping (disable multi-word phrase expansion)
  4. -Multi-Variant (disable synonym expansion)
  5. -Hierarchy Boost (disable TOC depth weighting)
  6. -Parent Boost (disable parent section relevance)
  7. -Book Penalty (disable large-book normalization)
  8. -Diversity Cap (disable per-book result limiting)
  9. Raw BM25 Only (no custom scoring at all)
  10. +Arabic Stemmer (experimental: morphological expansion)

Usage:
    python run_ablation.py --base-url http://localhost:8080 --token YOUR_TOKEN
"""

import argparse
import json
import time
from pathlib import Path

from evaluate import run_evaluation, load_gold_standard, format_breakdown_table

# ── Ablation configurations ──
ABLATION_CONFIGS = {
    "full_system": {
        "label": "Full System",
        "config": {},
    },
    "no_translation": {
        "label": "- Query Translation",
        "config": {"disable_query_translation": True},
    },
    "no_phrases": {
        "label": "- Phrase Mapping",
        "config": {"disable_phrase_mapping": True},
    },
    "no_multi_variant": {
        "label": "- Multi-Variant",
        "config": {"disable_multi_variant": True},
    },
    "no_hierarchy": {
        "label": "- Hierarchy Boost",
        "config": {"disable_hierarchy_boost": True},
    },
    "no_parent": {
        "label": "- Parent Boost",
        "config": {"disable_parent_boost": True},
    },
    "no_book_penalty": {
        "label": "- Book Penalty",
        "config": {"disable_book_penalty": True},
    },
    "no_diversity": {
        "label": "- Diversity Cap",
        "config": {"disable_diversity_cap": True},
    },
    "raw_bm25": {
        "label": "Raw BM25 Only",
        "config": {"raw_bm25_only": True},
    },
    "with_stemmer": {
        "label": "+ Arabic Stemmer",
        "config": {"enable_arabic_stemmer": True},
    },
}


def main():
    parser = argparse.ArgumentParser(description="Kizana Ablation Study Runner")
    parser.add_argument("--base-url", default="http://127.0.0.1:8080")
    parser.add_argument("--token", required=True, help="Admin JWT token")
    parser.add_argument("--max-results", type=int, default=20)
    parser.add_argument("--output-dir", default=str(Path(__file__).parent / "results"))
    parser.add_argument("--configs", nargs="*", default=None,
                        help="Specific config names to run (default: all)")
    args = parser.parse_args()
    
    gold_queries = load_gold_standard()
    output_dir = Path(args.output_dir)
    output_dir.mkdir(exist_ok=True)
    
    configs_to_run = args.configs or list(ABLATION_CONFIGS.keys())
    
    print(f"\n{'='*70}")
    print(f"KIZANA SEARCH — ABLATION STUDY")
    print(f"{'='*70}")
    print(f"  Queries:  {len(gold_queries)}")
    print(f"  Configs:  {len(configs_to_run)}")
    print(f"  URL:      {args.base_url}")
    print()
    
    all_results = {}
    key_metrics = ["MAP", "MRR", "NDCG@5", "NDCG@10", "P@5", "P@10"]
    
    for config_name in configs_to_run:
        if config_name not in ABLATION_CONFIGS:
            print(f"  WARNING: Unknown config '{config_name}', skipping")
            continue
        
        ablation = ABLATION_CONFIGS[config_name]
        label = ablation["label"]
        config = ablation["config"]
        
        print(f"  Running: {label:25s} ... ", end="", flush=True)
        
        start = time.time()
        results = run_evaluation(
            base_url=args.base_url,
            token=args.token,
            config=config,
            gold_queries=gold_queries,
            max_results=args.max_results,
        )
        elapsed = time.time() - start
        
        agg = results["aggregate_metrics"]
        metrics_str = " | ".join(f"{m}: {agg.get(m, 0):.3f}" for m in key_metrics[:3])
        print(f"done ({elapsed:.1f}s) — {metrics_str}")
        
        all_results[config_name] = {
            "label": label,
            "config": config,
            "metrics": agg,
            "per_language": results["per_language_metrics"],
            "per_domain": results["per_domain_metrics"],
            "total_queries": results["total_queries"],
            "avg_search_time_ms": results["avg_search_time_ms"],
        }
    
    # ── Print comparison table ──
    print(f"\n{'='*70}")
    print("ABLATION RESULTS — Aggregate")
    print(f"{'='*70}")
    
    try:
        from tabulate import tabulate
        headers = ["Configuration", "MAP", "MRR", "NDCG@5", "NDCG@10", "P@5", "P@10", "Avg ms"]
        rows = []
        for config_name in configs_to_run:
            if config_name not in all_results:
                continue
            r = all_results[config_name]
            row = [r["label"]]
            for m in key_metrics:
                row.append(r["metrics"].get(m, 0.0))
            row.append(r["avg_search_time_ms"])
            rows.append(row)
        print(tabulate(rows, headers=headers, floatfmt=".4f"))
    except ImportError:
        # Fallback without tabulate
        header = f"{'Configuration':25s} | " + " | ".join(f"{m:>8s}" for m in key_metrics) + " | {'Avg ms':>8s}"
        print(header)
        print("-" * len(header))
        for config_name in configs_to_run:
            if config_name not in all_results:
                continue
            r = all_results[config_name]
            values = " | ".join(f"{r['metrics'].get(m, 0.0):8.4f}" for m in key_metrics)
            print(f"{r['label']:25s} | {values} | {r['avg_search_time_ms']:8.1f}")
    
    # ── Compute deltas from full system ──
    if "full_system" in all_results:
        print(f"\n{'='*70}")
        print("DELTA FROM FULL SYSTEM (negative = component helps)")
        print(f"{'='*70}")
        
        full = all_results["full_system"]["metrics"]
        
        try:
            from tabulate import tabulate
            headers = ["Configuration"] + key_metrics
            rows = []
            for config_name in configs_to_run:
                if config_name == "full_system" or config_name not in all_results:
                    continue
                r = all_results[config_name]
                row = [r["label"]]
                for m in key_metrics:
                    delta = r["metrics"].get(m, 0.0) - full.get(m, 0.0)
                    row.append(delta)
                rows.append(row)
            print(tabulate(rows, headers=headers, floatfmt="+.4f"))
        except ImportError:
            for config_name in configs_to_run:
                if config_name == "full_system" or config_name not in all_results:
                    continue
                r = all_results[config_name]
                deltas = " | ".join(
                    f"{r['metrics'].get(m, 0.0) - full.get(m, 0.0):+8.4f}" for m in key_metrics
                )
                print(f"{r['label']:25s} | {deltas}")
    
    # ── Per-language comparison for full vs raw BM25 ──
    if "full_system" in all_results and "raw_bm25" in all_results:
        print(f"\n{'='*70}")
        print("PER-LANGUAGE: Full System vs Raw BM25")
        print(f"{'='*70}")
        
        for lang in sorted(all_results["full_system"].get("per_language", {}).keys()):
            full_lang = all_results["full_system"]["per_language"].get(lang, {})
            raw_lang = all_results["raw_bm25"]["per_language"].get(lang, {})
            
            print(f"\n  Language: {lang}")
            print(f"    {'Metric':>10s} | {'Full':>8s} | {'Raw BM25':>8s} | {'Delta':>8s}")
            print(f"    {'-'*42}")
            for m in key_metrics:
                f_val = full_lang.get(m, 0.0)
                r_val = raw_lang.get(m, 0.0)
                delta = f_val - r_val
                print(f"    {m:>10s} | {f_val:8.4f} | {r_val:8.4f} | {delta:+8.4f}")
    
    # ── Save results ──
    timestamp = time.strftime("%Y%m%d_%H%M%S")
    output_file = output_dir / f"ablation_{timestamp}.json"
    with open(output_file, "w", encoding="utf-8") as f:
        json.dump(all_results, f, ensure_ascii=False, indent=2)
    print(f"\nResults saved to {output_file}")
    
    # ── Generate LaTeX table for paper ──
    latex_file = output_dir / f"ablation_{timestamp}.tex"
    with open(latex_file, "w", encoding="utf-8") as f:
        f.write("% Auto-generated ablation table for Kizana Search paper\n")
        f.write("\\begin{table}[h]\n")
        f.write("\\centering\n")
        f.write("\\caption{Ablation Study Results}\n")
        f.write("\\label{tab:ablation}\n")
        f.write("\\begin{tabular}{l" + "c" * len(key_metrics) + "}\n")
        f.write("\\toprule\n")
        f.write("Configuration & " + " & ".join(key_metrics) + " \\\\\n")
        f.write("\\midrule\n")
        
        for config_name in configs_to_run:
            if config_name not in all_results:
                continue
            r = all_results[config_name]
            label = r["label"].replace("_", "\\_")
            values = " & ".join(f"{r['metrics'].get(m, 0.0):.3f}" for m in key_metrics)
            if config_name == "full_system":
                f.write(f"\\textbf{{{label}}} & {values} \\\\\n")
            else:
                f.write(f"{label} & {values} \\\\\n")
        
        f.write("\\bottomrule\n")
        f.write("\\end{tabular}\n")
        f.write("\\end{table}\n")
    
    print(f"LaTeX table saved to {latex_file}")


if __name__ == "__main__":
    main()
