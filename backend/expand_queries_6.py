#!/usr/bin/env python3
"""Final 200 queries to cross 10K."""
import json

with open("queries_10k.json", "r", encoding="utf-8") as f:
    queries = json.load(f)

seen = {q["text"].strip().lower() for q in queries}

def add(cat, text, lang="id"):
    key = text.strip().lower()
    if key not in seen:
        seen.add(key)
        queries.append({"id":"","text":text,"category":cat,"expected_domain":None,"lang":lang})

# Quick cross: "hukum X dalam keadaan Y"
states = ["darurat", "terpaksa", "lupa", "tidak tahu", "safar", "sakit keras",
    "hamil", "menyusui", "haid", "nifas", "junub", "perang"]
acts = ["shalat", "puasa", "zakat", "haji", "menikah", "jual beli",
    "potong rambut", "memakai wewangian", "makan daging",
    "minum obat", "menyembelih hewan", "bermusik",
    "memotret", "membuat patung", "meminjam uang", "mencukur jenggot"]

for s in states:
    for a in acts:
        add("keadaan2", f"hukum {a} dalam keadaan {s}")

for i, q in enumerate(queries):
    q["id"] = f"q{i+1:05d}"
print(f"Total: {len(queries)}")
with open("queries_10k.json", "w", encoding="utf-8") as f:
    json.dump(queries, f, ensure_ascii=False, indent=1)
eval_queries = [{"id": q["id"], "text": q["text"]} for q in queries]
with open("queries_10k_eval.json", "w", encoding="utf-8") as f:
    json.dump(eval_queries, f, ensure_ascii=False, indent=1)
print(f"Saved {len(queries)} queries")
