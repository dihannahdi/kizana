#!/usr/bin/env python3
"""
Kizana Search — User Study Framework
=====================================
Generates user study protocols, task sets, and survey instruments
for evaluating the system with real users (required for Q1 journals
like IP&M and JASIST).

Study Design:
  - Within-subjects counterbalanced design
  - 3 conditions: Kizana Full, Google Translate + BM25, No Translation
  - N=20+ participants (Islamic studies students/scholars)
  - Measured: Task success, search time, result quality, satisfaction

Output:
  - Task sets with ground truth
  - Pre/post questionnaires (SUS + custom domain questions)
  - Analysis script templates
  - LaTeX tables for paper

Usage:
    python user_study_framework.py --generate-all --output-dir results/user_study
"""

import argparse
import json
import random
import time
from pathlib import Path
from typing import Dict, List


# ══════════════════════════════════════════════════════════════
# STUDY PROTOCOL
# ══════════════════════════════════════════════════════════════

STUDY_PROTOCOL = """
═══════════════════════════════════════════════════════════════════
USER STUDY PROTOCOL — Kizana Search Evaluation
═══════════════════════════════════════════════════════════════════

STUDY TITLE:
  Evaluating Domain-Specific Cross-Lingual Information Retrieval
  for Classical Islamic Text Collections: A User Study

ETHICS:
  This study involves human participants performing information search
  tasks. It should be reviewed by an institutional ethics committee
  (IRB/ethics board) before deployment. Participants provide informed
  consent and may withdraw at any time.

1. RESEARCH QUESTIONS
─────────────────────
  RQ-U1: Does Kizana's domain-specific query translation lead to
         higher task success rates compared to standard machine
         translation?

  RQ-U2: Do users perceive the domain-translated results as more
         relevant and trustworthy?

  RQ-U3: How does search efficiency (time-to-find) differ across
         systems?

  RQ-U4: What is the overall user satisfaction with Kizana as
         measured by the System Usability Scale (SUS)?

2. STUDY DESIGN
───────────────
  Type:           Within-subjects, counterbalanced
  Conditions:     3 (Kizana Full, Google Translate + BM25, No Translation)
  Participants:   N ≥ 20
  Latin Square:   6 orderings (3! = 6), balanced assignment

  Counterbalance Matrix:
    Group 1: Kizana → GT+BM25 → NoTrans  (tasks A,B,C)
    Group 2: Kizana → NoTrans → GT+BM25  (tasks A,C,B)
    Group 3: GT+BM25 → Kizana → NoTrans  (tasks B,A,C)
    Group 4: GT+BM25 → NoTrans → Kizana  (tasks B,C,A)
    Group 5: NoTrans → Kizana → GT+BM25  (tasks C,A,B)
    Group 6: NoTrans → GT+BM25 → Kizana  (tasks C,B,A)

3. PARTICIPANT CRITERIA
───────────────────────
  Inclusion:
    • Currently enrolled in or graduated from Islamic studies program
    • Proficient in Bahasa Indonesia (reading + writing)
    • Ability to read Arabic text (at least basic level)
    • Experience with searching for Islamic legal references

  Stratification Target:
    • 10 pesantren students/graduates
    • 5 university Islamic studies students
    • 5 Islamic scholars/teachers (ustadz/kiai)

4. PROCEDURE
────────────
  Total Duration: ~45 minutes per participant

  a) Pre-study (5 min):
     - Informed consent form
     - Demographic questionnaire
     - Search experience self-assessment

  b) Training (5 min):
     - Brief introduction to each system condition
     - 1 practice task per condition (not scored)

  c) Main Tasks (25 min, ~8 min per condition):
     - 3 search tasks per condition (9 total)
     - Each task: find a specific ruling/reference in the kitab
     - Time limit: 3 minutes per task
     - Record: task success (binary), time, # queries tried

  d) Post-task Ratings (5 min):
     - After each condition: rate result quality (1-5 Likert)
     - Perceived relevance of top results
     - Confidence in the results found

  e) Post-study (5 min):
     - System Usability Scale (SUS) for Kizana
     - Preference ranking of the 3 conditions
     - Open-ended feedback

5. METRICS
──────────
  Objective:
    • Task Success Rate (TSR): Binary (found / not found)
    • Time-to-Find (TTF): Seconds until correct result found
    • Query Reformulation Rate (QRR): # queries per task

  Subjective:
    • Result Quality Rating (RQR): 5-point Likert
    • System Usability Scale (SUS): 10-item standardized
    • Net Promoter Score (NPS): Would recommend?
    • Preference Ranking: 1st, 2nd, 3rd

6. ANALYSIS PLAN
────────────────
  • Repeated-measures ANOVA for TSR, TTF, RQR (3 conditions)
  • Post-hoc pairwise comparisons with Bonferroni correction
  • Wilcoxon signed-rank test for Likert data
  • Descriptive statistics for SUS scores
  • Thematic analysis for open-ended feedback

7. POWER ANALYSIS
─────────────────
  For N=20, 3 conditions, α=0.05, power=0.80:
  Detectable effect size: f ≈ 0.35 (medium-to-large)
  This is appropriate given the expected performance gap
  between domain-specific and generic translation.

  For smaller effects, N=30 recommended (detectable f ≈ 0.28).
"""


# ══════════════════════════════════════════════════════════════
# SEARCH TASKS
# ══════════════════════════════════════════════════════════════

SEARCH_TASKS = [
    # Task Set A (used with one condition)
    {
        "set": "A",
        "tasks": [
            {
                "id": "A1",
                "instruction_id": "Cari hukum shalat sambil menggendong anak kecil menurut mazhab Syafi'i",
                "instruction_en": "Find the ruling on praying while carrying a small child according to the Shafi'i school",
                "expected_finding": "Shalat carrying a child is valid with conditions",
                "gold_concepts": ["حمل الطفل في الصلاة", "حمل النجاسة"],
                "difficulty": "easy",
            },
            {
                "id": "A2",
                "instruction_id": "Temukan pembahasan tentang hukum jual beli dengan akad salam (pesanan)",
                "instruction_en": "Find the discussion on the ruling of salam (forward) sale contracts",
                "expected_finding": "Salam is permitted with specific conditions",
                "gold_concepts": ["بيع السلم", "السلف"],
                "difficulty": "medium",
            },
            {
                "id": "A3",
                "instruction_id": "Cari dalil tentang wajibnya mahar dalam pernikahan dan batasannya",
                "instruction_en": "Find evidence on the obligation of mahr in marriage and its limits",
                "expected_finding": "Mahr is wajib, minimum amount discussed",
                "gold_concepts": ["المهر", "الصداق", "أقل المهر"],
                "difficulty": "hard",
            },
        ],
    },
    # Task Set B
    {
        "set": "B",
        "tasks": [
            {
                "id": "B1",
                "instruction_id": "Cari pembahasan tentang hukum puasa bagi ibu hamil dan menyusui",
                "instruction_en": "Find the discussion on fasting rulings for pregnant and breastfeeding women",
                "expected_finding": "Permitted to break fast, but fidyah/qadha required",
                "gold_concepts": ["الحامل والمرضع", "إفطار", "فدية"],
                "difficulty": "easy",
            },
            {
                "id": "B2",
                "instruction_id": "Temukan hukum transaksi dengan menggunakan mata uang kripto",
                "instruction_en": "Find rulings on transactions using cryptocurrency",
                "expected_finding": "Contemporary discussion on digital currencies",
                "gold_concepts": ["العملة الرقمية", "النقود", "المعاملات المالية"],
                "difficulty": "hard",
            },
            {
                "id": "B3",
                "instruction_id": "Cari syarat-syarat sahnya shalat Jumat",
                "instruction_en": "Find the conditions for valid Friday prayer",
                "expected_finding": "Number requirements, location conditions",
                "gold_concepts": ["صلاة الجمعة", "شروط", "العدد"],
                "difficulty": "medium",
            },
        ],
    },
    # Task Set C
    {
        "set": "C",
        "tasks": [
            {
                "id": "C1",
                "instruction_id": "Cari pembahasan tentang hukum tayammum dengan debu dinding",
                "instruction_en": "Find the discussion on tayammum using wall dust",
                "expected_finding": "Difference of opinion on clean earth types",
                "gold_concepts": ["التيمم", "الصعيد", "التراب"],
                "difficulty": "easy",
            },
            {
                "id": "C2",
                "instruction_id": "Temukan pembahasan tentang hukum cerai (khulu') atas permintaan istri",
                "instruction_en": "Find the discussion on khulu' (wife-initiated divorce)",
                "expected_finding": "Wife may request khulu' with compensation",
                "gold_concepts": ["الخلع", "فسخ النكاح"],
                "difficulty": "medium",
            },
            {
                "id": "C3",
                "instruction_id": "Cari pendapat ulama tentang zakat profesi / penghasilan",
                "instruction_en": "Find scholarly opinions on professional income zakat",
                "expected_finding": "Contemporary ijtihad on income zakat",
                "gold_concepts": ["زكاة الدخل", "زكاة المال", "النصاب"],
                "difficulty": "hard",
            },
        ],
    },
]


# ══════════════════════════════════════════════════════════════
# QUESTIONNAIRES
# ══════════════════════════════════════════════════════════════

SUS_QUESTIONNAIRE = [
    {"id": "SUS1", "text": "I think I would like to use this system frequently", "text_id": "Saya rasa saya ingin menggunakan sistem ini secara rutin"},
    {"id": "SUS2", "text": "I found this system unnecessarily complex", "text_id": "Saya merasa sistem ini terlalu kompleks"},
    {"id": "SUS3", "text": "I thought this system was easy to use", "text_id": "Saya pikir sistem ini mudah digunakan"},
    {"id": "SUS4", "text": "I think I would need technical support to use this system", "text_id": "Saya rasa saya membutuhkan bantuan teknis untuk menggunakan sistem ini"},
    {"id": "SUS5", "text": "I found the various functions well integrated", "text_id": "Saya merasa berbagai fungsi terintegrasi dengan baik"},
    {"id": "SUS6", "text": "I thought there was too much inconsistency", "text_id": "Saya merasa ada terlalu banyak ketidakkonsistenan"},
    {"id": "SUS7", "text": "I imagine most people would learn to use this quickly", "text_id": "Saya bayangkan kebanyakan orang akan cepat belajar menggunakan ini"},
    {"id": "SUS8", "text": "I found this system very cumbersome to use", "text_id": "Saya merasa sistem ini sangat merepotkan"},
    {"id": "SUS9", "text": "I felt very confident using this system", "text_id": "Saya merasa sangat percaya diri menggunakan sistem ini"},
    {"id": "SUS10", "text": "I needed to learn a lot before using this system", "text_id": "Saya perlu belajar banyak sebelum menggunakan sistem ini"},
]

DOMAIN_QUESTIONNAIRE = [
    {
        "id": "DQ1",
        "text": "The search results were relevant to my Islamic legal question",
        "text_id": "Hasil pencarian relevan dengan pertanyaan hukum Islam saya",
        "scale": "1-5 Likert (Strongly Disagree → Strongly Agree)",
    },
    {
        "id": "DQ2",
        "text": "I could find authoritative kitab references easily",
        "text_id": "Saya dapat menemukan referensi kitab yang otoritatif dengan mudah",
        "scale": "1-5 Likert",
    },
    {
        "id": "DQ3",
        "text": "The system understood my query intent correctly",
        "text_id": "Sistem memahami maksud pertanyaan saya dengan benar",
        "scale": "1-5 Likert",
    },
    {
        "id": "DQ4",
        "text": "I trust the search results for academic/fatwa reference",
        "text_id": "Saya percaya hasil pencarian ini untuk rujukan akademik/fatwa",
        "scale": "1-5 Likert",
    },
    {
        "id": "DQ5",
        "text": "The Arabic text snippets were helpful for verification",
        "text_id": "Cuplikan teks Arab membantu untuk verifikasi",
        "scale": "1-5 Likert",
    },
    {
        "id": "DQ6",
        "text": "How would you rate overall result quality?",
        "text_id": "Bagaimana penilaian Anda terhadap kualitas hasil keseluruhan?",
        "scale": "1-5 (Very Poor → Excellent)",
    },
]

DEMOGRAPHIC_QUESTIONS = [
    {"id": "D1", "text": "Age range", "options": ["18-24", "25-34", "35-44", "45-54", "55+"]},
    {"id": "D2", "text": "Educational background", "options": ["Pesantren", "Univ. Islamic Studies", "Both", "Other"]},
    {"id": "D3", "text": "Arabic proficiency", "options": ["Beginner", "Intermediate", "Advanced", "Native"]},
    {"id": "D4", "text": "Frequency of kitab consultation", "options": ["Daily", "Weekly", "Monthly", "Rarely"]},
    {"id": "D5", "text": "Current digital search tools used", "options": ["Shamela", "Google", "Kizana", "None", "Other"]},
]


def compute_sus_score(responses: List[int]) -> float:
    """
    Compute SUS score from 10 responses (1-5 Likert scale).
    Odd items: score - 1
    Even items: 5 - score
    Sum × 2.5 = SUS score (0-100)
    """
    if len(responses) != 10:
        return 0.0
    
    adjusted = []
    for i, resp in enumerate(responses):
        if i % 2 == 0:  # Odd items (0-indexed even)
            adjusted.append(resp - 1)
        else:  # Even items
            adjusted.append(5 - resp)
    
    return sum(adjusted) * 2.5


def generate_participant_assignment(n_participants: int) -> List[Dict]:
    """Generate counterbalanced condition assignments for participants."""
    orderings = [
        ["kizana", "gt_bm25", "no_trans"],
        ["kizana", "no_trans", "gt_bm25"],
        ["gt_bm25", "kizana", "no_trans"],
        ["gt_bm25", "no_trans", "kizana"],
        ["no_trans", "kizana", "gt_bm25"],
        ["no_trans", "gt_bm25", "kizana"],
    ]
    
    task_sets = ["A", "B", "C"]
    
    assignments = []
    for pid in range(1, n_participants + 1):
        ordering = orderings[(pid - 1) % len(orderings)]
        # Rotate task sets based on participant ID
        offset = (pid - 1) // len(orderings)
        rotated_tasks = task_sets[offset % 3:] + task_sets[:offset % 3]
        
        assignment = {
            "participant_id": f"P{pid:03d}",
            "group": ((pid - 1) % len(orderings)) + 1,
            "condition_order": ordering,
            "task_set_order": rotated_tasks,
        }
        assignments.append(assignment)
    
    return assignments


def generate_data_collection_form(participant: Dict) -> str:
    """Generate a text-based data collection form for a participant."""
    form = []
    form.append(f"PARTICIPANT: {participant['participant_id']}")
    form.append(f"GROUP: {participant['group']}")
    form.append(f"DATE: __________")
    form.append("")
    form.append("DEMOGRAPHICS:")
    for dq in DEMOGRAPHIC_QUESTIONS:
        form.append(f"  {dq['id']}. {dq['text']}: __________")
    form.append("")
    
    for i, (condition, task_set) in enumerate(
        zip(participant["condition_order"], participant["task_set_order"])
    ):
        form.append(f"\n{'='*50}")
        form.append(f"SESSION {i+1}: {condition.upper()} (Task Set {task_set})")
        form.append(f"{'='*50}")
        
        tasks = next(ts for ts in SEARCH_TASKS if ts["set"] == task_set)
        for task in tasks["tasks"]:
            form.append(f"\n  Task {task['id']}: {task['instruction_id']}")
            form.append(f"  Success (Y/N):   ____")
            form.append(f"  Time (seconds):  ____")
            form.append(f"  Queries tried:   ____")
        
        form.append(f"\n  Post-condition ratings (1-5):")
        for dq in DOMAIN_QUESTIONNAIRE:
            form.append(f"    {dq['id']}: ____  ({dq['text_id']})")
    
    form.append(f"\n{'='*50}")
    form.append("POST-STUDY: System Usability Scale (1-5)")
    form.append(f"{'='*50}")
    for sq in SUS_QUESTIONNAIRE:
        form.append(f"  {sq['id']}: ____  ({sq['text_id']})")
    
    form.append(f"\n  Preference ranking (1=best, 3=worst):")
    form.append(f"    Kizana:     ____")
    form.append(f"    GT+BM25:    ____")
    form.append(f"    No Trans:   ____")
    
    form.append(f"\n  Open feedback: ____________________________________________")
    
    return "\n".join(form)


def generate_analysis_template() -> str:
    """Generate a Python analysis script template for user study data."""
    return '''#!/usr/bin/env python3
"""
User Study Data Analysis Template
===================================
Fill in the data arrays below with collected results,
then run this script to compute statistics for the paper.
"""

import numpy as np
from scipy import stats

# ── RAW DATA ──
# Fill in after data collection. Each row = 1 participant.
# Columns: [Kizana, GT+BM25, NoTrans]

# Task Success Rate (proportion of 3 tasks completed per condition)
tsr_data = np.array([
    # [kizana, gt_bm25, no_trans]
    # [0.67, 0.33, 0.00],  # P001
    # [1.00, 0.67, 0.33],  # P002
    # ... add participant data
])

# Time-to-Find (average seconds per task per condition)
ttf_data = np.array([
    # [kizana, gt_bm25, no_trans]
])

# Result Quality Rating (average 1-5 Likert per condition)
rqr_data = np.array([
    # [kizana, gt_bm25, no_trans]
])

# SUS scores (only for Kizana)
sus_scores = np.array([
    # participant SUS scores
])


def analyze():
    if len(tsr_data) == 0:
        print("No data yet. Fill in the arrays above after data collection.")
        return
    
    print("=" * 60)
    print("USER STUDY RESULTS ANALYSIS")
    print("=" * 60)
    
    # ── Task Success Rate ──
    print("\\n1. Task Success Rate (TSR)")
    for i, cond in enumerate(["Kizana", "GT+BM25", "NoTrans"]):
        vals = tsr_data[:, i]
        print(f"   {cond:12s}: M={np.mean(vals):.3f}, SD={np.std(vals, ddof=1):.3f}")
    
    # Repeated-measures ANOVA (using Friedman for non-parametric)
    stat, p = stats.friedmanchisquare(tsr_data[:,0], tsr_data[:,1], tsr_data[:,2])
    print(f"   Friedman: χ²={stat:.3f}, p={p:.4f}")
    
    # Pairwise Wilcoxon
    pairs = [(0,1,"Kizana vs GT+BM25"), (0,2,"Kizana vs NoTrans"), (1,2,"GT+BM25 vs NoTrans")]
    for i, j, label in pairs:
        stat, p = stats.wilcoxon(tsr_data[:,i], tsr_data[:,j])
        print(f"   Wilcoxon {label}: W={stat:.1f}, p={p:.4f}")
    
    # ── Time-to-Find ──
    if len(ttf_data) > 0:
        print("\\n2. Time-to-Find (TTF)")
        for i, cond in enumerate(["Kizana", "GT+BM25", "NoTrans"]):
            vals = ttf_data[:, i]
            print(f"   {cond:12s}: M={np.mean(vals):.1f}s, SD={np.std(vals, ddof=1):.1f}s")
    
    # ── SUS Score ──
    if len(sus_scores) > 0:
        print("\\n3. System Usability Scale (Kizana)")
        print(f"   Mean SUS: {np.mean(sus_scores):.1f} (SD={np.std(sus_scores, ddof=1):.1f})")
        print(f"   Interpretation: ", end="")
        mean_sus = np.mean(sus_scores)
        if mean_sus >= 80.3: print("Excellent (A)")
        elif mean_sus >= 68: print("Good (B-C)")
        elif mean_sus >= 51: print("OK (D)")
        else: print("Poor (F)")


if __name__ == "__main__":
    analyze()
'''


def main():
    parser = argparse.ArgumentParser(description="Kizana User Study Framework Generator")
    parser.add_argument("--output-dir", default=str(Path(__file__).parent / "results" / "user_study"))
    parser.add_argument("--n-participants", type=int, default=24,
                        help="Number of participants (should be multiple of 6)")
    parser.add_argument("--generate-all", action="store_true")
    args = parser.parse_args()
    
    output_dir = Path(args.output_dir)
    output_dir.mkdir(parents=True, exist_ok=True)
    
    print(f"\n{'='*70}")
    print("KIZANA SEARCH — USER STUDY FRAMEWORK GENERATOR")
    print(f"{'='*70}")
    
    # ── 1. Protocol ──
    protocol_file = output_dir / "study_protocol.txt"
    with open(protocol_file, "w", encoding="utf-8") as f:
        f.write(STUDY_PROTOCOL)
    print(f"  Protocol:     {protocol_file}")
    
    # ── 2. Task Sets ──
    tasks_file = output_dir / "task_sets.json"
    with open(tasks_file, "w", encoding="utf-8") as f:
        json.dump(SEARCH_TASKS, f, ensure_ascii=False, indent=2)
    print(f"  Task Sets:    {tasks_file}")
    
    # ── 3. Questionnaires ──
    questionnaires = {
        "demographics": DEMOGRAPHIC_QUESTIONS,
        "domain_specific": DOMAIN_QUESTIONNAIRE,
        "sus": SUS_QUESTIONNAIRE,
    }
    q_file = output_dir / "questionnaires.json"
    with open(q_file, "w", encoding="utf-8") as f:
        json.dump(questionnaires, f, ensure_ascii=False, indent=2)
    print(f"  Questionnaires: {q_file}")
    
    # ── 4. Participant Assignments ──
    assignments = generate_participant_assignment(args.n_participants)
    assign_file = output_dir / "participant_assignments.json"
    with open(assign_file, "w", encoding="utf-8") as f:
        json.dump(assignments, f, ensure_ascii=False, indent=2)
    print(f"  Assignments:  {assign_file} ({len(assignments)} participants)")
    
    # ── 5. Data Collection Forms ──
    forms_dir = output_dir / "forms"
    forms_dir.mkdir(exist_ok=True)
    for a in assignments[:6]:  # Generate forms for first 6 (one per group)
        form_text = generate_data_collection_form(a)
        form_file = forms_dir / f"form_{a['participant_id']}.txt"
        with open(form_file, "w", encoding="utf-8") as f:
            f.write(form_text)
    print(f"  Sample Forms: {forms_dir}/ (6 forms generated)")
    
    # ── 6. Analysis Template ──
    analysis_file = output_dir / "analyze_results.py"
    with open(analysis_file, "w", encoding="utf-8") as f:
        f.write(generate_analysis_template())
    print(f"  Analysis:     {analysis_file}")
    
    # ── 7. LaTeX Description ──
    latex_file = output_dir / "user_study_tables.tex"
    with open(latex_file, "w", encoding="utf-8") as f:
        f.write("% User Study Design Summary\n")
        f.write("\\begin{table}[t]\n")
        f.write("\\centering\n")
        f.write("\\caption{User study design summary.}\n")
        f.write("\\label{tab:user-study-design}\n")
        f.write("\\begin{tabular}{ll}\n")
        f.write("\\toprule\n")
        f.write("Aspect & Detail \\\\\n")
        f.write("\\midrule\n")
        f.write(f"Design & Within-subjects, counterbalanced \\\\\n")
        f.write(f"Conditions & 3 (Kizana, GT+BM25, No Translation) \\\\\n")
        f.write(f"Participants & {args.n_participants} ({args.n_participants//6} per group) \\\\\n")
        f.write(f"Tasks per condition & 3 search tasks \\\\\n")
        f.write(f"Total tasks per participant & 9 \\\\\n")
        f.write(f"Duration & $\\sim$45 minutes \\\\\n")
        f.write(f"Objective metrics & TSR, TTF, QRR \\\\\n")
        f.write(f"Subjective metrics & SUS, Likert ratings \\\\\n")
        f.write("\\bottomrule\n")
        f.write("\\end{tabular}\n")
        f.write("\\end{table}\n\n")
        
        # Task descriptions table
        f.write("\\begin{table}[t]\n")
        f.write("\\centering\n")
        f.write("\\caption{Search task examples from the user study.}\n")
        f.write("\\label{tab:search-tasks}\n")
        f.write("\\begin{tabular}{clcl}\n")
        f.write("\\toprule\n")
        f.write("Set & Task Description & Difficulty & Expected Concepts \\\\\n")
        f.write("\\midrule\n")
        
        for ts in SEARCH_TASKS:
            for task in ts["tasks"]:
                desc = task["instruction_en"]
                if len(desc) > 50:
                    desc = desc[:47] + "..."
                concepts = ", ".join(task["gold_concepts"][:2])
                f.write(f"{ts['set']} & {desc} & {task['difficulty']} & {concepts} \\\\\n")
        
        f.write("\\bottomrule\n")
        f.write("\\end{tabular}\n")
        f.write("\\end{table}\n")
    
    print(f"  LaTeX:        {latex_file}")
    
    print(f"\n{'='*70}")
    print("USER STUDY FRAMEWORK GENERATED SUCCESSFULLY")
    print(f"{'='*70}")
    print(f"\nNext steps:")
    print(f"  1. Submit protocol to ethics board / IRB for approval")
    print(f"  2. Recruit {args.n_participants} participants matching criteria")
    print(f"  3. Conduct study sessions using generated forms")
    print(f"  4. Enter data into analyze_results.py")
    print(f"  5. Run analysis and include tables in paper")


if __name__ == "__main__":
    main()
