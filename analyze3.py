import json
with open('queries_10k_eval.json', encoding='utf-8') as f:
    qs = json.load(f)

def show_range(start, end, step=25):
    print(f"=== {start}-{end} (every {step}) ===")
    for i in range(start, end, step):
        print(f"{i}: {qs[i]['text'][:80]}")
    print()

show_range(0, 500, 25)
show_range(500, 1000, 25)
