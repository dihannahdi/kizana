#!/usr/bin/env python3
"""
Generate 10,000+ systematic queries for comprehensive search evaluation.
Uses combinatorial expansion across domains, patterns, languages, and variations.
"""
import json
import itertools

queries = []
qid = 0

def add(category, text, expected_domain=None, lang="id"):
    global qid
    qid += 1
    queries.append({
        "id": f"{category}_{qid:05d}",
        "text": text,
        "category": category,
        "expected_domain": expected_domain,
        "lang": lang
    })

# ═══════════════════════════════════════════════════════════════
# 1. IBADAH / SHALAT — ~600 queries
# ═══════════════════════════════════════════════════════════════

shalat_variants = ["shalat", "solat", "sholat", "salat"]
shalat_types = [
    ("jumat", "jumat"), ("jenazah", "jenazah"), ("tahajud", "tahajud"),
    ("tahajjud", "tahajjud"), ("dhuha", "dhuha"), ("tarawih", "tarawih"),
    ("witir", "witir"), ("ied", "ied"), ("gerhana", "gerhana"),
    ("istikharah", "istikharah"), ("istisqa", "istisqa"),
    ("jamak", "jamak"), ("qashar", "qashar"), ("qasar", "qasar"),
    ("subuh", "subuh"), ("dzuhur", "dzuhur"), ("ashar", "ashar"),
    ("maghrib", "maghrib"), ("isya", "isya"),
]

# shalat X combinations
for sv in shalat_variants:
    for st_name, st_word in shalat_types:
        add("ibadah", f"{sv} {st_word}", "عبادات")

# hukum shalat X
for st_name, st_word in shalat_types:
    add("ibadah", f"hukum shalat {st_word}", "عبادات")

# shalat question patterns
shalat_questions = [
    "hukum shalat {}", "tata cara shalat {}", "waktu shalat {}",
    "niat shalat {}", "syarat sah shalat {}", "rukun shalat {}",
    "sunnah shalat {}", "bacaan shalat {}", "cara shalat {}",
    "keutamaan shalat {}", "jumlah rakaat shalat {}",
]
for pattern in shalat_questions:
    for st_name, st_word in shalat_types[:8]:  # top 8 types
        add("ibadah", pattern.format(st_word), "عبادات")

# General shalat topics
shalat_topics = [
    "hukum meninggalkan shalat", "shalat berjamaah di rumah",
    "shalat berjamaah di masjid", "shalat sendirian hukumnya",
    "shalat sambil duduk karena sakit", "imam shalat perempuan",
    "makmum yang terlambat", "shalat munfarid",
    "saf shalat perempuan", "aurat dalam shalat",
    "pakaian shalat laki-laki", "pakaian shalat perempuan",
    "shalat di atas kendaraan", "shalat di pesawat",
    "shalat di kereta", "shalat qadhaa", "qadha shalat",
    "shalat sunnah rawatib", "shalat sunnah muakkad",
    "shalat sambil pegang bayi", "sujud sahwi kapan dilakukan",
    "sujud tilawah hukumnya", "sujud syukur tata caranya",
    "rukun shalat menurut syafi'i", "rukun shalat menurut hanafi",
    "syarat wajib shalat", "syarat sah shalat",
    "hal yang membatalkan shalat", "membatalkan shalat",
    "gerakan shalat yang benar", "bacaan shalat lengkap",
    "doa setelah shalat", "dzikir setelah shalat",
    "qunut subuh hukumnya", "doa qunut dibaca kapan",
    "takbiratul ihram cara", "tahiyyat awal dan akhir",
    "salam dalam shalat", "shalat dengan najis di badan",
    "shalat tanpa wudhu", "lupa rakaat shalat",
    "ragu-ragu dalam shalat", "shalat memakai gamis",
    "shalat di tanah lapang", "shalat di kuburan hukumnya",
    "shalat menghadap kiblat", "kiblat shalat di Indonesia",
]
for topic in shalat_topics:
    add("ibadah", topic, "عبادات")

# English shalat queries
en_shalat = [
    "ruling on friday prayer", "how to pray tahajjud",
    "prayer while traveling", "conditions for valid prayer",
    "pillars of prayer in Islam", "prayer in congregation",
    "prayer times in Islam", "prostration of forgetfulness",
    "prayer of the sick person", "women leading prayer",
    "is music allowed during prayer", "prayer direction qibla",
    "missed prayers how to make up", "night prayer tahajjud",
    "eclipse prayer in Islam", "rain prayer istisqa",
    "funeral prayer how to", "eid prayer rules",
]
for q in en_shalat:
    add("ibadah_en", q, "عبادات", "en")

# ═══════════════════════════════════════════════════════════════
# 2. THAHARAH — ~400 queries
# ═══════════════════════════════════════════════════════════════

thaharah_topics = [
    "cara wudhu yang benar", "syarat sah wudhu", "rukun wudhu",
    "sunnah wudhu", "hal yang membatalkan wudhu", "wudhu batal karena apa",
    "tayammum syarat dan caranya", "tayammum kapan boleh",
    "hadas besar dan kecil", "mandi junub tata cara",
    "mandi wajib hukumnya", "mandi sunnah jenis-jenisnya",
    "najis mughallazhah cara membersihkan", "najis mutawassithah",
    "najis mukhaffafah", "macam-macam najis", "jenis-jenis najis",
    "air mutanajjis boleh untuk wudhu", "air musta'mal hukumnya",
    "air suci tapi tidak menyucikan", "macam-macam air dalam islam",
    "haid dan istihadzah perbedaannya", "darah haid berapa hari",
    "nifas berapa lama", "istihadhah shalat tetap",
    "darah keluar apakah batal wudhu", "kentut membatalkan wudhu",
    "tidur membatalkan wudhu", "menyentuh perempuan batal wudhu",
    "bersentuhan kulit batal wudhu menurut syafi'i",
    "wudhu dengan air zamzam", "wudhu dengan air laut",
    "wudhu dengan air hujan", "masah khuf hukumnya",
    "masah sepatu boleh gak", "wudhu perban di kaki",
    "wudhu sambil berbicara", "tertib dalam wudhu",
    "muwalat dalam wudhu", "niat wudhu kapan dibaca",
    "hukum menyentuh mushaf tanpa wudhu",
    "masuk masjid tanpa wudhu", "kucing najis atau tidak",
    "anjing najis dalam islam", "air liur anjing najis",
    "istihalah najis menjadi suci", "penyucian najis",
    "thaharah sebelum shalat", "bersuci dari hadats",
]
for topic in thaharah_topics:
    add("thaharah", topic, "طهارة")

# Thaharah variations with different wordings
thaharah_variations = [
    ("wudhu", ["wudhu", "wudu", "wuduk", "ablution"]),
    ("tayammum", ["tayammum", "tayamum"]),
    ("mandi", ["mandi junub", "mandi wajib", "mandi besar"]),
    ("najis", ["najis", "najasah", "kenajisan"]),
]
thaharah_prefixes = ["hukum", "cara", "syarat", "tata cara", "hal yang membatalkan"]
for concept, variants in thaharah_variations:
    for variant in variants:
        for prefix in thaharah_prefixes:
            add("thaharah", f"{prefix} {variant}", "طهارة")

# English thaharah
en_thaharah = [
    "how to perform ablution", "conditions for valid wudu",
    "what breaks wudu", "tayammum dry ablution rules",
    "ritual bath ghusl janabah", "types of impurity najis",
    "purification rules in Islamic law", "menstruation rules fiqh",
    "postpartum bleeding nifas", "wiping over socks masah",
]
for q in en_thaharah:
    add("thaharah_en", q, "طهارة", "en")

# ═══════════════════════════════════════════════════════════════
# 3. PUASA / FASTING — ~400 queries
# ═══════════════════════════════════════════════════════════════

puasa_topics = [
    "hal yang membatalkan puasa", "membatalkan puasa apa saja",
    "puasa ramadhan hukumnya wajib", "niat puasa ramadhan",
    "niat puasa kapan dibaca", "sahur sebelum imsak",
    "fidyah puasa orang tua", "kafarat puasa",
    "puasa sunnah senin kamis", "puasa sunnah arafah",
    "puasa 6 hari syawal", "puasa asyura",
    "puasa sunnah rajab", "puasa sunnah sya'ban",
    "puasa daud tata cara", "puasa nadzar hukumnya",
    "puasa bagi ibu hamil", "puasa bagi ibu menyusui",
    "puasa bagi musafir", "puasa bagi orang sakit",
    "puasa bagi orang tua renta", "puasa bagi anak kecil",
    "berbuka puasa sengaja kafarat", "makan lupa saat puasa",
    "makan tidak sengaja saat puasa", "minum obat saat puasa",
    "suntik saat puasa batal tidak", "infus saat puasa",
    "transfusi darah saat puasa", "muntah saat puasa",
    "mimisan saat puasa", "donor darah saat puasa",
    "gosok gigi saat puasa", "sikat gigi saat puasa",
    "menelan ludah saat puasa", "mencicipi makanan saat puasa",
    "berkumur saat puasa", "celak mata saat puasa",
    "tetes mata saat puasa", "mandi saat puasa",
    "berhubungan suami istri saat puasa", "jimak saat puasa kafarat",
    "puasa qadha berapa hari", "qadha puasa ramadhan",
    "bayar fidyah puasa", "puasa syawal langsung atau tidak",
    "i'tikaf di bulan ramadhan", "lailatul qadr kapan",
    "keutamaan puasa ramadhan", "tarawih berapa rakaat",
    "sahur terakhir jam berapa", "imsak dan subuh bedanya",
    "buka puasa doa", "doa buka puasa", "doa sahur",
]
for topic in puasa_topics:
    add("puasa", topic, "عبادات")

# Puasa combinatorial
puasa_who = ["ibu hamil", "orang sakit", "musafir", "orang tua", "anak kecil", "wanita haid"]
for who in puasa_who:
    add("puasa", f"hukum puasa bagi {who}", "عبادات")
    add("puasa", f"boleh tidak puasa {who}", "عبادات")
    add("puasa", f"apakah {who} wajib puasa", "عبادات")

puasa_sunnah = ["senin kamis", "arafah", "asyura", "syawal", "rajab", "sya'ban", "daud"]
for ps in puasa_sunnah:
    add("puasa", f"puasa sunnah {ps} hukumnya", "عبادات")
    add("puasa", f"keutamaan puasa {ps}", "عبادات")

# ═══════════════════════════════════════════════════════════════
# 4. ZAKAT — ~300 queries
# ═══════════════════════════════════════════════════════════════

zakat_topics = [
    "nisab zakat mal berapa", "zakat fitrah beras atau uang",
    "delapan golongan penerima zakat", "mustahik zakat siapa saja",
    "zakat emas dan perak nisabnya", "zakat profesi hukumnya",
    "zakat pertanian padi", "zakat perdagangan",
    "zakat hewan ternak", "zakat saham dan investasi",
    "zakat tabungan", "zakat deposito",
    "haul zakat berapa lama", "hukum tidak bayar zakat",
    "amil zakat siapa", "zakat untuk masjid boleh gak",
    "zakat untuk saudara", "zakat untuk orang tua",
    "zakat untuk non muslim", "zakat fitrah berapa kg",
    "zakat fitrah uang berapa", "waktu bayar zakat fitrah",
    "zakat mal dan zakat fitrah bedanya", "mustahik zakat menurut quran",
]
for topic in zakat_topics:
    add("zakat", topic, "عبادات")

zakat_types = ["emas", "perak", "pertanian", "perdagangan", "hewan ternak",
               "profesi", "saham", "tabungan", "deposito", "kripto", "penghasilan"]
for zt in zakat_types:
    add("zakat", f"zakat {zt} hukumnya", "عبادات")
    add("zakat", f"nisab zakat {zt}", "عبادات")
    add("zakat", f"cara menghitung zakat {zt}", "عبادات")

# ═══════════════════════════════════════════════════════════════
# 5. HAJI / UMRAH — ~300 queries
# ═══════════════════════════════════════════════════════════════

haji_topics = [
    "rukun haji", "wajib haji", "sunnah haji",
    "syarat wajib haji", "syarat sah haji",
    "ihram dari miqat", "miqat haji dari Indonesia",
    "tawaf ifadhah", "tawaf qudum", "tawaf wada",
    "sa'i antara shafa dan marwa", "wukuf di arafah",
    "mabit di muzdalifah", "mabit di mina",
    "melempar jumrah", "jumrah aqabah", "jumrah ula wustha",
    "dam haji jenis dan hukumnya", "dam tamattu",
    "haji ifrad", "haji tamattu", "haji qiran",
    "umrah tata cara", "umrah di bulan ramadhan",
    "umrah berulang kali", "haji badal", "badal haji orang meninggal",
    "haji anak kecil", "haji perempuan tanpa mahram",
    "haji lansia", "haji berhutang", "tabungan haji",
    "haji wajib sekali seumur hidup", "haji sunnah",
    "manasik haji lengkap", "doa tawaf", "doa sa'i",
    "doa wukuf arafah", "larangan saat ihram",
    "tahallul awal dan tsani", "cukur rambut haji",
    "pakaian ihram laki-laki", "pakaian ihram perempuan",
]
for topic in haji_topics:
    add("haji", topic, "عبادات")

# ═══════════════════════════════════════════════════════════════
# 6. MUNAKAHAT (MARRIAGE/DIVORCE) — ~500 queries
# ═══════════════════════════════════════════════════════════════

nikah_topics = [
    "syarat sah nikah", "rukun nikah", "hukum nikah dalam islam",
    "nikah siri hukumnya", "nikah mut'ah", "nikah misyar",
    "nikah beda agama", "nikah campur agama", "nikah non muslim",
    "wali nikah siapa", "wali hakim nikah", "wali nasab",
    "mahar nikah minimal", "mahar nikah berapa",
    "ijab qabul nikah", "sighat nikah", "lafazh nikah",
    "saksi nikah dua orang", "pernikahan tanpa saksi",
    "nikah tanpa wali hukumnya", "nikah tanpa ijin orang tua",
    "menikah diam-diam", "nikah di bawah umur",
    "usia menikah dalam islam", "kafa'ah dalam nikah",
    "khitbah lamaran", "melihat calon istri",
    "walimah ursy hukumnya", "walimah syarat",
]
for topic in nikah_topics:
    add("nikah", topic, "مناكحات")

talak_topics = [
    "talak tiga sekaligus", "talak satu dua tiga",
    "talak raj'i", "talak ba'in", "talak kinayah",
    "talak sarih", "talak lewat whatsapp", "talak lewat sms",
    "talak saat marah", "talak saat emosi",
    "iddah cerai mati", "iddah talak raj'i", "iddah cerai hamil",
    "rujuk setelah talak", "rujuk tata caranya",
    "cerai gugat istri", "khuluk hukumnya",
    "fasakh nikah", "li'an suami istri",
    "cerai karena suami impoten", "cerai karena suami selingkuh",
    "cerai karena KDRT", "cerai di pengadilan agama",
    "hak asuh anak setelah cerai", "hadhanah hukumnya",
    "nafkah anak setelah cerai", "nafkah iddah",
]
for topic in talak_topics:
    add("nikah", topic, "مناكحات")

nafkah_topics = [
    "nafkah suami kepada istri", "nafkah wajib suami",
    "istri tidak mau taat", "nusyuz istri",
    "poligami syarat dan hukumnya", "poligami empat istri",
    "poligami adil hukumnya", "istri kedua hukumnya",
    "mahar mitsil", "mahar musamma",
    "hak suami dalam islam", "hak istri dalam islam",
    "kewajiban suami", "kewajiban istri",
    "hubungan suami istri malam pertama", "adab hubungan suami istri",
    "azl coitus interruptus", "KB dalam islam",
    "hak waris istri", "hak anak dalam islam",
    "anak angkat dalam islam", "tabanni hukumnya",
    "radha'ah persusuan", "mahram karena sesusuan",
    "nikah dengan saudara sesusuan", "larangan nikah",
    "muhrim yang haram dinikahi", "nasab anak zina",
    "anak luar nikah hak waris", "wali anak perempuan",
]
for topic in nafkah_topics:
    add("nikah", topic, "مناكحات")

# ═══════════════════════════════════════════════════════════════
# 7. MUAMALAT (TRANSACTIONS) — ~600 queries
# ═══════════════════════════════════════════════════════════════

muamalat_topics = [
    # Jual beli
    "jual beli dalam islam", "syarat sah jual beli",
    "jual beli online hukumnya", "jual beli salam",
    "jual beli istishna", "jual beli murabahah",
    "jual beli kredit cicilan", "jual beli dengan uang muka",
    "jual beli gharar", "jual beli najis",
    "jual beli yang dilarang", "macam-macam jual beli haram",
    "jual beli khiyar", "hak khiyar pembeli",
    "jual beli pre order", "jual beli dropship",
    "jual beli valuta asing", "jual beli emas",
    # Riba
    "riba bank konvensional", "riba hukumnya dalam islam",
    "riba fadhl dan riba nasiah", "macam-macam riba",
    "bunga bank termasuk riba", "bunga pinjaman riba",
    "kredit rumah riba", "KPR syariah",
    # Hutang piutang
    "hutang piutang hukumnya", "adab berhutang",
    "bayar hutang menunda-nunda", "gharim penerima zakat",
    "qardh hasan pinjaman tanpa bunga",
    # Perbankan syariah
    "bank syariah halal haram", "mudharabah akad bagi hasil",
    "musyarakah kerjasama usaha", "murabahah jual beli bank",
    "ijarah sewa menyewa", "wakalah perwakilan",
    # Gadai
    "gadai emas di bank syariah", "rahn gadai hukumnya",
    "gadai tanah", "gadai sawah",
    # Asuransi
    "asuransi syariah halal haram", "asuransi konvensional haram",
    "takaful asuransi islami",
    # Wakaf
    "wakaf tanah syarat", "wakaf uang hukumnya",
    "wakaf produktif", "nazhir wakaf",
    # Waris
    "pembagian warisan islam", "waris anak perempuan",
    "waris anak laki-laki", "waris istri", "waris suami",
    "waris ibu dan bapak", "aul dan radd dalam waris",
    "ashabah dalam waris", "dzawil furudh",
    "hijab waris", "waris kakek dan nenek",
    "waris cucu", "waris saudara",
    # Wasiat & hibah
    "wasiat maksimal sepertiga", "wasiat kepada ahli waris",
    "hibah harta kepada anak", "hibah menarik kembali",
    # Modern finance
    "hukum dropship dalam islam", "hukum MLM dalam islam",
    "hukum saham dalam islam", "hukum obligasi syariah",
    "hukum forex trading", "hukum cryptocurrency bitcoin",
    "hukum leasing dalam islam", "hukum paylater dalam islam",
    "hukum pinjaman online", "hukum fintech syariah",
]
for topic in muamalat_topics:
    add("muamalat", topic, "معاملات")

# Muamalat combinatorial
akad_types = ["mudharabah", "musyarakah", "murabahah", "ijarah", "salam",
              "istishna", "wakalah", "kafalah", "hawalah", "rahn"]
akad_questions = ["pengertian", "hukum", "syarat", "rukun", "contoh", "dalil"]
for akad in akad_types:
    for q in akad_questions:
        add("muamalat", f"{q} akad {akad}", "معاملات")

# ═══════════════════════════════════════════════════════════════
# 8. AQIDAH — ~400 queries
# ═══════════════════════════════════════════════════════════════

aqidah_topics = [
    "tauhid rububiyah uluhiyah", "tauhid asma wa sifat",
    "macam-macam tauhid", "syirik kecil dan besar",
    "syirik khafi", "syirik akbar contoh",
    "bid'ah hasanah dan sayyi'ah", "bid'ah dalam agama",
    "hukum bid'ah menurut ulama",
    "tawassul kepada orang shaleh", "tawassul hukumnya",
    "maulid nabi hukumnya", "perayaan maulid",
    "tahlilan hukumnya", "yasinan hukumnya",
    "iman kepada qadha qadar", "takdir dalam islam",
    "hukum murtad dalam islam", "riddah hukumnya",
    "sifat dua puluh allah", "sifat wajib allah",
    "sifat mustahil allah", "sifat jaiz allah",
    "arkanul iman enam perkara", "rukun iman",
    "iman kepada malaikat", "iman kepada kitab",
    "iman kepada rasul", "iman kepada hari akhir",
    "iman kepada qadha qadar", "kufur dan jenisnya",
    "nifaq munafik", "fasiq", "kafir dzimmi",
    "ahlussunnah wal jamaah aqidahnya", "asy'ariyah maturidiyah",
    "aqidah ahlul sunnah", "manhaj salaf",
    "isra miraj hukum merayakan", "malam nisfu sya'ban",
    "karamah wali allah", "keramat orang shaleh",
    "ziarah kubur hukumnya", "ziarah makam wali",
    "wasilah dan tawassul", "istighatsah hukumnya",
    "tabarruk dengan peninggalan nabi", "jimat dan rajah",
    "perdukunan dan ramalan", "meramal nasib hukumnya",
    "ruqyah syar'iyyah", "pengobatan dengan quran",
]
for topic in aqidah_topics:
    add("aqidah", topic, "عقيدة")

# ═══════════════════════════════════════════════════════════════
# 9. JINAYAT (CRIMINAL LAW) — ~250 queries
# ═══════════════════════════════════════════════════════════════

jinayat_topics = [
    "hukum mencuri dalam islam", "had sariqah pencurian",
    "nisab pencurian berapa", "potong tangan pencuri",
    "qishas hukumnya", "qishas pembunuhan",
    "diyat ganti rugi pembunuhan", "diyat berapa",
    "pembunuhan sengaja hukumnya", "pembunuhan semi sengaja",
    "pembunuhan tidak sengaja", "hukum membunuh dalam islam",
    "ta'zir dalam hukum islam", "hukuman ta'zir",
    "hudud jenis dan hukumnya", "macam-macam hudud",
    "had zina muhshan", "had zina ghairu muhshan",
    "hukum zina dalam islam", "rajam hukumnya",
    "cambuk hukuman zina", "li'an tuduhan zina",
    "qadzaf menuduh zina", "had minum khamr",
    "hukum miras dalam islam", "hukum narkoba islam",
    "hirabah perampokan", "bughat pemberontakan",
    "riddah murtad hukumnya", "hukum bunuh diri",
    "pembunuhan untuk bela diri", "membela diri dalam islam",
    "hukum aborsi dalam islam", "aborsi sebelum 40 hari",
    "aborsi karena pemerkosaan", "jinayah terhadap janin",
    "denda jinayah", "ganti rugi pelukaan",
    "arsy pelukaan", "hukumah pelukaan",
    "ta'zir penjara", "ta'zir pengasingan",
    "hukum korupsi dalam islam", "hukum suap",
]
for topic in jinayat_topics:
    add("jinayat", topic, "جنايات")

# ═══════════════════════════════════════════════════════════════
# 10. TASAWUF / AKHLAK — ~300 queries
# ═══════════════════════════════════════════════════════════════

tasawuf_topics = [
    "taubat nasuha syarat", "cara taubat yang benar",
    "dzikir setelah shalat", "wirid pagi dan petang",
    "riya dan ujub bedanya", "bahaya riya dalam ibadah",
    "ghibah dan namimah hukumnya", "hasad dengki hukumnya",
    "tawadhu rendah hati", "kibr sombong dosa",
    "sabar dalam islam", "ikhlas dalam beribadah",
    "tawakkal kepada allah", "husnuzhan baik sangka",
    "su'uzhan buruk sangka", "adab murid kepada guru",
    "adab makan dalam islam", "adab tidur dalam islam",
    "adab berpakaian dalam islam", "adab berbicara",
    "adab bertetangga", "adab bertamu",
    "silaturahmi hukumnya", "birrul walidain",
    "berbakti kepada orang tua", "durhaka kepada orang tua",
    "hak tetangga dalam islam", "berbuat baik kepada sesama",
    "akhlak mahmudah", "akhlak mazmumah",
    "sifat-sifat terpuji", "sifat-sifat tercela",
    "thariqat tasawuf hukumnya", "tarekat naqsyabandiyah",
    "tarekat qadiriyah", "suluk tasawuf",
    "maqam dan hal dalam tasawuf", "zuhud dalam islam",
    "wara dalam islam", "muraqabah muhasabah",
    "mahabbah cinta kepada allah", "khauf takut kepada allah",
    "raja harap kepada allah", "ihsan dalam islam",
]
for topic in tasawuf_topics:
    add("tasawuf", topic)

# ═══════════════════════════════════════════════════════════════
# 11. MAKANAN & MINUMAN — ~250 queries
# ═══════════════════════════════════════════════════════════════

makanan_topics = [
    "makanan haram dalam islam", "jenis binatang haram dimakan",
    "hukum makan gelatin babi", "gelatin sapi halal",
    "sembelihan ahli kitab", "cara menyembelih yang benar",
    "makan daging ular", "makan daging kodok",
    "makan daging anjing", "makan daging kucing",
    "makan daging kuda", "makan daging kelinci",
    "makan cumi dan kerang", "makan udang halal",
    "makan kepiting hukumnya", "makan bekicot",
    "makan serangga halal", "makan belalang",
    "khamr dan nabidz hukumnya", "minuman beralkohol halal haram",
    "hukum merokok dalam islam", "rokok haram menurut siapa",
    "hukum vape dan rokok elektrik", "hukum shisha",
    "istihalah najis menjadi suci", "istihalah dalam makanan",
    "makanan mengandung emulsifier", "makanan mengandung alkohol",
    "restoran non muslim boleh makan", "makanan dimasak non muslim",
    "bismillah sebelum makan", "doa sebelum dan sesudah makan",
    "adab makan dalam islam", "makan dengan tangan kanan",
    "makan sambil berdiri", "makan berlebihan",
]
for topic in makanan_topics:
    add("makanan", topic)

# ═══════════════════════════════════════════════════════════════
# 12. KONTEMPORER — ~500 queries
# ═══════════════════════════════════════════════════════════════

kontemporer_topics = [
    "hukum transplantasi organ", "hukum donor organ",
    "hukum bayi tabung", "inseminasi buatan",
    "hukum kloning manusia", "kloning hewan hukumnya",
    "vaksin mengandung babi", "vaksinasi hukumnya",
    "hukum aborsi dalam islam", "KB kontrasepsi hukumnya",
    "cryptocurrency bitcoin halal", "hukum trading forex",
    "hukum saham konvensional", "hukum obligasi",
    "hukum foto dan gambar makhluk hidup", "selfie hukumnya",
    "hukum video dan film", "nonton film hukumnya",
    "musik dan nyanyian hukumnya", "alat musik haram",
    "bank asi hukumnya", "donor asi dalam islam",
    "operasi plastik hukumnya", "operasi ganti kelamin",
    "hukum tato dalam islam", "hukum piercing",
    "hukum cukur alis", "hukum hair extension",
    "hukum demo dan demonstrasi", "dakwah di media sosial",
    "hukum pacaran dalam islam", "ikhtilat campur baur",
    "hukum berjabat tangan lawan jenis", "aurat di depan muhrim",
    "hukum melihat aurat", "jilbab wajib atau sunnah",
    "cadar niqab hukumnya", "jenggot hukumnya",
    "hukum asuransi jiwa", "asuransi kesehatan",
    "hukum rokok menurut empat mazhab", "hukum ganja",
    "hukum narkoba", "hukum minuman energi",
    "hukum makanan Jepang mentah", "sushi halal haram",
    "hukum pelihara anjing", "hukum pelihara kucing",
    "hukum membunuh ular", "hukum membunuh semut",
    "hukum bunuh nyamuk", "hukum bunuh cicak",
    "jual beli cryptocurrency", "NFT hukumnya",
    "investasi reksadana syariah", "peer to peer lending",
    "pinjaman online hukumnya", "hukum paylater",
    "gojek grab hukumnya", "ojek online halal",
    "hukum bekerja di bank konvensional",
    "hukum bekerja di tempat jual alkohol",
    "hukum arsitek membangun gereja", "hukum ucapan natal",
    "hukum menghadiri acara non muslim",
]
for topic in kontemporer_topics:
    add("kontemporer", topic)

# ═══════════════════════════════════════════════════════════════
# 13. QURAN & TAFSIR — ~200 queries
# ═══════════════════════════════════════════════════════════════

tafsir_topics = [
    "tafsir surat al fatihah", "tafsir ayat kursi",
    "tafsir surat yasin", "tafsir surat al mulk",
    "tafsir surat ar rahman", "tafsir surat al waqiah",
    "asbab nuzul surat al baqarah", "asbab nuzul surat an nisa",
    "makiyyah dan madaniyyah", "nasikh mansukh dalam quran",
    "muhkam dan mutasyabih", "i'jaz quran kemukjizatan",
    "qira'at sab'ah tujuh bacaan", "hukum tajwid",
    "adab membaca quran", "keutamaan membaca quran",
    "menghafal quran metode", "hukum membaca quran tanpa wudhu",
    "tafsir bil ma'tsur", "tafsir bil ra'yi",
]
for topic in tafsir_topics:
    add("tafsir", topic, "تفسير")

# ═══════════════════════════════════════════════════════════════
# 14. HADITS — ~150 queries
# ═══════════════════════════════════════════════════════════════

hadits_topics = [
    "hadits sahih pengertian", "hadits hasan pengertian",
    "hadits dhaif pengertian", "hadits maudhu palsu",
    "hadits mutawatir dan ahad", "hadits marfu mauquf maqthu",
    "sanad dan matan hadits", "jarh wa ta'dil",
    "takhrij hadits metode", "ilmu musthalah hadits",
    "hadits qudsi apa itu", "hadits nabawi",
    "kitab shahih bukhari", "kitab shahih muslim",
    "sunan abu dawud", "sunan tirmidzi",
    "sunan nasa'i", "sunan ibnu majah",
    "muwaththa imam malik", "musnad ahmad",
    "hadits tentang shalat", "hadits tentang puasa",
    "hadits tentang zakat", "hadits tentang haji",
    "hadits tentang nikah", "hadits tentang jual beli",
    "arbain nawawi empat puluh hadits",
]
for topic in hadits_topics:
    add("hadits", topic, "حديث")

# ═══════════════════════════════════════════════════════════════
# 15. ENGLISH QUERIES — ~400 queries
# ═══════════════════════════════════════════════════════════════

english_queries = [
    "ruling on interest in Islamic finance", "is music haram in Islam",
    "conditions for valid marriage", "fasting rules during Ramadan",
    "inheritance shares Islamic law", "ruling on apostasy",
    "pillars of Islam five", "pillars of faith six",
    "is photography allowed in Islam", "ruling on tattoo",
    "organ transplantation Islamic ruling", "test tube baby IVF",
    "cryptocurrency halal or haram", "forex trading allowed",
    "insurance in Islamic law", "mortgage halal",
    "abortion ruling in Islam", "contraception birth control",
    "cloning human Islamic perspective", "vaccination halal",
    "breast milk bank ruling", "plastic surgery allowed",
    "can women lead prayer", "mixed gender gatherings",
    "handshake opposite gender", "hijab mandatory",
    "niqab ruling scholars", "beard obligation",
    "smoking haram or makruh", "vaping ruling",
    "chocolate with alcohol content", "gelatin from pork",
    "eating at non Muslim restaurant", "slaughter requirements",
    "stunning animals before slaughter", "halal certification",
    "zakah on gold and silver", "zakah on salary",
    "hajj obligation conditions", "umrah during Ramadan",
    "funeral prayer absent", "eclipse prayer",
    "prayer while travelling", "combining prayers",
    "shortening prayers travel", "witr prayer ruling",
    "tahajjud prayer virtues", "istikhara prayer how",
    "dua after prayer", "dhikr morning evening",
    "repentance conditions Islam", "backbiting sin",
    "envy hasad cure", "pride kibr major sin",
    "patience sabr reward", "gratitude shukr Islam",
    "reliance on Allah tawakkul", "sincerity ikhlas",
    "hypocrisy nifaq signs", "major sins in Islam",
    "minor sins kaba'ir", "intercession shafa'ah",
    "grave punishment belief", "day of judgment signs",
    "paradise description Quran", "hellfire description",
    "angels their duties", "jinn existence ruling",
    "magic sorcery ruling", "evil eye ruqyah",
    "innovation bidah", "celebrating mawlid",
    "visiting graves ruling", "tawassul permissibility",
    "saints miracles karamah", "predestination qadr",
    "divorce rules Islam", "three talaq at once",
    "khula wife initiated divorce", "iddah waiting period",
    "child custody after divorce", "alimony nafaqah",
    "polygamy conditions Islam", "temporary marriage mutah",
    "dowry mahr amount", "marriage guardian wali",
    "witnesses in marriage", "interfaith marriage",
    "breastfeeding mahram", "adoption in Islam",
    "gifts hibah ruling", "will wasiyyah limit",
    "endowment waqf conditions", "debt loan ruling",
    "partnership musharakah", "profit sharing mudarabah",
    "lease ijarah contract", "guarantee kafalah",
    "transfer of debt hawalah", "agency wakalah",
    "sale on credit installment", "forward sale salam",
    "manufacturing contract istisna", "options khiyar",
    "gharar uncertainty prohibition", "gambling maysir",
    "theft punishment hudud", "adultery punishment",
    "false accusation qadhf", "apostasy punishment",
    "highway robbery hirabah", "rebellion bughat",
    "retaliation qisas", "blood money diyah",
    "discretionary punishment tazir",
]
for q in english_queries:
    add("english", q, None, "en")

# ═══════════════════════════════════════════════════════════════
# 16. ARABIC QUERIES — ~200 queries
# ═══════════════════════════════════════════════════════════════

arabic_queries = [
    "حكم الربا في الاسلام", "شروط صحة النكاح", "أحكام الطهارة",
    "الميراث والفرائض", "حكم الزكاة في المال", "أركان الصلاة",
    "شروط صحة الوضوء", "مبطلات الصيام", "أركان الحج",
    "الطلاق وأحكامه", "نكاح المتعة", "حكم الموسيقى والغناء",
    "حكم التصوير", "الأضحية وأحكامها", "العقيقة",
    "زكاة الفطر", "صلاة الجمعة أحكامها", "صلاة الجنازة",
    "الإجهاض حكمه", "الاستنساخ البشري", "نقل الأعضاء",
    "حكم التدخين", "حكم المخدرات", "حد السرقة",
    "حد الزنا", "القصاص والدية", "التعزير",
    "البيع والشراء", "الإجارة", "المضاربة والمشاركة",
    "الوقف وأحكامه", "الوصية وحدودها", "الهبة",
    "الرهن", "الكفالة", "الحوالة",
    "التوبة وشروطها", "الذكر والدعاء", "الغيبة والنميمة",
    "الكبر والعجب", "الحسد", "التوكل على الله",
    "بدعة حسنة", "التوسل", "زيارة القبور",
    "صفات الله تعالى", "توحيد الربوبية", "الشرك",
    "الردة وأحكامها", "النفاق", "الإيمان بالقدر",
    "حكم الخمر", "أحكام الأطعمة والأشربة", "الذبائح",
    "صلاة المسافر", "الجمع والقصر", "صلاة المريض",
    "سجود السهو", "سجود التلاوة", "صلاة الاستخارة",
    "صلاة التراويح", "صلاة الضحى", "صلاة الوتر",
    "نواقض الوضوء", "الغسل والجنابة", "التيمم",
    "الحيض والاستحاضة", "النفاس", "أنواع المياه",
    "النجاسات وتطهيرها", "المسح على الخفين",
]
for q in arabic_queries:
    add("arabic", q, None, "ar")

# ═══════════════════════════════════════════════════════════════
# 17. MIXED / COLLOQUIAL — ~400 queries
# ═══════════════════════════════════════════════════════════════

mixed_queries = [
    "boleh gak shalat pakai celana pendek", "gimana hukumnya nikah beda agama",
    "kucing najis gak sih", "kenapa riba diharamkan",
    "apakah tahlilan itu bid'ah", "boleh gak wanita jadi imam",
    "doa qunut dibaca kapan sih", "shalat gak pake wudhu gimana",
    "nikah siri boleh gak", "talak lewat whatsapp sah gak",
    "puasa tapi minum obat gimana", "puasa tapi kerja berat gimana",
    "zakat harus bayar ke siapa", "haji bisa gak dicicil",
    "gimana cara shalat yang bener", "wudhu di toilet boleh gak",
    "masjid masuk gak pake wudhu", "shalat di lantai kotor",
    "shalat pake kaos oblong", "puasa tapi merokok gimana",
    "riba bank itu dosa besar ya", "kripto halal gak sih",
    "bitcoin itu termasuk judi gak", "cicilan motor riba gak",
    "kredit hp riba atau bukan", "nabung di bank konvensional",
    "kerja di bank konvensional", "gaji dari bank riba halal",
    "asuransi wajib dari kantor", "BPJS termasuk riba gak",
    "vaksin ada babinya gak", "boleh makan di restoran chinese",
    "kue mengandung rhum", "coklat ada alkohol dikit",
    "parfum ada alkohol boleh", "hand sanitizer mengandung alkohol hukumnya",
    "obat sirup ada alkohol", "mie instan halal gak",
    "sate padang ada babi gak", "makan kambing boleh semua bagian",
    "emping melinjo haram gak", "tupai boleh dimakan gak",
    "ikan buntal beracun boleh dimakan", "jengkrik goreng halal",
    "cacing goreng halal gak", "ulat sagu hukumnya",
    "anjing boleh dipelihara gak", "kucing masuk rumah",
    "tokek halal dimakan gak", "ular halal dimakan gak",
    "gambar anime hukumnya", "nonton anime haram gak",
    "main game online hukumnya", "main PUBG haram",
    "TikTok haram gak", "Instagram hukumnya",
    "pacaran dalam islam gimana", "chatting sama cewek dosa gak",
    "lihat foto cewek cantik dosa", "nyanyi lagu pop hukumnya",
    "joget TikTok hukumnya", "cosplay hukumnya",
    "foto selfie haram gak", "bikin konten YouTube halal",
    "jadi influencer hukumnya", "endorse produk haram",
    "jualan online tanpa stok", "dropship boleh gak",
    "reseller MLM hukumnya", "ngutang buat modal usaha",
    "pinjam uang buat nikah", "mas kawin cincin emas boleh",
    "nikah di KUA gratis halal", "nikah massal hukumnya",
    "kawin lari hukumnya", "nikah gantung boleh gak",
    "tunangan bisa batal gak", "memberi hadiah ke calon istri",
    "taaruf online hukumnya", "kenalan lewat aplikasi",
    "boleh gak nolak lamaran", "wanita melamar duluan",
]
for q in mixed_queries:
    add("campuran", q)

# ═══════════════════════════════════════════════════════════════
# 18. SYSTEMATIC PATTERN EXPANSION — ~2000 queries
# ═══════════════════════════════════════════════════════════════

# Pattern: "hukum X" for many X
hukum_subjects = [
    "shalat", "puasa", "zakat", "haji", "umrah", "nikah", "talak",
    "riba", "judi", "miras", "narkoba", "korupsi", "pencurian",
    "zina", "ghibah", "namimah", "hasad", "bohong", "suap",
    "sogok", "kolusi", "nepotisme", "demo", "mogok kerja",
    "boikot produk", "jihad", "perang", "qital",
    "bunuh diri", "euthanasia", "aborsi", "bayi tabung",
    "kloning", "transplantasi organ", "donor darah",
    "vaksinasi", "KB", "azl", "sterilisasi",
    "adopsi anak", "anak angkat", "panti asuhan",
    "menabung", "investasi", "saham", "obligasi",
    "reksadana", "deposito", "forex", "kripto",
    "dropship", "MLM", "franchise", "leasing",
    "paylater", "pinjol", "kartu kredit",
    "asuransi", "BPJS", "dana pensiun",
    "rokok", "vape", "shisha", "ganja", "khat",
    "tato", "piercing", "sulam alis", "botox",
    "operasi plastik", "operasi ganti kelamin",
    "foto", "video", "film", "musik", "nyanyian",
    "TikTok", "YouTube", "Instagram", "game online",
    "jilbab", "cadar", "celana cingkrang", "jenggot",
    "pacaran", "ikhtilat", "khalwat", "jabat tangan",
    "ucapan natal", "valentine", "halloween",
    "merayakan tahun baru", "ulang tahun",
    "memelihara anjing", "memelihara kucing",
    "memelihara burung", "adu hewan",
    "tahlilan", "yasinan", "maulid nabi",
    "ziarah kubur", "membaca quran di kuburan",
    "kirim pahala untuk mayit", "sedekah untuk mayit",
    "shalat hajat", "istikharah untuk jodoh",
]
for subject in hukum_subjects:
    add("hukum_x", f"hukum {subject}")

# Pattern: "syarat X"
syarat_subjects = [
    "shalat", "wudhu", "tayammum", "puasa", "zakat", "haji",
    "umrah", "nikah", "talak", "khuluk", "jual beli",
    "sewa menyewa", "gadai", "wakaf", "wasiat",
    "imam shalat", "khotib jumat", "muadzin",
    "menjadi hakim", "menjadi saksi", "menjadi wali",
    "taubat", "kurban", "aqiqah",
]
for subject in syarat_subjects:
    add("syarat_x", f"syarat {subject}")
    add("syarat_x", f"syarat sah {subject}")

# Pattern: "cara X" / "tata cara X"
cara_subjects = [
    "wudhu", "tayammum", "mandi junub", "shalat",
    "shalat jenazah", "shalat istikharah", "shalat tahajjud",
    "shalat dhuha", "puasa", "berzakat", "haji",
    "umrah", "menikah", "menyembelih", "qurban",
    "aqiqah", "taubat", "ruqyah",
    "shalat berjamaah", "shalat gerhana",
    "memandikan jenazah", "mengkafani jenazah",
    "shalat mayit", "menguburkan jenazah",
]
for subject in cara_subjects:
    add("cara_x", f"tata cara {subject}")
    add("cara_x", f"cara {subject} yang benar")

# Pattern: "doa X"
doa_subjects = [
    "sebelum makan", "sesudah makan", "sebelum tidur",
    "bangun tidur", "masuk masjid", "keluar masjid",
    "masuk kamar mandi", "keluar kamar mandi",
    "sebelum bepergian", "naik kendaraan",
    "saat hujan", "saat petir", "setelah adzan",
    "qunut", "istikharah", "tahajjud",
    "buka puasa", "sahur", "setelah shalat",
    "untuk orang sakit", "untuk orang meninggal",
    "untuk kedua orang tua", "mohon jodoh",
    "mohon rezeki", "mohon kesembuhan",
    "mohon perlindungan", "selamat dari musibah",
]
for subject in doa_subjects:
    add("doa_x", f"doa {subject}")

# Pattern: "keutamaan X"
keutamaan_subjects = [
    "shalat berjamaah", "shalat tahajjud", "puasa senin kamis",
    "puasa ramadhan", "sedekah", "membaca quran",
    "dzikir", "shalawat", "istighfar",
    "shalat dhuha", "puasa arafah", "umrah ramadhan",
    "menolong sesama", "silaturahmi",
]
for subject in keutamaan_subjects:
    add("keutamaan_x", f"keutamaan {subject}")

# Pattern: "macam-macam X"
macam_subjects = [
    "air", "najis", "hadats", "puasa sunnah",
    "shalat sunnah", "zakat", "talak", "riba",
    "akad", "syirik", "bid'ah", "nikah",
    "waris", "hudud", "sujud",
]
for subject in macam_subjects:
    add("macam_x", f"macam-macam {subject}")
    add("macam_x", f"jenis-jenis {subject}")

# Pattern: "perbedaan X dan Y"
diff_pairs = [
    ("haid", "istihadzah"), ("zakat mal", "zakat fitrah"),
    ("sunnah", "wajib"), ("halal", "haram"),
    ("syirik", "kufur"), ("nifaq", "kufur"),
    ("talak raj'i", "talak ba'in"), ("qishas", "diyat"),
    ("riba fadhl", "riba nasiah"), ("hadits sahih", "hadits hasan"),
    ("bid'ah hasanah", "bid'ah dhalalah"),
    ("fardu ain", "fardu kifayah"),
    ("sunnah muakkadah", "sunnah ghairu muakkadah"),
    ("wajib", "fardu"), ("makruh tanzih", "makruh tahrim"),
    ("haji ifrad", "haji tamattu"), ("umrah wajib", "umrah sunnah"),
]
for a, b in diff_pairs:
    add("perbedaan", f"perbedaan {a} dan {b}")
    add("perbedaan", f"beda {a} dengan {b}")

# Pattern: "menurut mazhab X"
mazhab_names = ["syafi'i", "hanafi", "maliki", "hanbali"]
mazhab_topics = [
    "qunut subuh", "menyentuh wanita batal wudhu",
    "anjing najis", "makan kodok", "nikah tanpa wali",
    "talak tiga sekaligus", "shalat witir wajib",
    "membaca fatihah bagi makmum", "jumlah rakaat tarawih",
]
for topic in mazhab_topics:
    for mazhab in mazhab_names:
        add("mazhab", f"{topic} menurut {mazhab}")

# Pattern: "dalil X"
dalil_subjects = [
    "shalat lima waktu", "wajib puasa ramadhan",
    "wajib zakat", "wajib haji", "haram riba",
    "haram zina", "haram khamr", "wajib jilbab",
    "qunut subuh", "shalat tarawih",
]
for subject in dalil_subjects:
    add("dalil_x", f"dalil {subject}")

# ═══════════════════════════════════════════════════════════════
# 19. EDGE CASES & STRESS TESTS — ~200 queries
# ═══════════════════════════════════════════════════════════════

edge_cases = [
    # Single word queries
    "shalat", "wudhu", "puasa", "zakat", "haji", "nikah", "talak",
    "riba", "zina", "hudud", "qishas", "tauhid", "syirik",
    # Very short queries
    "hukum", "cara", "apa", "doa",
    # Very long colloquial queries
    "boleh gak makan nasi goreng yang dimasak sama alkohol dikit buat bumbu apakah haram",
    "kalau suami gak nafkahin istri selama 3 tahun terus istri minta cerai di pengadilan agama gimana hukumnya menurut mazhab syafi'i",
    "gimana caranya shalat tahajud yang bener kalau bangun tengah malam terus mau shalat witir juga setelahnya",
    "anak saya belum baligh tapi sudah pintar apakah sah shalatnya dan apakah wajib saya suruh shalat",
    "apakah boleh zakat fitrah dibayar dengan uang bukan beras kalau di daerah kami lebih mudah pakai uang",
    # Transliteration edge cases
    "sholat", "solat", "salah", "sholaat", "wudlu", "wudloo",
    "tayamum", "tayammum", "puoso", "shiyam", "hajj", "hadzj",
    # Misspellings
    "shalt", "wuhu", "pussa", "zkat",
    # Mixed script
    "hukum صلاة", "cara وضوء yang benar", "apakah ربا haram",
    # Numbers in queries
    "shalat 5 waktu", "rakaat shalat 4", "puasa 6 hari syawal",
    "talaq 3 kali", "istri 4 poligami", "zakat 2.5 persen",
    # Special characters
    "shalat jum'at", "i'tikaf", "ta'zir", "qadha'",
    "bid'ah", "mitsqal", "istiwa'",
]
for q in edge_cases:
    add("edge", q)

# ═══════════════════════════════════════════════════════════════
# 20. BAHTSUL MASAIL STYLE — ~200 queries
# ═══════════════════════════════════════════════════════════════

bahtsul_queries = [
    "bagaimana hukum shalat jumat bagi musafir yang singgah di suatu tempat kurang dari 4 hari",
    "apakah sah wudhu seseorang yang memakai kutek pada kukunya",
    "bagaimana hukum jual beli dengan sistem pre-order yang barangnya belum ada",
    "apakah boleh membayar zakat fitrah dengan uang menurut mazhab syafi'i",
    "bagaimana hukum menikah di bulan muharram atau safar",
    "apa hukum talak yang diucapkan suami dalam keadaan sangat marah",
    "bagaimana cara membagi warisan jika ahli waris hanya anak perempuan saja",
    "apakah boleh shalat dengan pakaian yang terkena tinta",
    "bagaimana hukum memakai parfum yang mengandung alkohol",
    "apakah sah puasa seseorang yang bangun kesiangan dan tidak sempat sahur",
    "bagaimana hukum bekerja sebagai driver ojek online yang mengantar pesanan makanan non halal",
    "apakah boleh wanita haid masuk masjid untuk mengambil barang",
    "bagaimana hukum shalat di atas kendaraan yang sedang berjalan",
    "apakah donor darah membatalkan puasa",
    "bagaimana hukum menjual barang yang masih dalam kredit",
    "apakah boleh imam shalat membaca mushaf saat shalat tarawih",
    "bagaimana hukum menyewakan tanah untuk ditanami tembakau",
    "apakah sah shalat jenazah tanpa memandikan mayit terlebih dahulu",
    "bagaimana hukum arisan dalam pandangan islam",
    "apakah boleh mengqadha shalat yang ditinggalkan selama bertahun-tahun",
    "bagaimana hukum menggabungkan niat puasa qadha dengan puasa sunnah",
    "apakah wajib menyembelih hewan kurban sendiri atau boleh diwakilkan",
    "bagaimana hukum suami yang tidak memberi nafkah lahir batin kepada istri",
    "apakah boleh berzakat kepada saudara kandung yang fakir",
    "bagaimana hukum jual beli emas secara online dengan transfer",
    "apakah hadits tentang keutamaan malam nisfu sya'ban itu shahih",
    "bagaimana hukum membaca al-quran untuk orang yang sudah meninggal",
    "apakah boleh shalat sunnah setelah shalat ashar",
    "bagaimana hukum memakai celana panjang di bawah mata kaki",
    "apakah wajib shalat sunnah rawatib",
    "bagaimana hukum bank syariah menurut ulama kontemporer",
    "apakah puasa hari sabtu saja dilarang",
    "bagaimana hukum wanita yang meninggalkan shalat karena haid apakah perlu qadha",
    "apakah boleh shalat dengan memakai masker",
    "bagaimana hukum menggunakan aplikasi pinjaman online berbunga",
    "apakah wajib memberikan mahar berupa uang atau boleh barang",
    "bagaimana hukum akad nikah melalui video call",
    "apakah boleh menunaikan shalat jumat di kantor",
    "bagaimana hukum wanita yang bepergian tanpa mahram",
    "apakah sah nikah tanpa adanya mas kawin",
]
for q in bahtsul_queries:
    add("bahtsul", q)

# ═══════════════════════════════════════════════════════════════
# 21. FURU' FIQHIYYAH DETAIL — ~400 queries
# ═══════════════════════════════════════════════════════════════

# Detailed sub-topics within each major chapter

# Shalat furu'
shalat_furu = [
    "bacaan tasyahud awal", "bacaan tasyahud akhir",
    "salam satu atau dua kali", "membaca basmalah keras atau pelan",
    "membaca fatihah bagi makmum", "amin keras atau pelan",
    "takbir intiqal hukumnya", "posisi tangan saat berdiri",
    "meletakkan tangan di dada atau perut", "posisi kaki saat duduk",
    "duduk iftirasy dan tawarruk", "shalat memakai sarung",
    "shalat memakai celana", "shalat memakai songkok", "shalat tanpa peci",
    "gerakan isyarat dalam shalat", "menangis saat shalat",
    "batuk saat shalat", "bersin saat shalat", "kentut saat shalat",
    "makmum masbuk berapa rakaat", "makmum masbuk shalat jumat",
    "sujud sahwi sebelum atau sesudah salam",
    "lupa tidak membaca tasyahud awal", "lupa berdiri padahal harus duduk",
    "shalat mundur atau maju karena sempit",
    "sholat di lorong masjid", "shalat di lantai dua masjid",
    "saf wanita di belakang pria", "anak kecil dalam saf shalat",
]
for q in shalat_furu:
    add("furu_shalat", q, "عبادات")

# Nikah furu'
nikah_furu = [
    "wali hakim kapan digunakan", "wali adhal menolak menikahkan",
    "wali mujbir siapa", "saksi dari kalangan perempuan",
    "ijab qabul bahasa Indonesia sah gak", "ijab qabul lewat telepon",
    "ijab qabul lewat video call", "nikah di bawah tangan",
    "nikah gantung tidak serumah", "malam pertama pengantin adab",
    "kewajiban suami malam pertama", "suami menolak berhubungan",
    "istri menolak berhubungan nusyuz", "nafkah kiswah pakaian istri",
    "nafkah maskan tempat tinggal", "nafkah anak yatim",
    "hadhanah sampai umur berapa", "anak ikut ibu atau bapak",
    "talak raj'i bisa rujuk", "rujuk caranya gimana",
]
for q in nikah_furu:
    add("furu_nikah", q, "مناكحات")

# Muamalat furu'
muamalat_furu = [
    "khiyar majlis pengertian", "khiyar syarat",
    "khiyar aib", "jaminan dalam jual beli",
    "jual beli barang cacat", "mengembalikan barang yang sudah dibeli",
    "riba dalam tukar menukar emas", "riba fadhl contoh",
    "riba nasiah contoh", "jual beli mata uang",
    "jual beli dengan harga tidak jelas", "jual beli ijon",
    "sewa menyewa rumah hukumnya", "sewa tanah pertanian",
    "bagi hasil pertanian muzara'ah", "musaqah pengertian",
    "syirkah abdan", "syirkah inan", "syirkah mufawadhah",
    "pinjaman tanpa bunga qardh", "menagih hutang yang tidak mampu bayar",
]
for q in muamalat_furu:
    add("furu_muamalat", q, "معاملات")

# ═══════════════════════════════════════════════════════════════
# 22. COMPARATIVE FIQH / KHILAF — ~200 queries
# ═══════════════════════════════════════════════════════════════

khilaf_topics = [
    "qunut subuh khilaf ulama", "membaca basmalah keras khilaf",
    "menyentuh wanita ajnabiyyah batal wudhu khilaf",
    "anjing najis atau tidak khilaf", "kodok halal atau haram khilaf",
    "nikah tanpa wali perbedaan mazhab", "talak tiga jatuh satu atau tiga",
    "jumlah rakaat tarawih 8 atau 20", "shalat witir satu atau tiga",
    "zakat profesi wajib atau tidak khilaf",
    "KB dalam islam perbedaan ulama", "rokok hukumnya khilaf",
    "musik hukumnya perbedaan pendapat",
    "cadar wajib atau sunnah perbedaan",
    "jenggot wajib memelihara menurut siapa",
    "memegang quran tanpa wudhu menurut empat mazhab",
    "berpuasa hari sabtu saja perbedaan pendapat",
    "shalat sunnah setelah ashar khilaf",
    "bid'ah hasanah ada atau tidak menurut ulama",
    "tawassul boleh atau tidak khilaf ulama",
]
for q in khilaf_topics:
    add("khilaf", q)

# ═══════════════════════════════════════════════════════════════
# 23. USUL FIQH — ~150 queries
# ═══════════════════════════════════════════════════════════════

usul_fiqh_topics = [
    "qiyas pengertian dan contoh", "istihsan dalam ushul fiqh",
    "maslahah mursalah", "istishab pengertian",
    "urf adat kebiasaan", "sadd adz dzari'ah",
    "maqashid syariah", "hifzh al nafs", "hifzh al aql",
    "hifzh al mal", "hifzh al din", "hifzh al nasab",
    "ijma pengertian", "ijtihad syarat", "taqlid hukumnya",
    "mujtahid mutlak", "muqallid pengertian",
    "nasikh mansukh", "am dan khas", "mutlaq dan muqayyad",
    "mujmal dan mubayyan", "haqiqi dan majazi",
    "wajib ain dan kifayah", "sunnah muakkadah",
    "makruh tanzih dan tahrim", "mubah pengertian",
    "rukhshah dan azimah", "dharurah dalam fiqh",
    "kaidah fiqhiyyah", "al umur bi maqashidiha",
    "al yaqin la yazul bi al syak", "al masyaqqah tajlib al taysir",
    "la dharar wa la dhirar", "al adah muhakkamah",
    "dar'u al mafasid muqaddam ala jalb al mashalih",
]
for q in usul_fiqh_topics:
    add("usul_fiqh", q)

# ═══════════════════════════════════════════════════════════════
# 24. SIRAH & TARIKH — ~100 queries
# ═══════════════════════════════════════════════════════════════

sirah_topics = [
    "perang badr", "perang uhud", "perang khandaq",
    "fathu makkah", "perjanjian hudaibiyah",
    "hijrah nabi ke madinah", "isra miraj",
    "piagam madinah", "khulafaur rasyidin",
    "khalifah abu bakar", "khalifah umar bin khattab",
    "khalifah utsman bin affan", "khalifah ali bin abi thalib",
    "bani umayyah", "bani abbasiyah",
    "imam syafi'i biografi", "imam malik biografi",
    "imam hanafi biografi", "imam ahmad bin hanbal",
    "imam ghazali pemikiran", "imam nawawi karya",
    "ibnu taimiyah pendapat", "ibnu qayyim pemikiran",
]
for q in sirah_topics:
    add("sirah", q)

# ═══════════════════════════════════════════════════════════════
# FINAL ASSEMBLY
# ═══════════════════════════════════════════════════════════════

# Deduplicate by text (keep first occurrence)
seen = set()
unique_queries = []
for q in queries:
    key = q["text"].strip().lower()
    if key not in seen:
        seen.add(key)
        unique_queries.append(q)

# Reassign sequential IDs
for i, q in enumerate(unique_queries):
    q["id"] = f"q{i+1:05d}"

print(f"Generated {len(unique_queries)} unique queries across categories:")
cats = {}
for q in unique_queries:
    cats[q["category"]] = cats.get(q["category"], 0) + 1
for cat, count in sorted(cats.items()):
    print(f"  {cat}: {count}")

# Save the query list
with open("queries_10k.json", "w", encoding="utf-8") as f:
    json.dump(unique_queries, f, ensure_ascii=False, indent=1)

print(f"\nSaved to queries_10k.json")

# Also generate the eval format (just id + text for the batch API)
eval_queries = [{"id": q["id"], "text": q["text"]} for q in unique_queries]
with open("queries_10k_eval.json", "w", encoding="utf-8") as f:
    json.dump(eval_queries, f, ensure_ascii=False, indent=1)

print(f"Saved eval format to queries_10k_eval.json")
