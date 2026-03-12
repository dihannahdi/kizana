import json, os, re
os.chdir('d:/nahdi/bahtsulmasail/backend')

with open('src/query_translator.rs', 'r', encoding='utf-8') as f:
    src = f.read()

# Extract all quoted strings that look like Indonesian/English terms
mapped = set(re.findall(r'"([a-zA-Z\' ]{2,30})"', src))
mapped_lower = {t.lower() for t in mapped}

queries = json.load(open('queries_10k.json', 'r', encoding='utf-8'))
target_cats = ('campuran', 'awam', 'skenario', 'kontemporer', 'tech', 'food', 'medical', 'edge')
campuran = [q['text'] for q in queries if q['category'] in target_cats]
words = set()
for q in campuran:
    for w in re.findall(r'[a-zA-Z]+', q.lower()):
        if len(w) > 2:
            words.add(w)

stops = {'yang','dan','atau','di','ke','dari','untuk','dengan','ada','itu','ini',
         'apa','apakah','gimana','kenapa','gak','sih','boleh','haram','halal',
         'hukumnya','hukum','bisa','jadi','dalam','tanpa','pakai','pake','buat',
         'sama','cara','tapi','terus','dosa','kalau','bagi','kan','gaji','kerja',
         'tidak','masuk','jika','ketika','lagi','baru','sudah','belum','mau',
         'punya','orang','uang','jual','beli','besar','kecil','banyak','sedikit',
         'rumah','lain','setelah','sebelum','saat','waktu','pergi','datang','makan',
         'minum','lihat','baca','tulis','kirim','terima','bayar','ambil','kasih',
         'tahu','lewat','bisa','masih','dulu','nanti','saja','sering','selalu',
         'pernah','akan','sangat','lebih','kurang','cukup','juga','lalu','maka',
         'oleh','pada','bisa','pun','aja','dong','deh','kok','loh','yah','nah',
         'the','and','what','how','can','about','with','does','for','are','has',
         'this','that','there','their','they','them','was','were','been','being',
         'have','had','having','would','could','should','will','shall','may','might',
         'must','need','used','let','make','take','give','keep','put','set','run',
         'get','got','say','said','know','think','come','see','look','want','use',
         'find','tell','ask','work','seem','feel','try','leave','call','turn',
         'may','also','back','even','still','well','just','only','very','often',
         'all','each','every','both','few','more','most','other','some','such',
         'than','too','any','new','old','first','last','long','great','little',
         'own','sure','thing','many','well','between','after','before','under',
         'over','through','where','when','while','during'}

unmapped = sorted(words - mapped_lower - stops)
print(f'Total words from target categories: {len(words)}')
print(f'Already mapped: {len(words & mapped_lower)}')
print(f'Unmapped (excl stops): {len(unmapped)}')
print()
print('Unmapped terms worth mapping:')
for w in unmapped:
    print(f'  {w}')
