#!/usr/bin/env python3
"""Debug: inspect all result file structures."""
import json

files = [
    'baselines_20260303_163020.json',
    'external_baseline_20260303_170824.json',
]

for fname in files:
    path = f'/opt/kizana/evaluation/results/{fname}'
    with open(path) as f:
        d = json.load(f)
    print(f'\n=== {fname} ===')
    print(f'Type: {type(d).__name__}')
    if isinstance(d, dict):
        for k, v in d.items():
            if isinstance(v, dict):
                print(f'  {k} (dict): keys={list(v.keys())[:6]}')
                # Check nested
                for kk, vv in list(v.items())[:2]:
                    if isinstance(vv, dict):
                        print(f'    {kk} (dict): keys={list(vv.keys())[:6]}')
                    elif isinstance(vv, (int, float, str)):
                        print(f'    {kk}: {vv}')
            elif isinstance(v, list):
                print(f'  {k} (list): len={len(v)}')
            else:
                print(f'  {k}: {type(v).__name__} = {str(v)[:80]}')
