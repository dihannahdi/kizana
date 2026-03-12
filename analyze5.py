import json

with open('queries_10k_eval.json', encoding='utf-8') as f:
    qs = json.load(f)

def show_range(start, end, step=10):
    print(f"=== {start}-{end} (every {step}) ===")
    for i in range(start, end, step):
        print(f"{i}: {qs[i]['text'][:80]}")
    print()

# Dense scan of hard sections
show_range(8550, 8750, 10)      # "apa yang membatalkan X" patterns
show_range(9200, 9530, 15)      # Arabic queries + "pembagian waris" 
show_range(9530, 9720, 10)      # "pembagian waris menurut kitab X"
