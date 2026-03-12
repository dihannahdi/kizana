#!/usr/bin/env python3
"""Final push to 10K+."""
import json

with open("queries_10k.json", "r", encoding="utf-8") as f:
    queries = json.load(f)

seen = {q["text"].strip().lower() for q in queries}

def add(cat, text, domain=None, lang="id"):
    key = text.strip().lower()
    if key not in seen:
        seen.add(key)
        queries.append({"id":"","text":text,"category":cat,"expected_domain":domain,"lang":lang})

# A: Comparative mazhab on specific issues
mazahib = ["syafii", "hanafi", "maliki", "hanbali"]
issues = [
    "menyentuh wanita batal wudhu", "membaca fatihah bagi makmum",
    "niat puasa dari malam hari", "zakat fitrah berupa uang",
    "qunut subuh", "shalat witir wajib atau sunnah",
    "nikah tanpa wali", "talak dalam keadaan marah",
    "mengusap sebagian atau seluruh kepala", "berniat puasa di siang hari",
    "shalat tarawih 8 atau 20 rakaat", "najis anjing",
    "mengangkat tangan setelah takbiratul ihram",
    "doa iftitah wajib atau sunnah", "tasyahud awal wajib atau sunnah",
    "shalat berjamaah fardhu kifayah atau ain",
    "membaca basmalah keras dalam fatihah",
    "makan ikan tanpa disembelih",
    "darah yang keluar selain haid",
    "batas waktu mengusap khuf",
]
for mazhab in mazahib:
    for issue in issues:
        add("mazhab_issue", f"pendapat mazhab {mazhab} tentang {issue}")

# B: Historical / biographical
figures = [
    "Abu Bakar ash Shiddiq", "Umar bin Khattab", "Utsman bin Affan",
    "Ali bin Abi Thalib", "Khadijah binti Khuwailid",
    "Aisyah binti Abu Bakar", "Fatimah az Zahra",
    "Hamzah bin Abdul Muthalib", "Khalid bin Walid",
    "Salahuddin al Ayyubi", "Ibn Sina", "Al Khawarizmi",
    "Al Ghazali", "Ibn Rusyd", "Ibn Khaldun",
    "Imam Bukhari", "Imam Muslim", "Imam Nawawi",
    "Ibn Hajar al Asqalani", "Jalaluddin as Suyuthi",
]
bio_q = ["siapakah", "biografi singkat", "kontribusi", "karya-karya"]
for fig in figures:
    for q in bio_q:
        add("biografi", f"{q} {fig}")

# C: Dua / supplication specific queries
dua_topics = [
    "doa masuk masjid", "doa keluar masjid",
    "doa masuk kamar mandi", "doa keluar kamar mandi",
    "doa sebelum makan", "doa sesudah makan",
    "doa sebelum tidur", "doa bangun tidur",
    "doa naik kendaraan", "doa bepergian",
    "doa masuk pasar", "doa memakai baju baru",
    "doa ketika hujan turun", "doa ketika mendengar petir",
    "doa setelah adzan", "doa antara adzan dan iqamah",
    "doa qunut nazilah", "doa istiftah",
    "doa sujud", "doa duduk di antara dua sujud",
    "doa tasyahud akhir", "doa setelah salam",
    "doa ketika sakit", "doa menjenguk orang sakit",
    "doa untuk mayit", "doa masuk kuburan",
    "doa tahun baru hijriyah", "doa awal dan akhir tahun",
    "doa malam lailatul qadr", "doa hari arafah",
    "doa mohon keturunan", "doa mohon jodoh",
    "doa mohon rezeki", "doa mohon perlindungan",
    "doa ketika marah", "doa ketika takut",
    "doa ketika gempa", "doa ketika banjir",
    "doa kafarat majelis",
]
for d in dua_topics:
    add("dua_specific", d)

# D: Fill remaining with more Arabic
ar_extra = [
    "فقه المرأة في الإسلام", "أحكام الحيض والنفاس",
    "الاغتسال من الحيض", "مدة الحيض والاستحاضة",
    "أقل الحيض وأكثره", "حكم الصلاة للمستحاضة",
    "أحكام الجنابة", "ما يوجب الغسل",
    "أحكام المسح على الخفين", "مدة المسح على الخفين للمقيم والمسافر",
    "حكم الماء المستعمل", "الماء الطهور والطاهر والنجس",
    "حكم الاستنجاء بالحجارة", "آداب قضاء الحاجة",
    "حكم إزالة النجاسة", "أنواع النجاسات",
    "نجاسة الكلب والخنزير", "حكم بول الرضيع",
    "حكم السواك", "سنن الفطرة",
    "أحكام الختان", "حكم حلق اللحية",
    "حكم إسبال الإزار", "حكم لبس الذهب للرجال",
    "حكم لبس الحرير للرجال", "خاتم الذهب والفضة",
    "أحكام اللباس في الإسلام", "ستر العورة في الصلاة",
    "عورة المرأة أمام المرأة", "عورة المرأة أمام المحارم",
    "حكم كشف الوجه للمرأة", "حكم النقاب",
    "صلاة التطوع وأنواعها", "صلاة الضحى فضلها ووقتها",
    "صلاة التهجد والقيام", "صلاة الحاجة",
    "صلاة التوبة", "صلاة الشكر",
    "سجود التلاوة أحكامه", "سجود الشكر أحكامه",
]
for q in ar_extra:
    add("ar_extra", q, None, "ar")

# ASSEMBLE
for i, q in enumerate(queries):
    q["id"] = f"q{i+1:05d}"

print(f"Total: {len(queries)}")

with open("queries_10k.json", "w", encoding="utf-8") as f:
    json.dump(queries, f, ensure_ascii=False, indent=1)

eval_queries = [{"id": q["id"], "text": q["text"]} for q in queries]
with open("queries_10k_eval.json", "w", encoding="utf-8") as f:
    json.dump(eval_queries, f, ensure_ascii=False, indent=1)

print(f"Saved {len(queries)} queries")
