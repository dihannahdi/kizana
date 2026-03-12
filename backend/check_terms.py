import re

with open('src/query_translator.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Find all single-word term_map entries
keys = re.findall(r'for key in &\[(.*?)\]', content)
all_keys = set()
for k_group in keys:
    for m in re.findall(r'"([^"]+)"', k_group):
        all_keys.add(m)

check = [
    'imam','makmum','rakaat','tasyahud','aurat','muhrim','mahram',
    'junub','rokok','tato','transplantasi','kontrasepsi','kloning',
    'operasi','wali','akad','wasiat','hibah','gadai','kutek','plester',
    'paylater','piercing','cukur','alis','jabat','merokok',
    'demonstrasi','demo','bayi','tabung','kelamin','sushi',
    'nft','metaverse','virtual','robot','pandemi','lockdown',
    'streaming','youtuber','tiktok','anime','manga','cosplay',
    'drama','korea','chatting','tinder','pubg'
]
for term in sorted(check):
    status = 'MAPPED' if term in all_keys else 'MISSING'
    print(f'{term}: {status}')

print(f'\nTotal single-word keys in term_map: {len(all_keys)}')
