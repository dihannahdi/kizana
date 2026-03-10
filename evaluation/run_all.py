w#!/usr/bin/env python3
"""
Kizana Search — Complete Evaluation Suite
==========================================
Runs all evaluation tasks in sequence:
  1. Self-test metrics module
  2. Ablation study (10 configurations)
  3. Baseline comparison (6 configurations)
  4. Generate summary report

Usage:
    python run_all.py --base-url http://localhost:8080 --token YOUR_TOKEN
"""

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path


def run_script(name: str, args: list) -> int:
    """Run a Python script and return exit code."""
    script = Path(__file__).parent / name
    cmd = [sys.executable, str(script)] + args
    print(f"\n{'─'*60}")
    print(f"Running: {name}")
    print(f"{'─'*60}")
    result = subprocess.run(cmd, cwd=str(script.parent))
    return result.returncode


def main():
    parser = argparse.ArgumentParser(description="Kizana Complete Evaluation Suite")
    parser.add_argument("--base-url", default="http://127.0.0.1:8080")
    parser.add_argument("--token", required=True, help="Admin JWT token")
    parser.add_argument("--max-results", type=int, default=20)
    args = parser.parse_args()
    
    start = time.time()
    
    print(f"\n{'='*60}")
    print(f"KIZANA SEARCH — COMPLETE EVALUATION SUITE")
    print(f"{'='*60}")
    print(f"  Base URL: {args.base_url}")
    print(f"  Time:     {time.strftime('%Y-%m-%d %H:%M:%S')}")
    print()
    
    # 1. Self-test metrics
    print("Step 1: Verifying metrics module...")
    rc = run_script("metrics.py", [])
    if rc != 0:
        print("ERROR: Metrics self-test failed!")
        sys.exit(1)
    
    # 2. Ablation study
    print("\nStep 2: Running ablation study...")
    rc = run_script("run_ablation.py", [
        "--base-url", args.base_url,
        "--token", args.token,
        "--max-results", str(args.max_results),
    ])
    if rc != 0:
        print("ERROR: Ablation study failed!")
        sys.exit(1)
    
    # 3. Baseline comparison
    print("\nStep 3: Running baseline comparison...")
    rc = run_script("run_baselines.py", [
        "--base-url", args.base_url,
        "--token", args.token,
        "--max-results", str(args.max_results),
    ])
    if rc != 0:
        print("ERROR: Baseline comparison failed!")
        sys.exit(1)
    
    elapsed = time.time() - start
    
    print(f"\n{'='*60}")
    print(f"ALL EVALUATIONS COMPLETE")
    print(f"{'='*60}")
    print(f"  Total time: {elapsed:.1f}s ({elapsed/60:.1f}min)")
    print(f"  Results in: {Path(__file__).parent / 'results'}")
    print(f"\nGenerated files:")
    results_dir = Path(__file__).parent / "results"
    if results_dir.exists():
        for f in sorted(results_dir.iterdir()):
            size = f.stat().st_size
            print(f"  {f.name} ({size:,} bytes)")


if __name__ == "__main__":
    main()
