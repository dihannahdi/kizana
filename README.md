# Kizana: Cross-Lingual Information Retrieval for Classical Islamic Jurisprudence Texts

[![IP&M Submission](https://img.shields.io/badge/Submitted-Information%20Processing%20%26%20Management-blue)](https://www.sciencedirect.com/journal/information-processing-and-management)

> **Kizana** is a fully offline, domain-specific cross-lingual information retrieval (CLIR) system enabling multilingual access to 7,872 classical Arabic Islamic books (3.4 million table-of-contents entries) via rule-based query translation from Indonesian, English, and mixed-language queries.

---

## Key Results

| Configuration | MAP | MRR | NDCG@10 |
|---|---|---|---|
| No Translation (baseline) | 0.139 | 0.140 | 0.127 |
| **Kizana Optimized** | **0.790** | **0.794** | **0.593** |
| Google Translate + BM25 | 0.799 | 0.876 | 0.649 |

- Evaluated on **96 gold-standard queries** across 4 languages (Indonesian, English, Arabic, mixed)
- Inter-annotator agreement: κ = 0.793, Krippendorff's α = 0.739
- No statistically significant difference vs. Google Translate: *t*(95) = −1.458, *p* = 0.148
- Arabic queries: Kizana **outperforms** Google Translate (MAP 0.838 vs. 0.831)

---

## Repository Structure

```
kizana/
├── backend/          Rust (Actix-web) — search engine, BM25/Tantivy, query translation
├── frontend/         SvelteKit — bilingual UI (Indonesian/English/Arabic RTL)
├── evaluation/       Evaluation framework, gold standard dataset, ablation scripts
│   ├── gold_standard.jsonl       96 annotated queries (4 langs × 4 domains)
│   ├── evaluate.py               Main evaluation harness (MAP, MRR, NDCG)
│   ├── run_optimized.py          Optimized configuration evaluation
│   ├── run_external_baseline.py  Google Translate baseline
│   ├── run_ablation.py           10-configuration ablation study
│   ├── validate_gold_standard.py Inter-annotator agreement calculation
│   └── results/                  All evaluation output (JSON + LaTeX tables)
└── paper/            LaTeX manuscript + submission package (IP&M)
    ├── manuscript_anonymized.tex  XeLaTeX source (anonymized)
    ├── kizana_q1_paper.tex        Full paper with author details
    └── submission/                Elsevier submission package
```

---

## System Architecture

```
SvelteKit (port 3000)          → Frontend, RTL Arabic support, bilingual UI
  ↕ /api/* proxy
Actix-Web / Rust (port 8080)   → Query handler, auth, search orchestration
  ├─ Tantivy BM25               → Lexical search over 3.4M+ TOC entries
  ├─ SQLite (20GB)              → 7,872 books full text + TOC hierarchy
  ├─ Redis                      → Search result cache (1h TTL)
  └─ AI Synthesizer             → Optional LLM-based answer synthesis
```

---

## Evaluation Dataset

The gold standard dataset (`evaluation/gold_standard.jsonl`) contains **96 queries** across:

| Language | Count | Example |
|---|---|---|
| Indonesian | 24 | "hukum vaksin dalam Islam" |
| English | 24 | "ruling on photography of living beings" |
| Arabic | 24 | "حكم التصوير في الفقه الإسلامي" |
| Mixed | 24 | "boleh gak foto makhluk hidup menurut ulama" |

Domains: Ibadah (worship), Muamalat (commerce), Munakahat (family law), Aqidah (theology)

---

## Running the Evaluation

```bash
cd evaluation
pip install -r requirements.txt

# Default configuration
python evaluate.py

# Optimized configuration
python run_optimized.py

# Full ablation study (10 configurations)
python run_ablation.py

# External baseline (requires Google Translate API)
python run_external_baseline.py

# Gold standard validation (inter-annotator agreement)
python validate_gold_standard.py
```

---

## Citation

```bibtex
@article{nahdi2026kizana,
  title   = {Kizana: Cross-Lingual Information Retrieval for Classical Islamic
             Jurisprudence Texts Using Domain-Specific Query Translation},
  author  = {Nahdi, Dihan},
  journal = {Information Processing \& Management},
  year    = {2026},
  note    = {Under review}
}
```

---

## Live System

**bahtsulmasail.tech** — production deployment serving the Indonesian pesantren community.

---

*"العلم نور" — Knowledge is light.*
