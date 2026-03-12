#!/usr/bin/env python3
"""Comprehensive audit of v12 eval results — checks all previously failing queries."""
import json
import sys

def load_results(path):
    with open(path, 'r', encoding='utf-8') as f:
        return json.load(f)

def check_query(results, query_id, expected_terms, desc):
    """Check if a specific query's results contain expected Arabic terms."""
    for r in results.get("by_query", []):
        if r["id"] == query_id:
            all_text = ""
            for t in r.get("top3", []):
                all_text += t.get("title", "") + " " + str(t.get("snippet_len", 0)) + " "
            
            terms = r.get("translated_terms", [])
            num = r.get("num_results", 0)
            score = r.get("top_score", 0)
            domain = r.get("domain", "")
            
            found = []
            missing = []
            for et in expected_terms:
                if any(et in t for t in terms):
                    found.append(et)
                else:
                    missing.append(et)
            
            status = "✅" if num > 0 and len(found) > 0 else "❌"
            
            print(f"\n{status} [{query_id}] \"{r['text']}\"")
            print(f"   Domain: {domain} | Results: {num} | Top Score: {score:.1f}")
            print(f"   Terms ({len(terms)}): {', '.join(terms[:8])}")
            if found:
                print(f"   ✓ Found: {', '.join(found)}")
            if missing:
                print(f"   ✗ Missing: {', '.join(missing)}")
            
            # Show top 3 titles
            for i, t3 in enumerate(r.get("top3", [])[:3]):
                print(f"   #{i+1}: [{t3.get('score',0):.1f}] {t3.get('title','')[:80]} ({t3.get('book','')})")
            
            return num > 0, len(found), len(missing)
    
    print(f"\n❌ [{query_id}] NOT FOUND in results — {desc}")
    return False, 0, len(expected_terms)

def main():
    results = load_results("eval_results_v12.json")
    
    print("=" * 80)
    print("KIZANA SEARCH v12 — COMPREHENSIVE AUDIT")
    print("=" * 80)
    
    # Summary stats
    summary = {k: v for k, v in results.items() if k != "results"}
    print(f"\nTotal Queries: {summary.get('total_queries', 0)}")
    print(f"Queries with Results: {summary.get('queries_with_results', 0)}")
    print(f"Zero Results: {summary.get('queries_no_results', 0)}")
    print(f"Avg Search Time: {summary.get('avg_search_time_ms', 0):.1f}ms")
    print(f"Avg Top Score: {summary.get('avg_top_score', 0):.1f}")
    
    print(f"\nScore Distribution:")
    for k, v in summary.get("score_distribution", {}).items():
        print(f"  {k}: {v}")
    
    print(f"\nDomain Distribution:")
    for k, v in summary.get("domain_distribution", {}).items():
        print(f"  {k}: {v}")
    
    print(f"\nTranslation Coverage: {summary.get('translation_coverage', {})}")
    
    # ═══ AUDIT PREVIOUSLY FAILING QUERIES ═══
    print("\n" + "=" * 80)
    print("AUDIT: Previously ZERO RESULT Queries")
    print("=" * 80)
    
    zero_results_fixed = 0
    zero_results_total = 3
    
    ok, _, _ = check_query(results, "hj02", ["إحرام", "ميقات"], "ihram dari miqat")
    if ok: zero_results_fixed += 1
    
    ok, _, _ = check_query(results, "ak01", ["آداب", "متعلم", "طالب"], "adab murid kepada guru")
    if ok: zero_results_fixed += 1
    
    ok, _, _ = check_query(results, "cm04", ["جراحة", "تجميل"], "operasi plastik hukumnya")
    if ok: zero_results_fixed += 1
    
    print(f"\n→ Fixed {zero_results_fixed}/{zero_results_total} zero-result queries")
    
    # ═══ AUDIT PREVIOUSLY WRONG RESULT QUERIES ═══
    print("\n" + "=" * 80)
    print("AUDIT: Previously WRONG RESULT Queries")
    print("=" * 80)
    
    wrong_fixed = 0
    wrong_total = 5
    
    ok, f, _ = check_query(results, "pu01", ["مبطلات", "مفسدات", "نواقض", "صيام"], "hal yang membatalkan puasa")
    if ok and f >= 2: wrong_fixed += 1
    
    ok, f, _ = check_query(results, "cm01", ["عورة", "لباس", "ستر"], "shalat pakai celana pendek")
    if ok and f >= 1: wrong_fixed += 1
    
    ok, f, _ = check_query(results, "cm02", ["اختلاف", "الدين", "كتابية"], "nikah beda agama")
    if ok and f >= 1: wrong_fixed += 1
    
    ok, f, _ = check_query(results, "aq08", ["صفات", "العشرون"], "sifat dua puluh allah")
    if ok and f >= 1: wrong_fixed += 1
    
    ok, f, _ = check_query(results, "cm03", ["بيع", "ما لا يملك"], "hukum dropship dalam islam")
    if ok and f >= 1: wrong_fixed += 1
    
    print(f"\n→ Fixed {wrong_fixed}/{wrong_total} wrong-result queries")
    
    # ═══ AUDIT DOMAIN DETECTION FIXES ═══
    print("\n" + "=" * 80)
    print("AUDIT: Domain Detection Fixes")
    print("=" * 80)
    
    domain_fixes = 0
    domain_total = 3
    
    for r in results.get("by_query", []):
        if r["id"] == "ib13":  # imam perempuan shalat
            d = r.get("domain", "")
            ok = "عبادات" in d
            status = "✅" if ok else "❌"
            print(f"\n{status} [ib13] \"imam perempuan shalat\" → domain: {d} (expected: عبادات)")
            if ok: domain_fixes += 1
        
        if r["id"] == "zk04":  # zakat emas
            d = r.get("domain", "")
            ok = "عبادات" in d
            status = "✅" if ok else "❌"
            print(f"\n{status} [zk04] \"zakat emas\" → domain: {d} (expected: عبادات)")
            if ok: domain_fixes += 1
        
        if r["id"] == "aq06":  # qadha qadar
            d = r.get("domain", "")
            ok = "عقيدة" in d
            status = "✅" if ok else "❌"  
            print(f"\n{status} [aq06] \"qadha qadar\" → domain: {d} (expected: عقيدة)")
            if ok: domain_fixes += 1
    
    print(f"\n→ Fixed {domain_fixes}/{domain_total} domain detection issues")
    
    # ═══ AUDIT MORPHOLOGICAL EXTRACTION ═══
    print("\n" + "=" * 80)
    print("AUDIT: Morphological Root Extraction (Novel Feature)")
    print("=" * 80)
    
    morph_queries = [
        ("pu01", "membatalkan", ["بطلان", "مبطلات", "باطل", "نواقض"]),
        ("cm06", "berwudhu", ["وضوء", "الوضوء"]),
    ]
    
    morph_ok = 0
    for qid, word, expected in morph_queries:
        ok, f, m = check_query(results, qid, expected, f"morphological: {word}")
        if ok and f >= 1: morph_ok += 1
    
    print(f"\n→ Morphological extraction working for {morph_ok}/{len(morph_queries)} queries")
    
    # ═══ FULL QUERY COVERAGE AUDIT ═══
    print("\n" + "=" * 80)
    print("FULL AUDIT: All 120 Queries")
    print("=" * 80)
    
    total = 0
    with_results = 0
    high_score = 0  # score >= 90
    has_translation = 0
    
    problems = []
    
    for r in results.get("by_query", []):
        total += 1
        num = r.get("num_results", 0)
        score = r.get("top_score", 0)
        terms = r.get("translated_terms", [])
        
        if num > 0:
            with_results += 1
        else:
            problems.append(f"  ❌ ZERO RESULTS: [{r['id']}] \"{r['text']}\"")
        
        if score >= 90:
            high_score += 1
        
        if len(terms) > 0:
            has_translation += 1
        else:
            problems.append(f"  ⚠️ NO TRANSLATION: [{r['id']}] \"{r['text']}\"")
        
        # Check for low term count (potential undertranslation)
        if len(terms) < 2 and num > 0:
            problems.append(f"  ⚠️ LOW TERMS ({len(terms)}): [{r['id']}] \"{r['text']}\"")
    
    print(f"\nResults: {with_results}/{total} queries returned results ({with_results/total*100:.1f}%)")
    print(f"High Score (≥90): {high_score}/{total} ({high_score/total*100:.1f}%)")
    print(f"Translated: {has_translation}/{total} ({has_translation/total*100:.1f}%)")
    
    if problems:
        print(f"\nRemaining Issues ({len(problems)}):")
        for p in problems:
            print(p)
    else:
        print("\n🎉 No remaining issues found!")
    
    # ═══ FINAL SCORE ═══
    print("\n" + "=" * 80)
    print("FINAL SCORECARD")
    print("=" * 80)
    print(f"  Zero Results Fixed:     {zero_results_fixed}/{zero_results_total}")
    print(f"  Wrong Results Fixed:    {wrong_fixed}/{wrong_total}")
    print(f"  Domain Detection Fixed: {domain_fixes}/{domain_total}")
    print(f"  Query Coverage:         {with_results}/{total} ({with_results/total*100:.1f}%)")
    print(f"  High Score Rate:        {high_score}/{total} ({high_score/total*100:.1f}%)")
    print(f"  Translation Coverage:   {has_translation}/{total} ({has_translation/total*100:.1f}%)")
    
    total_fixes = zero_results_fixed + wrong_fixed + domain_fixes
    total_possible = zero_results_total + wrong_total + domain_total
    print(f"\n  OVERALL FIX RATE: {total_fixes}/{total_possible} ({total_fixes/total_possible*100:.1f}%)")

if __name__ == "__main__":
    main()
