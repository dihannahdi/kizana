import json
with open('queries_10k_eval.json', encoding='utf-8') as f:
    qs = json.load(f)

def show_range(start, end, step=25):
    print(f"=== {start}-{end} (every {step}) ===")
    for i in range(start, end, step):
        print(f"{i}: {qs[i]['text'][:80]}")
    print()

show_range(7000, 7500, 25)
show_range(3000, 3500, 25)
show_range(3500, 4000, 25)
show_range(4000, 4500, 25)
show_range(4500, 5000, 25)
