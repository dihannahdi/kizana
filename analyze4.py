import json

with open('queries_10k_eval.json', encoding='utf-8') as f:
    qs = json.load(f)

def show_range(start, end, step=20):
    print(f"=== {start}-{end} (every {step}) ===")
    for i in range(start, end, step):
        print(f"{i}: {qs[i]['text'][:80]}")
    print()

# Look at ranges with dense "pengertian X / hukum X" patterns
show_range(3070, 3200, 5)
show_range(3200, 3500, 10)
show_range(4200, 4500, 10)
