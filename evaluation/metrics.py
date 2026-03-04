"""
IR Evaluation Metrics for Kizana Search
========================================
Implements standard information retrieval metrics for academic evaluation:
- NDCG@K (Normalized Discounted Cumulative Gain)
- MAP (Mean Average Precision)
- P@K (Precision at K)
- MRR (Mean Reciprocal Rank)
- Recall@K

All metrics follow definitions from:
- Manning, Raghavan & Schütze (2008). Introduction to Information Retrieval.
- Järvelin & Kekäläinen (2002). Cumulated gain-based evaluation of IR techniques.
"""

import math
from typing import List, Dict, Optional


def dcg_at_k(relevance_scores: List[int], k: int) -> float:
    """
    Discounted Cumulative Gain at position K.
    
    DCG@K = Σ(i=1..K) (2^rel_i - 1) / log2(i + 1)
    
    Args:
        relevance_scores: List of relevance judgments (0=irrelevant, 1=partial, 2=relevant, 3=highly_relevant)
        k: Cutoff position
    
    Returns:
        DCG@K score (float)
    """
    dcg = 0.0
    for i, rel in enumerate(relevance_scores[:k]):
        dcg += (2 ** rel - 1) / math.log2(i + 2)  # i+2 because i is 0-indexed
    return dcg


def ndcg_at_k(relevance_scores: List[int], k: int) -> float:
    """
    Normalized Discounted Cumulative Gain at position K.
    
    NDCG@K = DCG@K / IDCG@K
    where IDCG@K is DCG@K of the ideal ranking (sorted by relevance desc).
    
    Args:
        relevance_scores: List of relevance judgments for returned results
        k: Cutoff position
    
    Returns:
        NDCG@K score in [0, 1]
    """
    actual_dcg = dcg_at_k(relevance_scores, k)
    ideal_scores = sorted(relevance_scores, reverse=True)
    ideal_dcg = dcg_at_k(ideal_scores, k)
    
    if ideal_dcg == 0:
        return 0.0
    return actual_dcg / ideal_dcg


def precision_at_k(relevance_scores: List[int], k: int, threshold: int = 1) -> float:
    """
    Precision at K: fraction of top-K results that are relevant.
    
    P@K = |relevant in top K| / K
    
    Args:
        relevance_scores: List of relevance judgments
        k: Cutoff position
        threshold: Minimum relevance score to count as "relevant" (default=1)
    
    Returns:
        P@K score in [0, 1]
    """
    top_k = relevance_scores[:k]
    if not top_k:
        return 0.0
    relevant_count = sum(1 for r in top_k if r >= threshold)
    return relevant_count / len(top_k)


def average_precision(relevance_scores: List[int], threshold: int = 1) -> float:
    """
    Average Precision for a single query.
    
    AP = (1/R) Σ(k=1..N) P@k × rel(k)
    where R = total relevant documents
    
    Args:
        relevance_scores: List of relevance judgments for all returned results
        threshold: Minimum relevance score to count as "relevant"
    
    Returns:
        AP score in [0, 1]
    """
    relevant_count = 0
    precision_sum = 0.0
    
    for i, rel in enumerate(relevance_scores):
        if rel >= threshold:
            relevant_count += 1
            precision_sum += relevant_count / (i + 1)
    
    if relevant_count == 0:
        return 0.0
    return precision_sum / relevant_count


def mean_average_precision(all_relevance_scores: List[List[int]], threshold: int = 1) -> float:
    """
    Mean Average Precision across all queries.
    
    MAP = (1/Q) Σ(q=1..Q) AP(q)
    
    Args:
        all_relevance_scores: List of relevance score lists, one per query
        threshold: Minimum relevance score to count as "relevant"
    
    Returns:
        MAP score in [0, 1]
    """
    if not all_relevance_scores:
        return 0.0
    return sum(average_precision(scores, threshold) for scores in all_relevance_scores) / len(all_relevance_scores)


def reciprocal_rank(relevance_scores: List[int], threshold: int = 1) -> float:
    """
    Reciprocal Rank: 1 / position of first relevant result.
    
    Args:
        relevance_scores: List of relevance judgments
        threshold: Minimum relevance score to count as "relevant"
    
    Returns:
        RR score in [0, 1] (0 if no relevant result found)
    """
    for i, rel in enumerate(relevance_scores):
        if rel >= threshold:
            return 1.0 / (i + 1)
    return 0.0


def mean_reciprocal_rank(all_relevance_scores: List[List[int]], threshold: int = 1) -> float:
    """
    Mean Reciprocal Rank across all queries.
    
    MRR = (1/Q) Σ(q=1..Q) RR(q)
    
    Args:
        all_relevance_scores: List of relevance score lists, one per query
        threshold: Minimum relevance score to count as "relevant"
    
    Returns:
        MRR score in [0, 1]
    """
    if not all_relevance_scores:
        return 0.0
    return sum(reciprocal_rank(scores, threshold) for scores in all_relevance_scores) / len(all_relevance_scores)


def recall_at_k(relevance_scores: List[int], k: int, total_relevant: int, threshold: int = 1) -> float:
    """
    Recall at K: fraction of total relevant documents found in top K.
    
    R@K = |relevant in top K| / |total relevant|
    
    Args:
        relevance_scores: List of relevance judgments
        k: Cutoff position
        total_relevant: Total number of relevant documents for this query
        threshold: Minimum relevance score to count as "relevant"
    
    Returns:
        R@K score in [0, 1]
    """
    if total_relevant == 0:
        return 0.0
    relevant_in_k = sum(1 for r in relevance_scores[:k] if r >= threshold)
    return relevant_in_k / total_relevant


def compute_all_metrics(
    relevance_scores: List[int],
    k_values: List[int] = [1, 3, 5, 10, 20],
    total_relevant: Optional[int] = None,
    threshold: int = 1,
) -> Dict[str, float]:
    """
    Compute all metrics for a single query.
    
    Args:
        relevance_scores: List of relevance judgments for returned results
        k_values: List of K values for @K metrics
        total_relevant: Total relevant docs (for recall); if None, uses count from relevance_scores
        threshold: Minimum relevance score for binary relevance
    
    Returns:
        Dictionary of metric_name → value
    """
    if total_relevant is None:
        total_relevant = sum(1 for r in relevance_scores if r >= threshold)
    
    metrics = {}
    
    for k in k_values:
        metrics[f"NDCG@{k}"] = ndcg_at_k(relevance_scores, k)
        metrics[f"P@{k}"] = precision_at_k(relevance_scores, k, threshold)
        metrics[f"R@{k}"] = recall_at_k(relevance_scores, k, total_relevant, threshold)
    
    metrics["AP"] = average_precision(relevance_scores, threshold)
    metrics["RR"] = reciprocal_rank(relevance_scores, threshold)
    
    return metrics


def compute_batch_metrics(
    all_relevance_scores: List[List[int]],
    k_values: List[int] = [1, 3, 5, 10, 20],
    total_relevants: Optional[List[int]] = None,
    threshold: int = 1,
) -> Dict[str, float]:
    """
    Compute averaged metrics across all queries.
    
    Args:
        all_relevance_scores: List of relevance score lists, one per query
        k_values: K values for @K metrics
        total_relevants: Per-query total relevant counts; if None, computed from scores
        threshold: Minimum relevance score for binary relevance
    
    Returns:
        Dictionary of averaged metric_name → value
    """
    if not all_relevance_scores:
        return {}
    
    n = len(all_relevance_scores)
    batch_metrics = {}
    
    for i, scores in enumerate(all_relevance_scores):
        tr = total_relevants[i] if total_relevants else None
        query_metrics = compute_all_metrics(scores, k_values, tr, threshold)
        for key, value in query_metrics.items():
            batch_metrics[key] = batch_metrics.get(key, 0.0) + value
    
    # Average
    for key in batch_metrics:
        batch_metrics[key] /= n
    
    # Also add MAP and MRR explicitly
    batch_metrics["MAP"] = mean_average_precision(all_relevance_scores, threshold)
    batch_metrics["MRR"] = mean_reciprocal_rank(all_relevance_scores, threshold)
    
    return batch_metrics


# ── Self-test ──
if __name__ == "__main__":
    print("=== IR Metrics Self-Test ===\n")
    
    # Test case: query with known relevance
    scores = [3, 2, 0, 1, 0, 2, 0, 0, 1, 0]
    print(f"Relevance scores: {scores}")
    
    m = compute_all_metrics(scores, k_values=[1, 3, 5, 10])
    for k, v in sorted(m.items()):
        print(f"  {k}: {v:.4f}")
    
    # Test batch
    all_scores = [
        [3, 2, 0, 1, 0],
        [0, 0, 1, 2, 3],
        [2, 2, 2, 0, 0],
    ]
    print(f"\nBatch ({len(all_scores)} queries):")
    bm = compute_batch_metrics(all_scores, k_values=[1, 3, 5])
    for k, v in sorted(bm.items()):
        print(f"  {k}: {v:.4f}")
    
    print("\n✓ All metrics computed successfully")
