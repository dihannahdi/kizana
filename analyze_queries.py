import json
with open('queries_10k_eval.json', encoding='utf-8') as f:
    qs = json.load(f)
print(f'Total: {len(qs)}')
# Sample ranges 1000-2500
print("=== 1000-2500 range (every 50) ===")
for i in range(1000, 2500, 50):
    print(f"{i}: {qs[i]['text'][:80]}")
print()
print("=== 2500-3000 range (every 50) ===")
for i in range(2500, 3000, 50):
    print(f"{i}: {qs[i]['text'][:80]}")
print()
print("=== 7500-8500 range (every 30) ===")
for i in range(7500, 8500, 30):
    print(f"{i}: {qs[i]['text'][:80]}")
print()
print("=== 8500-10000 range (every 30) ===")
for i in range(8500, 10000, 30):
    print(f"{i}: {qs[i]['text'][:80]}")
