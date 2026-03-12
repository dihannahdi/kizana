import json

with open('queries_10k_eval.json', encoding='utf-8') as f:
    qs = json.load(f)

def show_range(start, end, step=10):
    print(f"=== {start}-{end} (every {step}) ===")
    for i in range(start, min(end, len(qs)), step):
        print(f"{i}: {qs[i]['text'][:80]}")
    print()

show_range(9720, 10030, 10)
