#!/usr/bin/env python3
"""
Final expansion round to reach 10K+ queries.
"""
import json
import random
random.seed(42)

with open("queries_10k.json", "r", encoding="utf-8") as f:
    queries = json.load(f)

seen = {q["text"].strip().lower() for q in queries}

def add(category, text, expected_domain=None, lang="id"):
    key = text.strip().lower()
    if key in seen:
        return
    seen.add(key)
    queries.append({
        "id": "", "text": text, "category": category,
        "expected_domain": expected_domain, "lang": lang
    })

# ═══════════════════════════════════════════════════════════════
# EXPANSION A: Prefix × Topic matrix (3000+ queries)
# ═══════════════════════════════════════════════════════════════

prefixes = [
    "hukum", "pengertian", "definisi", "makna",
    "hikmah", "manfaat", "bahaya", "dalil",
    "sejarah", "asal mula", "pendapat ulama tentang",
    "hadits tentang", "ayat tentang", "kaidah tentang",
    "perbedaan pendapat tentang",
]

topics_large = [
    # Ibadah
    "shalat wajib", "shalat sunnah", "shalat jumat", "shalat jenazah",
    "shalat tahajjud", "shalat dhuha", "shalat tarawih", "shalat witir",
    "shalat ied", "shalat gerhana", "shalat istikharah", "shalat istisqa",
    "shalat jamaah", "shalat munfarid", "shalat qasar", "shalat jamak",
    "wudhu", "tayammum", "mandi wajib", "mandi sunnah",
    "puasa ramadhan", "puasa sunnah", "puasa nadzar", "puasa kafarat",
    "zakat mal", "zakat fitrah", "zakat emas", "zakat pertanian",
    "haji wajib", "umrah wajib", "tawaf", "sa'i", "wukuf",
    # Muamalat
    "jual beli", "sewa menyewa", "gadai", "pinjaman",
    "hutang", "riba", "gharar", "maysir",
    "mudharabah", "musyarakah", "murabahah", "ijarah",
    "salam", "istishna", "wakalah", "kafalah",
    "wakaf", "hibah", "wasiat", "luqathah",
    "syuf'ah", "ji'alah", "qardh",
    # Nikah
    "nikah", "talak", "khuluk", "fasakh",
    "iddah", "rujuk", "nafkah", "mahar",
    "poligami", "nusyuz", "syiqaq", "ila",
    "zihar", "li'an", "radha'ah", "hadhanah",
    # Jinayat
    "qishas", "diyat", "hudud", "ta'zir",
    "had sariqah", "had zina", "had khamr", "had qadzaf",
    "hirabah", "bughat", "riddah",
    # Aqidah
    "tauhid", "syirik", "iman", "kufur",
    "nifaq", "bid'ah", "tawassul", "karamah",
    # Tasawuf
    "taubat", "ikhlas", "riya", "ujub",
    "hasad", "ghibah", "namimah", "kibr",
    "zuhud", "wara", "tawakkal", "sabar",
    "syukur", "khauf", "raja", "mahabbah",
    # Usul fiqh
    "qiyas", "ijma", "ijtihad", "istihsan",
    "istishab", "maslahah mursalah", "urf",
    "sadd adz dzariah", "maqashid syariah",
]

for prefix in prefixes:
    for topic in topics_large:
        add("matrix", f"{prefix} {topic}")

# ═══════════════════════════════════════════════════════════════
# EXPANSION B: "Apakah X termasuk Y" pattern
# ═══════════════════════════════════════════════════════════════

category_pairs = [
    ("sunnah", ["mandi sebelum jumat", "siwak", "kumur-kumur", "menghirup air",
                "mendahulukan kanan", "doa setelah wudhu", "memakai minyak wangi",
                "berjenggot", "memakai surban", "makan dengan tangan kanan",
                "tidur miring kanan", "membaca basmalah sebelum makan"]),
    ("wajib", ["shalat lima waktu", "puasa ramadhan", "zakat", "haji",
               "menutup aurat", "berbakti kepada orang tua", "menjaga jiwa",
               "niat dalam wudhu", "membaca fatihah dalam shalat"]),
    ("haram", ["riba", "zina", "khamr", "judi", "suap", "korupsi",
               "ghibah", "namimah", "bunuh diri", "menyiksa hewan",
               "memakan bangkai", "darah yang mengalir", "daging babi",
               "memakai emas bagi laki-laki", "memakai sutra bagi laki-laki"]),
    ("makruh", ["makan bawang mentah", "menguap tanpa ditutup",
                "membuang ingus di masjid", "menoleh-noleh dalam shalat",
                "memejamkan mata dalam shalat", "berdiri satu kaki"]),
    ("mubah", ["makan daging ayam", "minum kopi", "minum teh",
               "memakai pakaian warna hitam", "tidur siang",
               "bermain olahraga", "berenang", "berkuda", "memanah"]),
    ("bid'ah", ["tahlilan", "yasinan", "maulid nabi", "nisfu sya'ban",
                "shalat raghaib", "qunut nazilah", "dzikir berjamaah",
                "hizb", "ratib", "tawassul"]),
    ("rukun islam", ["syahadat", "shalat", "zakat", "puasa", "haji"]),
    ("rukun iman", ["iman kepada allah", "malaikat", "kitab", "rasul",
                    "hari akhir", "qadha qadar"]),
    ("dosa besar", ["syirik", "membunuh", "zina", "minum khamr",
                    "durhaka orang tua", "sihir", "memakan riba",
                    "memakan harta anak yatim", "lari dari perang",
                    "menuduh zina wanita baik"]),
    ("najis", ["darah", "nanah", "muntah", "air kencing", "kotoran",
               "mani", "madzi", "wadi", "air liur anjing", "babi",
               "khamr", "bangkai"]),
]

for category, items in category_pairs:
    for item in items:
        add("termasuk", f"apakah {item} termasuk {category}")

# ═══════════════════════════════════════════════════════════════
# EXPANSION C: Imam/Ulama-specific queries
# ═══════════════════════════════════════════════════════════════

ulama_names = [
    "imam syafi'i", "imam malik", "imam abu hanifah", "imam ahmad",
    "imam ghazali", "imam nawawi", "ibnu taimiyah", "ibnu qayyim",
    "ibnu hajar", "imam suyuthi", "imam ramli", "ibnu hajar haitami",
    "imam subki", "imam isnawi", "imam zarkasyi", "imam bulqini",
    "imam rafi'i", "imam muzani", "imam bujairimi", "imam bajuri",
    "syekh ibrahim al bajuri", "syekh nawawi al bantani",
    "syekh mahfudh at tarmasi", "syekh ihsan jampes",
    "imam zakaria al anshari", "imam khatib syarbini",
]

ulama_topics = [
    "biografi", "karya tulis", "pendapat tentang qunut",
    "mazhab", "murid-murid", "guru-guru",
    "pendapat tentang bid'ah", "kitab terkenal",
]

for ulama in ulama_names:
    for topic in ulama_topics:
        add("ulama", f"{topic} {ulama}")

# ═══════════════════════════════════════════════════════════════
# EXPANSION D: Time-specific queries
# ═══════════════════════════════════════════════════════════════

months_hijri = [
    "muharram", "safar", "rabiul awal", "rabiul akhir",
    "jumadil awal", "jumadil akhir", "rajab", "sya'ban",
    "ramadhan", "syawal", "dzulqa'dah", "dzulhijjah"
]

for month in months_hijri:
    add("bulan", f"amalan di bulan {month}")
    add("bulan", f"keutamaan bulan {month}")
    add("bulan", f"puasa di bulan {month}")
    add("bulan", f"peristiwa penting bulan {month}")

days = ["senin", "selasa", "rabu", "kamis", "jumat", "sabtu", "ahad"]
for day in days:
    add("hari", f"amalan di hari {day}")
    add("hari", f"puasa hari {day}")
    add("hari", f"keutamaan hari {day}")

# Night-specific
nights = ["lailatul qadr", "malam nisfu sya'ban", "malam jumat",
          "malam isra miraj", "malam maulid nabi"]
for night in nights:
    add("malam", f"amalan {night}")
    add("malam", f"keutamaan {night}")

# ═══════════════════════════════════════════════════════════════
# EXPANSION E: Place-specific queries
# ═══════════════════════════════════════════════════════════════

places = [
    "masjid", "mushalla", "rumah", "kantor", "sekolah",
    "pasar", "kuburan", "kamar mandi", "dapur",
    "masjidil haram", "masjid nabawi", "masjidil aqsha",
    "arafah", "mina", "muzdalifah", "jabal rahmah",
    "gua hira", "gua tsur", "makam nabi",
]

for place in places:
    add("tempat", f"hukum shalat di {place}")
    add("tempat", f"adab memasuki {place}")

# ═══════════════════════════════════════════════════════════════
# EXPANSION F: "Bagaimana jika" conditional queries
# ═══════════════════════════════════════════════════════════════

conditions = [
    "bagaimana jika imam salah bacaan",
    "bagaimana jika ragu sudah wudhu atau belum",
    "bagaimana jika lupa jumlah rakaat shalat",
    "bagaimana jika terlambat shalat jumat",
    "bagaimana jika tidak mampu berdiri saat shalat",
    "bagaimana jika air wudhu terkontaminasi",
    "bagaimana jika muntah saat puasa",
    "bagaimana jika menelan dahak saat puasa",
    "bagaimana jika tidak ada air untuk wudhu",
    "bagaimana jika tidak ada tanah untuk tayammum",
    "bagaimana jika zakat tidak sampai ke mustahik",
    "bagaimana jika mahar belum dibayar saat meninggal",
    "bagaimana jika suami hilang tanpa kabar",
    "bagaimana jika saksi nikah ternyata fasiq",
    "bagaimana jika ijab qabul tidak sesuai",
    "bagaimana jika wali nikah menolak tanpa alasan",
    "bagaimana jika talak diucapkan dalam bahasa isyarat",
    "bagaimana jika suami murtad setelah nikah",
    "bagaimana jika istri murtad setelah nikah",
    "bagaimana jika pembeli menemukan cacat pada barang",
    "bagaimana jika penjual menyembunyikan cacat barang",
    "bagaimana jika barang yang dibeli hilang sebelum diterima",
    "bagaimana jika salah satu pihak meninggal sebelum akad selesai",
    "bagaimana jika hutang tidak mampu dibayar",
    "bagaimana jika pewaris meninggalkan hutang lebih besar dari harta",
    "bagaimana jika ahli waris berbeda agama",
    "bagaimana jika ahli waris membunuh pewaris",
    "bagaimana jika mayit tidak ada yang memandikan",
    "bagaimana jika mayit tidak diketahui agamanya",
    "bagaimana jika jenazah tenggelam di laut",
    "bagaimana jika tidak menemukan kiblat",
    "bagaimana jika waktu shalat hampir habis",
    "bagaimana jika khutbah jumat menggunakan bahasa Indonesia",
    "bagaimana jika imam shalat batal wudhunya",
    "bagaimana jika ada hadats saat tawaf",
    "bagaimana jika terpisah dari rombongan haji",
    "bagaimana jika melewati miqat tanpa ihram",
]
for q in conditions:
    add("conditional", q)

# ═══════════════════════════════════════════════════════════════
# EXPANSION G: "Apa saja" enumeration queries
# ═══════════════════════════════════════════════════════════════

apa_saja = [
    "apa saja rukun shalat", "apa saja syarat wudhu",
    "apa saja pembatal wudhu", "apa saja hal yang membatalkan puasa",
    "apa saja rukun nikah", "apa saja syarat sah jual beli",
    "apa saja macam-macam najis", "apa saja macam-macam air",
    "apa saja macam-macam shalat sunnah", "apa saja macam-macam puasa sunnah",
    "apa saja macam-macam zakat", "apa saja rukun haji",
    "apa saja wajib haji", "apa saja larangan saat ihram",
    "apa saja sunnah wudhu", "apa saja sunnah ab'adh shalat",
    "apa saja dosa besar dalam islam", "apa saja rukun islam",
    "apa saja rukun iman", "apa saja kitab hadits enam",
    "apa saja kitab tafsir terkenal", "apa saja mazhab empat",
    "apa saja syarat menjadi imam", "apa saja syarat menjadi muadzin",
    "apa saja syarat menjadi khotib", "apa saja syarat menjadi saksi",
    "apa saja hak suami dalam islam", "apa saja hak istri dalam islam",
    "apa saja hak anak dalam islam", "apa saja kewajiban orang tua",
    "apa saja maqashid syariah", "apa saja kaidah fiqhiyyah",
    "apa saja sumber hukum islam", "apa saja dalil naqli dan aqli",
]
for q in apa_saja:
    add("apa_saja", q)

# ═══════════════════════════════════════════════════════════════
# EXPANSION H: English systematic
# ═══════════════════════════════════════════════════════════════

en_prefixes = ["ruling on", "conditions for", "is it permissible to",
               "evidence for", "wisdom behind", "types of",
               "difference between", "scholarly opinion on"]

en_topics = [
    "prayer", "fasting", "zakah", "hajj", "marriage", "divorce",
    "inheritance", "sale", "lease", "interest", "gambling",
    "alcohol", "pork", "music", "photography", "tattoo",
    "organ donation", "IVF", "insurance", "stocks", "cryptocurrency",
    "smoking", "vaping", "hijab", "niqab", "beard", "gold for men",
    "silk for men", "abortion", "contraception", "cloning",
]

for prefix in en_prefixes:
    for topic in en_topics:
        add("en_matrix", f"{prefix} {topic}", None, "en")

# ═══════════════════════════════════════════════════════════════
# EXPANSION I: Arabic systematic
# ═══════════════════════════════════════════════════════════════

ar_prefixes = ["حكم", "شروط", "أركان", "أنواع", "فضل", "دليل", "تعريف"]

ar_topics = [
    "الصلاة", "الصيام", "الزكاة", "الحج", "العمرة",
    "النكاح", "الطلاق", "البيع", "الإجارة", "الربا",
    "الوقف", "الوصية", "الميراث", "القصاص", "الحدود",
    "التوبة", "الإخلاص", "التوحيد", "الشرك", "البدعة",
    "الجهاد", "الذكاة", "الأضحية", "العقيقة", "الكفارة",
]

for prefix in ar_prefixes:
    for topic in ar_topics:
        add("ar_matrix", f"{prefix} {topic}", None, "ar")

# ═══════════════════════════════════════════════════════════════
# EXPANSION J: Common misspellings and transliteration errors
# ═══════════════════════════════════════════════════════════════

misspellings = [
    ("sholat", "shalat"), ("solat", "shalat"), ("salat", "shalat"),
    ("wudhlu", "wudhu"), ("wudlu", "wudhu"), ("wudu", "wudhu"),
    ("puoso", "puasa"), ("shiyam", "puasa"),
    ("zakaat", "zakat"), ("dzakat", "zakat"),
    ("hajj", "haji"), ("hadzj", "haji"),
    ("tayamum", "tayammum"), ("tayyamum", "tayammum"),
    ("talaq", "talak"), ("tholaq", "talak"),
    ("nikakh", "nikah"), ("nikaah", "nikah"),
    ("ruko'", "ruku"), ("sujud", "sujud"),
    ("khulug", "khuluk"), ("fasah", "fasakh"),
]

misspelling_templates = [
    "hukum {}", "cara {} yang benar", "syarat {}",
    "tata cara {}", "rukun {}"
]

for wrong, right in misspellings:
    for template in misspelling_templates:
        add("misspell", template.format(wrong))

# ═══════════════════════════════════════════════════════════════
# EXPANSION K: "Menurut" attribution queries
# ═══════════════════════════════════════════════════════════════

menurut_sources = [
    "menurut al quran", "menurut hadits", "menurut ulama",
    "menurut MUI", "menurut NU", "menurut Muhammadiyah",
    "menurut mazhab syafi'i", "menurut mazhab hanafi",
    "menurut mazhab maliki", "menurut mazhab hanbali",
    "menurut imam ghazali", "menurut ibnu taimiyah",
    "menurut jumhur ulama", "menurut qaul mu'tamad",
]

menurut_topics = [
    "hukum musik", "hukum rokok", "hukum cadar",
    "talak tiga sekaligus", "qunut subuh",
    "maulid nabi", "tahlilan", "ziarah kubur",
    "hukum asuransi", "hukum bank konvensional",
    "jilbab wajib", "nikah siri",
]

for source in menurut_sources:
    for topic in menurut_topics:
        add("menurut", f"{topic} {source}")

# ═══════════════════════════════════════════════════════════════
# EXPANSION L: "Apakah boleh" permission queries
# ═══════════════════════════════════════════════════════════════

boleh_items = [
    "shalat tanpa menutup kepala", "shalat dengan celana pendek",
    "shalat dengan baju bergambar", "shalat di atas kasur",
    "shalat di dalam mobil", "wudhu dengan air panas",
    "wudhu dengan air mineral", "tayammum di tembok",
    "puasa tanpa makan sahur", "membayar zakat dengan barang",
    "haji dengan uang pinjaman", "umrah untuk orang lain",
    "menikah tanpa restu orang tua", "menikah via telepon",
    "menceraikan istri yang sedang hamil", "poligami tanpa izin istri pertama",
    "menjual barang yang belum diterima", "menjual barang yang masih dicicil",
    "menggadaikan barang milik orang lain", "meminjamkan barang yang dipinjam",
    "berzakat kepada orang tua", "berzakat kepada anak sendiri",
    "berzakat kepada non muslim", "menggunakan uang zakat untuk membangun masjid",
    "menggabungkan shalat tanpa sebab safar", "shalat jumat di rumah",
    "imam shalat membaca dari mushaf", "shalat dengan bahasa isyarat",
    "puasa setengah hari", "menukar fidyah puasa dengan uang",
    "kurban satu kambing untuk sekeluarga", "aqiqah setelah dewasa",
    "menikahkan diri sendiri tanpa wali", "wanita melamar laki-laki",
    "membaca quran tanpa suara", "membaca quran sambil tiduran",
    "membaca quran dari handphone", "shalat sambil memakai headset",
]
for item in boleh_items:
    add("boleh", f"apakah boleh {item}")

# ═══════════════════════════════════════════════════════════════
# EXPANSION M: Specific number queries
# ═══════════════════════════════════════════════════════════════

number_queries = [
    "berapa rakaat shalat subuh", "berapa rakaat shalat dzuhur",
    "berapa rakaat shalat ashar", "berapa rakaat shalat maghrib",
    "berapa rakaat shalat isya", "berapa rakaat shalat jumat",
    "berapa rakaat shalat tarawih", "berapa rakaat shalat witir",
    "berapa rakaat shalat dhuha", "berapa rakaat shalat tahajjud",
    "berapa kali sujud sahwi", "berapa kali takbir shalat ied",
    "berapa kali talak yang bisa rujuk", "berapa lama masa iddah",
    "berapa nisab zakat emas dalam gram", "berapa persen zakat penghasilan",
    "berapa nisab zakat perak", "berapa kg zakat fitrah",
    "berapa hari puasa ramadhan minimal", "berapa hari puasa kafarat",
    "berapa orang minimal shalat jumat", "berapa saksi dalam nikah",
    "berapa batas usia menikah dalam islam", "berapa minimal mahar",
]
for q in number_queries:
    add("berapa", q)

# ═══════════════════════════════════════════════════════════════
# ASSEMBLE FINAL
# ═══════════════════════════════════════════════════════════════

for i, q in enumerate(queries):
    q["id"] = f"q{i+1:05d}"

print(f"Total unique queries: {len(queries)}")
cats = {}
for q in queries:
    cats[q["category"]] = cats.get(q["category"], 0) + 1
for cat, count in sorted(cats.items(), key=lambda x: -x[1]):
    print(f"  {cat}: {count}")

with open("queries_10k.json", "w", encoding="utf-8") as f:
    json.dump(queries, f, ensure_ascii=False, indent=1)

eval_queries = [{"id": q["id"], "text": q["text"]} for q in queries]
with open("queries_10k_eval.json", "w", encoding="utf-8") as f:
    json.dump(eval_queries, f, ensure_ascii=False, indent=1)

print(f"\nSaved {len(queries)} queries")
