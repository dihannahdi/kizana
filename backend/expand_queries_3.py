#!/usr/bin/env python3
"""Final expansion to reach 10K+ queries."""
import json

with open("queries_10k.json", "r", encoding="utf-8") as f:
    queries = json.load(f)

seen = {q["text"].strip().lower() for q in queries}

def add(cat, text, domain=None, lang="id"):
    key = text.strip().lower()
    if key not in seen:
        seen.add(key)
        queries.append({"id":"","text":text,"category":cat,"expected_domain":domain,"lang":lang})

# ═══════════════════════════════════════════════════════════════
# A: More actor×ibadah (situational queries)
# ═══════════════════════════════════════════════════════════════

more_actors = ["pedagang", "petani", "dokter", "guru", "tentara",
    "polisi", "nelayan", "supir", "pilot", "astronot",
    "mahasiswa", "pelajar", "bayi", "balita", "remaja",
    "lansia", "difabel", "tunanetra", "tunarungu",
    "penghuni penjara", "pengungsi", "korban bencana"]

more_acts = ["shalat", "puasa", "membayar zakat mal", "menunaikan haji",
    "umrah", "berkurban", "aqiqah", "berdzikir", "membaca quran"]

for actor in more_actors:
    for act in more_acts:
        add("situational", f"hukum {actor} {act}")

# ═══════════════════════════════════════════════════════════════
# B: Food × halal status
# ═══════════════════════════════════════════════════════════════

foods = [
    "sushi", "sashimi", "steak medium rare", "foie gras", "caviar",
    "truffle", "escargot", "lobster", "kepiting raja", "tiram",
    "kerang", "cumi", "gurita", "ubur-ubur", "teripang",
    "daging buaya", "daging rusa", "daging kelinci", "daging unta",
    "daging kerbau", "daging kambing", "daging sapi", "daging ayam",
    "daging bebek", "daging merpati", "daging burung puyuh",
    "telur penyu", "susu kuda liar", "madu tawon", "propolis",
    "royal jelly", "sarang burung walet", "rumput laut",
    "jamur", "tape", "kombucha", "bir non alkohol", "wine non alkohol",
    "vanilla extract", "rhum extract", "angkak", "terasi",
    "petis udang", "kecap ikan", "miso", "natto",
    "tempeh", "tahu sumedang", "oncom", "keju",
    "yoghurt", "kefir", "coklat", "permen karet",
    "jelly dari gelatin ikan", "marshmallow",
    "gummy bear", "es krim", "whey protein",
]

for food in foods:
    add("food", f"hukum makan {food} dalam islam")
    add("food", f"apakah {food} halal")

# ═══════════════════════════════════════════════════════════════
# C: Modern technology × Islamic ruling
# ═══════════════════════════════════════════════════════════════

tech = [
    "artificial intelligence", "machine learning", "robotika",
    "smartphone", "smart home", "IoT", "drone",
    "3D printing", "biometrik", "blockchain",
    "cloud computing", "big data", "social media",
    "e-commerce", "fintech", "edtech",
    "telemedicine", "autonomous vehicle", "metaverse",
    "augmented reality", "virtual reality", "deepfake",
    "cloning", "CRISPR gene editing", "stem cell",
    "brain computer interface", "nanotechnology",
    "space tourism", "nuclear energy", "solar energy",
]

for t in tech:
    add("tech", f"hukum {t} dalam islam")
    add("tech", f"pandangan islam tentang {t}")

# ═══════════════════════════════════════════════════════════════
# D: Profession × Islamic ruling
# ═══════════════════════════════════════════════════════════════

professions = [
    "dokter", "perawat", "bidan", "apoteker",
    "hakim", "jaksa", "pengacara", "notaris",
    "guru", "dosen", "peneliti", "ilmuwan",
    "tentara", "polisi", "intel", "diplomat",
    "akuntan", "bankir", "trader", "asuransi agent",
    "seniman", "musisi", "penyanyi", "aktor",
    "model", "desainer fashion", "fotografer",
    "jurnalis", "reporter", "editor", "penulis",
    "programmer", "hacker", "youtuber", "influencer",
    "chef", "bartender", "sommelier", "barista",
    "penari", "pesulap", "badut",
    "petinju", "pegulat", "jockey",
    "supir taksi", "kurir", "pilot",
]

for prof in professions:
    add("profesi", f"hukum bekerja sebagai {prof} dalam islam")

# ═══════════════════════════════════════════════════════════════
# E: Paired comparison "X vs Y menurut islam"
# ═══════════════════════════════════════════════════════════════

pairs = [
    ("bank syariah", "bank konvensional"),
    ("asuransi syariah", "asuransi konvensional"),
    ("saham syariah", "saham konvensional"),
    ("KPR syariah", "KPR konvensional"),
    ("pegadaian syariah", "pegadaian konvensional"),
    ("obligasi syariah", "obligasi konvensional"),
    ("reksadana syariah", "reksadana konvensional"),
    ("hotel syariah", "hotel konvensional"),
    ("pariwisata halal", "pariwisata umum"),
    ("nikah sirri", "nikah resmi"),
    ("talak raj'i", "talak bain"),
    ("fasakh", "khuluk"),
    ("zakat", "sedekah"),
    ("infaq", "sedekah"),
    ("hibah", "hadiah"),
    ("wakaf", "sedekah jariyah"),
    ("shalat jamaah", "shalat sendiri"),
    ("puasa wajib", "puasa sunnah"),
    ("haji tamattu", "haji ifrad"),
    ("qurban", "aqiqah"),
]

for a, b in pairs:
    add("versus", f"perbedaan {a} dan {b}")
    add("versus", f"mana lebih utama {a} atau {b}")
    add("versus", f"{a} vs {b} menurut islam")

# ═══════════════════════════════════════════════════════════════
# F: Waris detailed scenarios
# ═══════════════════════════════════════════════════════════════

waris_scenarios = [
    "waris suami meninggal punya 1 anak laki 2 anak perempuan",
    "waris istri meninggal suami dan 3 anak perempuan",
    "waris ayah meninggal ibu dan 2 anak laki",
    "waris tidak ada anak hanya saudara",
    "waris kakek dan nenek saja",
    "waris suami istri tanpa anak",
    "waris anak tunggal perempuan",
    "waris anak tunggal laki-laki",
    "waris orang tua dan anak",
    "waris dzawil arham",
    "waris ashabah bin nafs",
    "waris ashabah bil ghair",
    "waris ashabah ma'al ghair",
    "waris aul cara menghitung",
    "waris radd cara menghitung",
    "waris gharrawain masalah",
    "waris musytarakah masalah",
    "waris akdariyah masalah",
    "waris anak dalam kandungan",
    "waris orang hilang mafqud",
    "waris khuntsa musykil",
    "waris beda agama",
    "waris pembunuh pewaris",
    "waris budak dan merdeka",
]
for q in waris_scenarios:
    add("waris", q)

# ═══════════════════════════════════════════════════════════════
# G: More English natural language
# ═══════════════════════════════════════════════════════════════

en_natural = [
    "what is the Islamic ruling on working in a bank",
    "can a Muslim eat at McDonald's",
    "is it permissible to have a mortgage",
    "what does Islam say about democracy",
    "can women travel alone in Islam",
    "is it sinful to miss a prayer",
    "what happens if you don't pay zakat",
    "how many times can a man divorce his wife",
    "can a woman ask for divorce in Islam",
    "what are the grounds for divorce in Islam",
    "is adoption allowed in Islamic law",
    "what Islam says about surrogacy",
    "can Muslims celebrate birthdays",
    "is celebrating New Year haram",
    "what does Islam say about racism",
    "Islamic view on LGBTQ",
    "can Muslims eat kosher food",
    "is chess halal or haram",
    "can women wear perfume outside",
    "is shaking hands with opposite gender allowed",
    "what breaks wudu in Shafi'i school",
    "can you pray with shoes on",
    "is it permissible to pray in English",
    "can you fast without praying",
    "what is the penalty for drinking alcohol in Islam",
    "how to calculate inheritance shares in Islam",
    "what are the signs of the Day of Judgment",
    "what is the Islamic view on evolution",
    "can Muslims donate blood",
    "is life insurance halal",
    "what is riba exactly",
    "can you pay zakat to your siblings",
    "is it haram to take photos",
    "what is the ruling on anime",
    "can Muslims listen to nasheed",
    "is investing in stocks halal",
    "what does Islam say about mental health",
    "can a Muslim be a vegetarian",
    "is it haram to waste food",
    "what is the difference between Sunni and Shia",
]
for q in en_natural:
    add("en_natural", q, None, "en")

# ═══════════════════════════════════════════════════════════════
# H: More Arabic scholarly queries
# ═══════════════════════════════════════════════════════════════

ar_scholarly = [
    "أحكام المسح على الجبيرة", "حكم الصلاة في السفينة",
    "حكم الجمع بين الصلاتين", "شروط القصر في السفر",
    "مسافة القصر", "حكم صلاة المسافر",
    "أحكام صلاة الجمعة للمسافر", "حكم الأضحية للمسافر",
    "زكاة الحلي المعد للاستعمال", "زكاة عروض التجارة",
    "نصاب زكاة الإبل", "نصاب زكاة البقر",
    "نصاب زكاة الغنم", "زكاة الركاز والمعادن",
    "أحكام الهدي والأضحية", "ذبح الأضحية عن الميت",
    "توزيع لحم الأضحية", "حكم بيع جلد الأضحية",
    "شروط الأضحية", "وقت ذبح الأضحية",
    "حكم نكاح الشغار", "نكاح التحليل حكمه",
    "الإيلاء وأحكامه", "الظهار وكفارته",
    "اللعان وآثاره", "العدة وأحكامها",
    "نفقة المعتدة", "سكنى المعتدة",
    "الرضاع المحرم شروطه", "عدد الرضعات المحرمة",
    "أحكام اللقيط", "أحكام اللقطة",
    "الشفعة وشروطها", "الجعالة وأحكامها",
    "المساقاة وأحكامها", "المزارعة وأحكامها",
    "الإقرار وأحكامه", "الشهادة وشروطها",
    "اليمين والنذر", "كفارة اليمين",
    "النذر المعلق", "نذر المعصية",
    "الاستئجار على الطاعات", "أجرة الإمام والمؤذن",
    "حكم أخذ الأجرة على تعليم القرآن",
    "حكم بيع المصحف", "حكم القراءة على الأموات",
    "حكم الدعاء الجماعي بعد الصلاة",
    "حكم المصافحة بين الرجل والمرأة",
    "حكم خلوة الرجل بالمرأة الأجنبية",
    "حكم النظر إلى المرأة الأجنبية",
    "ضابط العورة في الصلاة",
]
for q in ar_scholarly:
    add("ar_scholarly", q, None, "ar")

# ═══════════════════════════════════════════════════════════════
# I: Jenazah & death-related detailed
# ═══════════════════════════════════════════════════════════════

jenazah_topics = [
    "hukum memandikan jenazah", "cara memandikan jenazah laki-laki",
    "cara memandikan jenazah perempuan", "hukum mengkafani jenazah",
    "cara mengkafani jenazah laki-laki", "cara mengkafani jenazah perempuan",
    "jumlah kain kafan", "shalat jenazah berapa takbir",
    "doa shalat jenazah", "shalat ghaib jenazah",
    "menguburkan jenazah adabnya", "kedalaman lubang kubur",
    "arah kubur menghadap kiblat", "talqin mayit",
    "talqin setelah dikubur hukumnya", "ziarah kubur tata caranya",
    "ziarah kubur wanita hukumnya", "membaca quran di kuburan",
    "mengirim pahala untuk mayit", "sedekah jariyah untuk mayit",
    "doa untuk orang meninggal", "tahlil 7 hari 40 hari 100 hari",
    "haul orang meninggal", "peringatan kematian dalam islam",
    "bunuh diri dalam islam akibatnya", "euthanasia hukumnya",
    "autopsi jenazah", "kremasi dalam islam",
    "menggali kubur hukumnya", "memindahkan jenazah",
    "jenazah tenggelam tidak ditemukan", "jenazah yang sudah hancur",
    "shalat jenazah untuk anak kecil", "shalat jenazah untuk bayi",
    "keguguran janin hukumnya", "janin meninggal dalam kandungan",
    "aqiqah bayi meninggal", "nama bayi meninggal",
    "wasiat sebelum meninggal", "hutang orang meninggal",
]
for q in jenazah_topics:
    add("jenazah", q)

# ═══════════════════════════════════════════════════════════════
# J: Cross-product of "hikmah" / "rahasia" di balik ibadah
# ═══════════════════════════════════════════════════════════════

hikmah_topics = [
    "hikmah shalat lima waktu", "hikmah puasa ramadhan",
    "hikmah zakat", "hikmah menunaikan haji",
    "hikmah wudhu", "hikmah menutup aurat",
    "hikmah larangan riba", "hikmah larangan zina",
    "hikmah poligami dalam islam", "hikmah iddah bagi wanita",
    "hikmah shalat berjamaah", "hikmah shalat jumat",
    "hikmah qurban", "hikmah aqiqah",
    "hikmah khitan", "hikmah nikah dalam islam",
    "hikmah talak diperbolehkan", "hikmah warisan islam",
    "hikmah sedekah", "hikmah silaturahmi",
    "hikmah shalat tahajjud", "hikmah dzikir",
    "hikmah taubat", "hikmah sabar",
    "rahasia di balik shalat", "rahasia di balik puasa",
    "rahasia di balik wudhu", "rahasia di balik haji",
    "filosofi ibadah dalam islam", "makna spiritual shalat",
    "makna spiritual puasa", "makna spiritual zakat",
]
for q in hikmah_topics:
    add("hikmah", q)

# ═══════════════════════════════════════════════════════════════
# K: Economic/business-specific fiqh queries
# ═══════════════════════════════════════════════════════════════

business_queries = [
    "hukum franchise dalam islam", "hukum waralaba menurut islam",
    "hukum bisnis MLM multi level marketing",
    "hukum jual beli saham gorengan", "hukum short selling saham",
    "hukum margin trading", "hukum futures trading",
    "hukum binary options", "hukum spread betting",
    "hukum day trading saham", "hukum scalping forex",
    "hukum copy trading", "hukum auto trading bot",
    "hukum jual beli tanah kavling", "hukum makelar properti",
    "hukum komisi penjualan", "hukum diskon dan cashback",
    "hukum undian berhadiah", "hukum door prize",
    "hukum give away", "hukum endorsement produk",
    "hukum affiliate marketing", "hukum google adsense",
    "hukum iklan yang berlebihan", "hukum iklan menyesatkan",
    "hukum monopoli dalam islam", "hukum kartel harga",
    "hukum dumping harga", "hukum ijon sawah",
    "hukum tengkulak", "hukum menimbun barang",
    "hukum spekulasi harga", "hukum manipulasi pasar",
    "hukum insider trading", "hukum pencucian uang",
    "hukum pajak dalam islam", "pajak dan zakat bedanya",
    "hukum gratifikasi", "hukum hadiah kepada pejabat",
    "hukum pungli", "hukum upah minimum",
    "hukum PHK karyawan", "hukum outsourcing",
    "hukum kerja kontrak", "hukum magang tidak dibayar",
    "hukum tip dan uang pelicin", "hukum uang pangkal sekolah",
    "hukum jual beli followers", "hukum jual beli akun game",
    "hukum jual account sosmed", "hukum jual data pribadi",
]
for q in business_queries:
    add("bisnis", q)

# ═══════════════════════════════════════════════════════════════
# L: Social/ethical issues in Islam
# ═══════════════════════════════════════════════════════════════

social_queries = [
    "hak asasi manusia dalam islam", "kesetaraan gender Islam",
    "perbudakan dalam islam sejarah", "jihad pengertian dan jenis",
    "terorisme pandangan islam", "radikalisme dan islam",
    "pluralisme dan toleransi", "nasionalisme dalam islam",
    "demokrasi dalam islam", "sistem khilafah",
    "hukum pidana islam di negara modern", "implementasi syariah",
    "hudud di negara modern", "qishas di era modern",
    "ekonomi islam vs ekonomi konvensional",
    "perbankan syariah vs konvensional detail",
    "pasar modal syariah", "obligasi syariah sukuk",
    "fintech syariah", "ekonomi digital halal",
    "filantropi islam", "sedekah produktif",
    "wakaf produktif modern", "crowdfunding syariah",
    "pendidikan islam", "kurikulum pesantren",
    "modernisasi pesantren", "tradisi dan modernitas",
    "ijtihad kontemporer", "fatwa kontemporer",
    "peran ulama di era modern", "dai di era digital",
    "hukum kontes kecantikan", "hukum pertunjukan seni",
    "hukum teater dan drama", "hukum stand up comedy",
    "hukum parkour dan extreme sport", "hukum mixed martial arts",
    "hukum tinju dan gulat", "hukum balapan",
    "hukum berburu untuk olahraga", "hukum memancing ikan",
]
for q in social_queries:
    add("sosial", q)

# ═══════════════════════════════════════════════════════════════
# M: Animals in Islam
# ═══════════════════════════════════════════════════════════════

animals = [
    "anjing", "kucing", "tikus", "ular", "kalajengking", "lebah",
    "semut", "laba-laba", "lalat", "nyamuk", "cicak", "tokek",
    "bunglon", "iguana", "kadal", "kura-kura", "penyu",
    "buaya", "komodo", "biawak", "ular kobra", "ular piton",
    "elang", "rajawali", "gagak", "burung hantu", "kakatua",
    "merpati", "puyuh", "ayam hutan", "merak", "bangau",
    "kelelawar", "landak", "berang-berang", "musang",
    "harimau", "singa", "serigala", "rubah", "beruang",
    "gajah", "kuda nil", "badak", "jerapah", "zebra",
    "monyet", "gorila", "orangutan", "lumba-lumba", "paus",
    "hiu", "pari", "belut", "lele", "nila", "gurame",
]

for animal in animals:
    add("hewan", f"hukum memakan {animal} dalam islam")

# ═══════════════════════════════════════════════════════════════
# N: More detailed mixed language
# ═══════════════════════════════════════════════════════════════

mixed_detailed = [
    "shalat tarawih 8 atau 20 rakaat mana yang benar",
    "qunut subuh wajib menurut siapa sunnah menurut siapa",
    "menyentuh istri batal wudhu menurut syafii tapi tidak menurut hanafi",
    "imam syafii bilang kodok haram tapi imam malik bilang boleh",
    "talak tiga sekaligus jatuh tiga menurut jumhur jatuh satu menurut ibnu taimiyah",
    "rokok haram menurut MUI tapi makruh menurut sebagian ulama lama",
    "cadar wajib menurut sebagian ulama salafi sunnah menurut jumhur",
    "tahlilan bid'ah menurut wahabi sunnah menurut NU",
    "maulid nabi boleh menurut aswaja haram menurut salafi",
    "ziarah kubur sunnah bagi laki-laki khilaf bagi perempuan",
    "nikah tanpa wali sah menurut hanafi tidak sah menurut syafii",
    "membaca fatihah makmum wajib menurut syafii tidak wajib menurut hanafi",
    "shalat witir wajib menurut hanafi sunnah menurut jumhur",
    "mengusap kepala seluruh atau sebagian khilaf empat mazhab",
    "makan daging kelinci halal menurut jumhur haram menurut sebagian",
    "musik kontemporer khilaf ulama antara halal dan haram",
    "foto dan video khilaf ulama kontemporer",
    "cryptocurrency ada yang membolehkan ada yang mengharamkan",
    "vaksin babi khilaf ulama ada unsur haram tapi darurat",
    "bank asi khilaf ulama kontemporer",
]
for q in mixed_detailed:
    add("mixed_detail", q)

# ═══════════════════════════════════════════════════════════════
# O: Seasonal/event-based queries
# ═══════════════════════════════════════════════════════════════

events = [
    "amalan 10 hari pertama dzulhijjah", "amalan malam lailatul qadr",
    "amalan hari arafah bagi yang tidak haji", "amalan 10 terakhir ramadhan",
    "amalan hari jumat", "amalan malam jumat",
    "persiapan sebelum ramadhan", "hal yang dilakukan setelah ramadhan",
    "tradisi lebaran dalam islam", "shalat ied fitri tata cara",
    "shalat ied adha tata cara", "takbiran malam ied",
    "halal bihalal setelah lebaran", "mudik hukumnya",
    "memberi THR kepada pegawai", "angpao dari non muslim",
    "perayaan tahun baru hijriyah", "muharram bulan haram",
    "bulan-bulan haram dalam islam", "hari-hari yang diharamkan puasa",
    "hari tasyrik puasa dilarang", "puasa ayyamul bidh",
    "puasa 10 muharram dan 9 muharram",
]
for q in events:
    add("event", q)

# ═══════════════════════════════════════════════════════════════
# P: More detailed English topic queries
# ═══════════════════════════════════════════════════════════════

en_detailed = [
    "can you pray salah without hijab at home",
    "do you have to pray all 5 prayers",
    "what if someone dies without praying",
    "how to make up years of missed prayers",
    "can you eat pork if there is nothing else available",
    "is stunning before slaughter allowed in Islam",
    "halal slaughter method detailed explanation",
    "concept of bid'ah good and bad innovation",
    "can Muslims use conventional banking services",
    "Islamic ruling on working for Google or Facebook",
    "is day trading considered gambling",
    "ruling on buying lottery tickets",
    "can zakat be given to build schools",
    "can waqf property be sold",
    "Islamic view on patent and intellectual property",
    "what is the ruling on copyright in Islam",
    "Islamic bioethics overview",
    "end of life decisions Islamic perspective",
    "brain death and organ donation Islamic view",
    "genetic testing before marriage Islamic ruling",
    "prenatal testing and selective abortion",
    "Islamic ruling on surrogacy detailed",
    "ruling on sperm and egg donation",
    "gender reassignment surgery Islamic view",
    "intersex conditions Islamic jurisprudence",
    "medical marijuana Islamic ruling",
    "psychedelic therapy Islamic perspective",
    "Islamic view on climate change responsibility",
    "animal rights and welfare in Islam",
    "factory farming Islamic perspective",
    "can Muslims be vegan for ethical reasons",
    "Islamic perspective on refugees and asylum",
    "citizenship and immigration Islamic view",
    "Islamic ruling on whistleblowing",
    "corporate social responsibility in Islam",
    "Islamic view on artificial intelligence ethics",
]
for q in en_detailed:
    add("en_detailed", q, None, "en")

# ═══════════════════════════════════════════════════════════════
# ASSEMBLE
# ═══════════════════════════════════════════════════════════════

for i, q in enumerate(queries):
    q["id"] = f"q{i+1:05d}"

print(f"Total: {len(queries)}")
cats = {}
for q in queries:
    cats[q["category"]] = cats.get(q["category"], 0) + 1
for cat, count in sorted(cats.items(), key=lambda x: -x[1])[:20]:
    print(f"  {cat}: {count}")
print(f"  ... and {len(cats)-20} more categories")

with open("queries_10k.json", "w", encoding="utf-8") as f:
    json.dump(queries, f, ensure_ascii=False, indent=1)

eval_queries = [{"id": q["id"], "text": q["text"]} for q in queries]
with open("queries_10k_eval.json", "w", encoding="utf-8") as f:
    json.dump(eval_queries, f, ensure_ascii=False, indent=1)

print(f"Saved {len(queries)} queries")
