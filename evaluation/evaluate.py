#!/usr/bin/env python3
"""
Kizana Search Evaluation Runner
================================
Sends gold standard queries to the evaluation endpoint and judges results.

Relevance is judged automatically by checking if relevant Arabic keywords
appear in the returned text content. This provides a reproducible baseline
for comparison (manual judgments can be added later for higher accuracy).

Usage:
    python evaluate.py --base-url http://localhost:8080 --token YOUR_ADMIN_TOKEN
    python evaluate.py --base-url https://bahtsulmasail.tech --token YOUR_TOKEN --config '{"raw_bm25_only": true}'
"""

import argparse
import json
import sys
import time
from pathlib import Path
from typing import Dict, List, Optional, Tuple

import requests
from metrics import compute_all_metrics, compute_batch_metrics

# ── Constants ──
GOLD_STANDARD_FILE = Path(__file__).parent / "gold_standard.jsonl"
DEFAULT_K_VALUES = [1, 3, 5, 10, 20]
BATCH_SIZE = 20  # Queries per API call


def load_gold_standard(path: Path = GOLD_STANDARD_FILE) -> List[Dict]:
    """Load gold standard queries from JSONL file."""
    queries = []
    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if line:
                queries.append(json.loads(line))
    return queries


def judge_result(result: Dict, relevant_keywords: List[str]) -> int:
    """
    Auto-judge a single search result against relevant keywords.
    
    Relevance scale:
        3 = highly relevant (3+ keyword matches)
        2 = relevant (2 keyword matches)
        1 = partially relevant (1 keyword match)
        0 = not relevant (0 keyword matches)
    
    We search in the result's content, title, hierarchy, and book_name fields.
    """
    hierarchy = result.get("hierarchy", [])
    if isinstance(hierarchy, list):
        hierarchy = " ".join(hierarchy)
    elif not isinstance(hierarchy, str):
        hierarchy = str(hierarchy)
    
    content = " ".join([
        result.get("content_snippet", ""),
        result.get("content", ""),
        result.get("title", ""),
        hierarchy,
        result.get("book_name", ""),
        result.get("section", ""),
    ])
    
    matches = 0
    for kw in relevant_keywords:
        if kw in content:
            matches += 1
    
    if matches >= 3:
        return 3
    elif matches >= 2:
        return 2
    elif matches >= 1:
        return 1
    return 0


def run_evaluation(
    base_url: str,
    token: str,
    config: Optional[Dict] = None,
    gold_queries: Optional[List[Dict]] = None,
    max_results: int = 20,
    k_values: List[int] = DEFAULT_K_VALUES,
    verbose: bool = False,
) -> Dict:
    """
    Run evaluation against the Kizana eval endpoint.
    
    Args:
        base_url: Backend base URL (e.g., http://localhost:8080)
        token: Admin JWT token
        config: EvalConfig override (for ablation study)
        gold_queries: Gold standard queries (loaded from file if None)
        max_results: Max results per query
        k_values: K values for metrics
        verbose: Print per-query details
    
    Returns:
        Dictionary with metrics and per-query results
    """
    if gold_queries is None:
        gold_queries = load_gold_standard()
    
    if config is None:
        config = {}
    
    # Build batch request
    eval_queries = [{"id": q["id"], "text": q["text"]} for q in gold_queries]
    
    all_relevance_scores = []
    per_query_results = []
    total_search_time = 0
    
    # Process in batches
    for batch_start in range(0, len(eval_queries), BATCH_SIZE):
        batch = eval_queries[batch_start:batch_start + BATCH_SIZE]
        
        payload = {
            "queries": batch,
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
        except requests.RequestException as e:
            print(f"ERROR: Batch request failed: {e}")
            if hasattr(e, 'response') and e.response is not None:
                print(f"  Response: {e.response.text[:500]}")
            sys.exit(1)
        
        total_search_time += data.get("total_time_ms", 0)
        
        # Judge results
        for result in data["results"]:
            query_id = result["query_id"]
            gold = next((q for q in gold_queries if q["id"] == query_id), None)
            if not gold:
                continue
            
            relevance_scores = []
            for r in result["results"]:
                rel = judge_result(r, gold["relevant_keywords"])
                relevance_scores.append(rel)
            
            # Pad to max_results if fewer results returned
            while len(relevance_scores) < max_results:
                relevance_scores.append(0)
            
            all_relevance_scores.append(relevance_scores)
            
            query_metrics = compute_all_metrics(
                relevance_scores, k_values, gold.get("min_relevant", 3)
            )
            
            per_query_results.append({
                "query_id": query_id,
                "query_text": result["query_text"],
                "lang": gold.get("lang", "unknown"),
                "domain": gold.get("domain", "unknown"),
                "num_results": result["num_results"],
                "search_time_ms": result["search_time_ms"],
                "translated_terms": result["translated_terms"],
                "relevance_scores": relevance_scores[:result["num_results"]],
                "metrics": query_metrics,
            })
            
            if verbose:
                print(f"  [{query_id}] {result['query_text'][:60]}...")
                print(f"    Results: {result['num_results']}, NDCG@5: {query_metrics.get('NDCG@5', 0):.3f}, "
                      f"P@5: {query_metrics.get('P@5', 0):.3f}, RR: {query_metrics.get('RR', 0):.3f}")
    
    # Compute aggregate metrics
    batch_metrics = compute_batch_metrics(all_relevance_scores, k_values)
    
    # Compute per-language and per-domain breakdowns
    lang_groups = {}
    domain_groups = {}
    for pqr in per_query_results:
        lang = pqr["lang"]
        domain = pqr["domain"]
        if lang not in lang_groups:
            lang_groups[lang] = []
        lang_groups[lang].append(pqr["relevance_scores"])
        if domain not in domain_groups:
            domain_groups[domain] = []
        domain_groups[domain].append(pqr["relevance_scores"])
    
    lang_metrics = {}
    for lang, scores in lang_groups.items():
        lang_metrics[lang] = compute_batch_metrics(scores, k_values)
    
    domain_metrics = {}
    for domain, scores in domain_groups.items():
        domain_metrics[domain] = compute_batch_metrics(scores, k_values)
    
    return {
        "config": config,
        "total_queries": len(per_query_results),
        "total_search_time_ms": total_search_time,
        "avg_search_time_ms": total_search_time / max(len(per_query_results), 1),
        "aggregate_metrics": batch_metrics,
        "per_language_metrics": lang_metrics,
        "per_domain_metrics": domain_metrics,
        "per_query_results": per_query_results,
    }


def format_metrics_table(metrics: Dict[str, float], title: str = "") -> str:
    """Format metrics as a readable table."""
    try:
        from tabulate import tabulate
        rows = sorted(metrics.items(), key=lambda x: x[0])
        table = tabulate(rows, headers=["Metric", "Value"], floatfmt=".4f")
        if title:
            return f"\n{'='*60}\n{title}\n{'='*60}\n{table}\n"
        return table
    except ImportError:
        lines = [f"\n{'='*60}", title if title else "Metrics", f"{'='*60}"]
        for k, v in sorted(metrics.items()):
            lines.append(f"  {k:15s}: {v:.4f}")
        return "\n".join(lines)


def format_breakdown_table(breakdown: Dict[str, Dict[str, float]], group_name: str, key_metrics: List[str] = None) -> str:
    """Format per-group metrics as a comparative table."""
    if key_metrics is None:
        key_metrics = ["MAP", "MRR", "NDCG@5", "NDCG@10", "P@5", "P@10"]
    
    try:
        from tabulate import tabulate
        headers = [group_name] + key_metrics
        rows = []
        for group, metrics in sorted(breakdown.items()):
            row = [group] + [metrics.get(m, 0.0) for m in key_metrics]
            rows.append(row)
        return tabulate(rows, headers=headers, floatfmt=".4f")
    except ImportError:
        lines = [f"\n{group_name:12s} | " + " | ".join(f"{m:>8s}" for m in key_metrics)]
        lines.append("-" * len(lines[0]))
        for group, metrics in sorted(breakdown.items()):
            values = " | ".join(f"{metrics.get(m, 0.0):8.4f}" for m in key_metrics)
            lines.append(f"{group:12s} | {values}")
        return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser(description="Kizana Search Evaluation Runner")
    parser.add_argument("--base-url", default="http://127.0.0.1:8080",
                        help="Backend base URL")
    parser.add_argument("--token", required=True, help="Admin JWT token")
    parser.add_argument("--config", default="{}", help="EvalConfig JSON string")
    parser.add_argument("--config-name", default="default", help="Name for this configuration")
    parser.add_argument("--max-results", type=int, default=20, help="Max results per query")
    parser.add_argument("--gold-file", default=str(GOLD_STANDARD_FILE),
                        help="Path to gold standard JSONL file")
    parser.add_argument("--output", help="Output JSON file for results")
    parser.add_argument("--verbose", "-v", action="store_true", help="Print per-query details")
    parser.add_argument("--lang-filter", help="Only evaluate queries of this language (id/en/ar/mixed)")
    parser.add_argument("--domain-filter", help="Only evaluate queries of this domain")
    args = parser.parse_args()
    
    config = json.loads(args.config)
    gold_queries = load_gold_standard(Path(args.gold_file))
    
    # Apply filters
    if args.lang_filter:
        gold_queries = [q for q in gold_queries if q.get("lang") == args.lang_filter]
    if args.domain_filter:
        gold_queries = [q for q in gold_queries if q.get("domain") == args.domain_filter]
    
    print(f"\n{'='*60}")
    print(f"Kizana Search Evaluation")
    print(f"{'='*60}")
    print(f"  URL:       {args.base_url}")
    print(f"  Config:    {args.config_name} {json.dumps(config) if config else '(default)'}")
    print(f"  Queries:   {len(gold_queries)}")
    print(f"  Max K:     {args.max_results}")
    print()
    
    start = time.time()
    results = run_evaluation(
        base_url=args.base_url,
        token=args.token,
        config=config,
        gold_queries=gold_queries,
        max_results=args.max_results,
        verbose=args.verbose,
    )
    elapsed = time.time() - start
    
    print(f"\nEvaluation completed in {elapsed:.1f}s")
    print(f"Total search time: {results['total_search_time_ms']}ms "
          f"(avg {results['avg_search_time_ms']:.1f}ms/query)")
    
    # Print aggregate metrics
    print(format_metrics_table(results["aggregate_metrics"], f"Aggregate Metrics ({results['total_queries']} queries)"))
    
    # Print per-language breakdown
    if results["per_language_metrics"]:
        print(f"\n--- Per-Language Breakdown ---")
        print(format_breakdown_table(results["per_language_metrics"], "Language"))
    
    # Print per-domain breakdown
    if results["per_domain_metrics"]:
        print(f"\n--- Per-Domain Breakdown ---")
        print(format_breakdown_table(results["per_domain_metrics"], "Domain"))
    
    # Save results
    if args.output:
        output_path = Path(args.output)
        with open(output_path, "w", encoding="utf-8") as f:
            json.dump(results, f, ensure_ascii=False, indent=2)
        print(f"\nResults saved to {output_path}")
    
    # Also save to timestamped file
    timestamp = time.strftime("%Y%m%d_%H%M%S")
    auto_output = Path(__file__).parent / "results" / f"eval_{args.config_name}_{timestamp}.json"
    auto_output.parent.mkdir(exist_ok=True)
    with open(auto_output, "w", encoding="utf-8") as f:
        json.dump(results, f, ensure_ascii=False, indent=2)
    print(f"Results auto-saved to {auto_output}")


if __name__ == "__main__":
    main()
