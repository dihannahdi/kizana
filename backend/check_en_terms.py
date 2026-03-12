import re

with open('src/query_translator.rs', 'r', encoding='utf-8') as f:
    content = f.read()

keys = re.findall(r'for key in &\[(.*?)\]', content)
all_keys = set()
for k_group in keys:
    for m in re.findall(r'"([^"]+)"', k_group):
        all_keys.add(m)

en_check = [
    'ruling','prayer','ablution','fasting','congregation','prostration',
    'eclipse','istisqa','funeral','eid','ghusl','janabah','impurity',
    'purification','divorce','inheritance','lease','rent','gold','silk',
    'abortion','vaping','photography','organ','donation','ivf','stocks',
    'beard','hijab','niqab','smoking','cloning','insurance','cryptocurrency',
    'alcohol','pork','gambling','music','conditions','pillars','obligatory',
    'travel','sick','women','men','traveler','resident','makeup','time',
    'direction','qibla','missed','night','rain','charity','alms','tithe',
    'pilgrimage','hajj','umrah','zakah','zakat','marriage','wedding',
    'dowry','custody','breastfeeding','waiting period','guardian',
    'selling','buying','trade','interest','usury','partnership','loan',
    'debt','contract','pledge','will','testament','endowment','gift',
    'theft','murder','apostasy','war','jihad','defense','treason',
    'monotheism','polytheism','faith','creed','innovation','predestination',
    'intercession','repentance','sincerity','patience','gratitude',
    'commentary','verse','chapter','hadith','narration','chain',
    'manners','ethics','morals','backbiting','slander','lying'
]
for t in sorted(en_check):
    status = 'MAPPED' if t in all_keys else 'MISSING'
    print(f'{t}: {status}')
