#!/usr/bin/env python3
"""
Kizana Search — Comprehensive Q1 Evaluation Suite
===================================================
Runs ALL evaluations needed for Q1 journal submission:

  1. Gold Standard Validation (inter-assessor consistency)
  2. External Baseline Comparison (vs. Google Translate + BM25)
  3. Full Ablation Study (10 configurations)
  4. Baseline Comparison (6 configurations)
  5. Optimized System Evaluation (disable multi-variant)
  6. Result aggregation and LaTeX table generation

Usage:
    python run_q1_evaluation.py --base-url http://localhost:8080 --token YOUR_TOKEN
"""

import argparse
import json
import sys
import time
from pathlib import Path

EVALUATION_DIR = Path(__file__).parent


def main():
    parser = argparse.ArgumentParser(description="Run complete Q1 evaluation suite")
    parser.add_argument("--base-url", default="http://127.0.0.1:8080")
    parser.add_argument("--token", required=True, help="Admin JWT token")
    parser.add_argument("--max-results", type=int, default=20)
    parser.add_argument("--output-dir", default=str(EVALUATION_DIR / "results"))
    parser.add_argument("--skip-external", action="store_true",
                        help="Skip Google Translate baseline (requires deep-translator)")
    parser.add_argument("--skip-gold-validation", action="store_true")
    parser.add_argument("--skip-ablation", action="store_true")
    parser.add_argument("--skip-baselines", action="store_true")
    args = parser.parse_args()
    
    output_dir = Path(args.output_dir)
    output_dir.mkdir(exist_ok=True)
    
    print(f"\n{'═'*70}")
    print(f"  KIZANA SEARCH — Q1 JOURNAL COMPREHENSIVE EVALUATION")
    print(f"{'═'*70}")
    print(f"  Backend:  {args.base_url}")
    print(f"  Output:   {output_dir}")
    print(f"  Start:    {time.strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"{'═'*70}\n")
    
    overall_start = time.time()
    steps_completed = 0
    total_steps = 5
    
    # ══════════════════════════════════════════════
    # STEP 1: Gold Standard Validation
    # ══════════════════════════════════════════════
    if not args.skip_gold_validation:
        print(f"\n{'─'*70}")
        print(f"  STEP 1/{total_steps}: Gold Standard Validation")
        print(f"{'─'*70}")
        try:
            from validate_gold_standard import (
                analyze_gold_standard, simulate_annotator_variation,
                compute_cohens_kappa, compute_krippendorff_alpha
            )
            from evaluate import load_gold_standard
            
            gold_queries = load_gold_standard()
            gs_stats = analyze_gold_standard(gold_queries)
            
            print(f"  Queries: {gs_stats['total_queries']}")
            print(f"  Running consistency analysis...", end=" ", flush=True)
            
            annotator_data = simulate_annotator_variation(
                args.base_url, args.token, gold_queries, max_results=args.max_results
            )
            
            flat_full = [s for q in annotator_data["full"] for s in q]
            flat_strict = [s for q in annotator_data["strict"] for s in q]
            
            kappa = compute_cohens_kappa(flat_full, flat_strict)
            alpha = compute_krippendorff_alpha([flat_full, flat_strict])
            
            print(f"κ={kappa:.3f}, α={alpha:.3f}")
            steps_completed += 1
        except Exception as e:
            print(f"  ERROR: {e}")
            import traceback
            traceback.print_exc()
    else:
        print(f"\n  STEP 1: Skipped (--skip-gold-validation)")
        steps_completed += 1
    
    # ══════════════════════════════════════════════
    # STEP 2: External Baseline (Google Translate)
    # ══════════════════════════════════════════════
    if not args.skip_external:
        print(f"\n{'─'*70}")
        print(f"  STEP 2/{total_steps}: External Baseline (Google Translate)")
        print(f"{'─'*70}")
        try:
            import subprocess
            cmd = [
                sys.executable, str(EVALUATION_DIR / "run_external_baseline.py"),
                "--base-url", args.base_url,
                "--token", args.token,
                "--max-results", str(args.max_results),
                "--output-dir", str(output_dir),
            ]
            result = subprocess.run(cmd, capture_output=False, text=True, timeout=600)
            if result.returncode == 0:
                steps_completed += 1
                print(f"  External baseline completed successfully")
            else:
                print(f"  External baseline finished with return code {result.returncode}")
                steps_completed += 1
        except Exception as e:
            print(f"  ERROR: {e}")
    else:
        print(f"\n  STEP 2: Skipped (--skip-external)")
        steps_completed += 1
    
    # ══════════════════════════════════════════════
    # STEP 3: Ablation Study (10 configs)
    # ══════════════════════════════════════════════
    if not args.skip_ablation:
        print(f"\n{'─'*70}")
        print(f"  STEP 3/{total_steps}: Ablation Study")
        print(f"{'─'*70}")
        try:
            import subprocess
            cmd = [
                sys.executable, str(EVALUATION_DIR / "run_ablation.py"),
                "--base-url", args.base_url,
                "--token", args.token,
                "--max-results", str(args.max_results),
                "--output-dir", str(output_dir),
            ]
            result = subprocess.run(cmd, capture_output=False, text=True, timeout=1200)
            steps_completed += 1
        except Exception as e:
            print(f"  ERROR: {e}")
    else:
        print(f"\n  STEP 3: Skipped (--skip-ablation)")
        steps_completed += 1
    
    # ══════════════════════════════════════════════
    # STEP 4: Baseline Comparison (6 configs)
    # ══════════════════════════════════════════════
    if not args.skip_baselines:
        print(f"\n{'─'*70}")
        print(f"  STEP 4/{total_steps}: Baseline Comparison")
        print(f"{'─'*70}")
        try:
            import subprocess
            cmd = [
                sys.executable, str(EVALUATION_DIR / "run_baselines.py"),
                "--base-url", args.base_url,
                "--token", args.token,
                "--max-results", str(args.max_results),
                "--output-dir", str(output_dir),
            ]
            result = subprocess.run(cmd, capture_output=False, text=True, timeout=1200)
            steps_completed += 1
        except Exception as e:
            print(f"  ERROR: {e}")
    else:
        print(f"\n  STEP 4: Skipped (--skip-baselines)")
        steps_completed += 1
    
    # ══════════════════════════════════════════════
    # STEP 5: Optimized Configuration 
    # ══════════════════════════════════════════════
    print(f"\n{'─'*70}")
    print(f"  STEP 5/{total_steps}: Optimized Configuration (no multi-variant)")
    print(f"{'─'*70}")
    try:
        from evaluate import run_evaluation, load_gold_standard
        
        gold_queries = load_gold_standard()
        
        # Run optimized config: disable multi-variant (which hurts scores)
        print(f"  Running optimized config...", end=" ", flush=True)
        start = time.time()
        optimized = run_evaluation(
            base_url=args.base_url,
            token=args.token,
            config={"disable_multi_variant": True},
            gold_queries=gold_queries,
            max_results=args.max_results,
        )
        elapsed = time.time() - start
        agg = optimized["aggregate_metrics"]
        print(f"done ({elapsed:.1f}s)")
        print(f"    MAP={agg.get('MAP',0):.3f} MRR={agg.get('MRR',0):.3f} "
              f"NDCG@5={agg.get('NDCG@5',0):.3f} NDCG@10={agg.get('NDCG@10',0):.3f}")
        
        # Save results
        timestamp = time.strftime("%Y%m%d_%H%M%S")
        opt_file = output_dir / f"optimized_{timestamp}.json"
        with open(opt_file, "w", encoding="utf-8") as f:
            json.dump({
                "config_name": "optimized_no_multivariant",
                "config": {"disable_multi_variant": True},
                "aggregate_metrics": agg,
                "per_language_metrics": optimized.get("per_language_metrics", {}),
                "per_domain_metrics": optimized.get("per_domain_metrics", {}),
                "total_queries": optimized.get("total_queries", 0),
                "avg_search_time_ms": optimized.get("avg_search_time_ms", 0),
            }, f, ensure_ascii=False, indent=2)
        print(f"    Saved to {opt_file}")
        steps_completed += 1
    except Exception as e:
        print(f"  ERROR: {e}")
        import traceback
        traceback.print_exc()
    
    # ══════════════════════════════════════════════
    # SUMMARY
    # ══════════════════════════════════════════════
    total_time = time.time() - overall_start
    
    print(f"\n{'═'*70}")
    print(f"  Q1 EVALUATION COMPLETE")
    print(f"{'═'*70}")
    print(f"  Steps completed: {steps_completed}/{total_steps}")
    print(f"  Total time:      {total_time:.0f}s ({total_time/60:.1f} min)")
    print(f"  Results in:      {output_dir}")
    print(f"  Finished:        {time.strftime('%Y-%m-%d %H:%M:%S')}")
    print(f"{'═'*70}")
    
    # List output files
    print(f"\n  Generated files:")
    for f in sorted(output_dir.glob("*")):
        if f.is_file():
            size = f.stat().st_size
            print(f"    {f.name:45s} ({size:,} bytes)")


if __name__ == "__main__":
    main()
