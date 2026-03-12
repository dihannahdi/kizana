import requests, json
BASE = "http://localhost:8080"
EMAIL = "eval_admin@bahtsulmasail.tech"
PASSWORD = "EvalTest2026"
r = requests.post(f"{BASE}/api/auth/login", json={"email": EMAIL, "password": PASSWORD}, timeout=30)
token = r.json()["token"]
headers = {"Authorization": f"Bearer {token}"}

with open("/opt/kizana/backend/queries_10k_eval.json") as f:
    all_qs = json.load(f)

# Sample every 25 from 1000-2800
targets = list(range(1000, 2800, 25))
payload = [{"id": str(i), "text": all_qs[i]["text"]} for i in targets]

results = requests.post(f"{BASE}/api/eval/batch", json={"queries": payload}, headers=headers, timeout=300).json()["results"]
zeros = [(r["id"], all_qs[int(r["id"])]["text"][:70]) for r in results if r["num_results"] == 0]
low = [(r["id"], r["num_results"], all_qs[int(r["id"])]["text"][:70]) for r in results if 0 < r["num_results"] < 5]
print(f"Range 1000-2800: {len(zeros)} zeros | {len(low)} low")
for z in zeros: print(f"  ZERO {z[0]}: {z[1]}")
for l in low: print(f"  LOW({l[1]}) {l[0]}: {l[2]}")
