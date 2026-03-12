#!/usr/bin/env python3
"""
Deploy v15 to VPS and re-run evaluation.
Steps:
1. SCP source code to VPS
2. Build on VPS
3. Delete old Tantivy index (forces re-index with normalized Arabic)
4. Restart backend
5. Wait for re-indexing to complete
6. Run v15 evaluation
"""
import subprocess
import time
import sys
import os
import requests

os.chdir(os.path.dirname(os.path.abspath(__file__)))

VPS = "root@31.97.223.169"
BACKEND_PATH = "/opt/kizana/backend"
INDEX_PATH = "/opt/kizana/tantivy_index"
API_URL = "https://bahtsulmasail.tech/api"

def run_ssh(cmd, timeout=600):
    """Run command on VPS via SSH"""
    full = f'ssh {VPS} "{cmd}"'
    print(f"  SSH> {cmd}")
    result = subprocess.run(full, shell=True, capture_output=True, text=True, timeout=timeout)
    if result.stdout.strip():
        print(f"  OUT> {result.stdout.strip()[:200]}")
    if result.returncode != 0 and result.stderr.strip():
        print(f"  ERR> {result.stderr.strip()[:200]}")
    return result

def run_scp(local, remote):
    """Copy file to VPS"""
    cmd = f'scp -r "{local}" {VPS}:{remote}'
    print(f"  SCP> {local} → {remote}")
    return subprocess.run(cmd, shell=True, capture_output=True, text=True, timeout=120)

def check_api():
    """Check if API is responding"""
    try:
        r = requests.post(f"{API_URL}/auth/login", json={
            "email": "eval_admin@bahtsulmasail.tech",
            "password": "EvalTest2026"
        }, timeout=10)
        return r.status_code == 200
    except Exception:
        return False

def check_index_status(token):
    """Check if the index is built"""
    try:
        r = requests.post(f"{API_URL}/query", json={
            "query": "test"
        }, headers={"Authorization": f"Bearer {token}"}, timeout=30)
        return r.status_code == 200 and len(r.json().get("results", [])) > 0
    except Exception:
        return False

def main():
    print("=" * 70)
    print("KIZANA v15 DEPLOYMENT")
    print("=" * 70)
    
    # Step 1: Copy source
    print("\n[1/5] Copying source code to VPS...")
    src_dir = os.path.join(os.path.dirname(os.path.abspath(__file__)), "src")
    result = run_scp(src_dir, f"{BACKEND_PATH}/")
    if result.returncode != 0:
        print("ERROR: SCP failed!")
        sys.exit(1)
    print("  ✓ Source copied")
    
    # Step 2: Build
    print("\n[2/5] Building on VPS (cargo build --release)...")
    result = run_ssh(f"cd {BACKEND_PATH} && source /root/.cargo/env && cargo build --release 2>&1 | tail -5", timeout=900)
    combined = (result.stdout + result.stderr).lower()
    if "error" in combined or result.returncode != 0:
        print("ERROR: Build failed!")
        print(result.stdout)
        print(result.stderr)
        sys.exit(1)
    print("  ✓ Build complete")
    
    # Step 3: Delete old index
    print("\n[3/5] Deleting old Tantivy index (forces re-index with normalized Arabic)...")
    run_ssh(f"rm -rf {INDEX_PATH}")
    print("  ✓ Old index deleted")
    
    # Step 4: Restart
    print("\n[4/5] Restarting kizana-backend...")
    run_ssh("systemctl restart kizana-backend")
    print("  ✓ Restart command sent")
    
    # Step 5: Wait for re-indexing
    print("\n[5/5] Waiting for re-indexing to complete...")
    print("  (Indexing 7,872 books with Arabic normalization...)")
    
    # Give it time to start
    time.sleep(10)
    
    # Login
    for attempt in range(60):
        try:
            r = requests.post(f"{API_URL}/auth/login", json={
                "email": "eval_admin@bahtsulmasail.tech",
                "password": "EvalTest2026"
            }, timeout=10)
            if r.status_code == 200:
                token = r.json().get("token", "")
                if token:
                    print("  ✓ API is up, logged in")
                    break
        except Exception:
            pass
        print(f"  Waiting for API... ({attempt+1}/60)")
        time.sleep(10)
    else:
        print("ERROR: API did not come up!")
        sys.exit(1)
    
    # Wait for index to be built
    for attempt in range(180):  # up to 30 min
        if check_index_status(token):
            print(f"  ✓ Index is built and serving results!")
            break
        if attempt % 6 == 0:
            # Check logs
            log_result = run_ssh("journalctl -u kizana-backend --no-pager -n 3 2>/dev/null")
        print(f"  Indexing in progress... ({attempt+1})")
        time.sleep(10)
    else:
        print("ERROR: Indexing did not complete in 30 minutes!")
        sys.exit(1)
    
    print("\n" + "=" * 70)
    print("DEPLOYMENT COMPLETE")
    print("=" * 70)
    print("Next steps:")
    print(f"  1. Run eval: python run_eval_10k.py  (save to eval_results_v15.json)")
    print(f"  2. Compare:  python compare_eval.py eval_results_10k.json eval_results_v15.json")

if __name__ == "__main__":
    main()
