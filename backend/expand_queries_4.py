#!/usr/bin/env python3
"""Push past 10K with final cross-products."""
import json

with open("queries_10k.json", "r", encoding="utf-8") as f:
    queries = json.load(f)

seen = {q["text"].strip().lower() for q in queries}

def add(cat, text, domain=None, lang="id"):
    key = text.strip().lower()
    if key not in seen:
        seen.add(key)
        queries.append({"id":"","text":text,"category":cat,"expected_domain":domain,"lang":lang})

# ═══ A: Verb prefix × topic (large cross-product) ═══

verbs_id = [
    "apakah wajib", "apakah haram", "apakah sunnah", "apakah makruh",
    "kapan waktu", "dimana tempat", "mengapa harus", "siapa yang wajib",
    "apa dalil", "apa hikmah", "apa syarat", "apa rukun",
    "apa yang membatalkan", "apa yang dilarang saat", "apa yang dilakukan setelah",
    "apa doa sebelum", "apa doa sesudah", "bagaimana tata cara",
    "berapa kadar", "berapa minimal",
]

topics_id = [
    "shalat dhuha", "shalat tahajjud", "shalat istikharah",
    "shalat tarawih", "shalat witir", "shalat ied",
    "shalat jenazah", "shalat istisqa", "shalat kusuf",
    "sujud sahwi", "sujud tilawah", "sujud syukur",
    "tayammum", "istinja", "mandi junub",
    "mandi wajib", "ghusl", "wudhu",
    "puasa senin kamis", "puasa dawud", "puasa asyura",
    "puasa arafah", "puasa qadha", "puasa nadzar",
    "puasa kafarat", "itikaf", "fidyah puasa",
    "zakat fitrah", "zakat mal", "zakat penghasilan",
    "zakat emas", "zakat pertanian", "zakat perdagangan",
    "haji wada", "thawaf", "sa'i",
    "wukuf arafah", "mabit muzdalifah", "melontar jumrah",
    "tahallul", "dam haji", "badal haji",
    "aqiqah", "qurban", "walimah",
    "khitan", "adzanm", "iqamah",
    "khutbah jumat", "qunut", "wirid",
    "tawassul", "istighatsah", "ratib",
]

for verb in verbs_id:
    for topic in topics_id:
        add("verb_topic", f"{verb} {topic}")

# ═══ B: More English verb × topic ═══

verbs_en = [
    "is it permissible to", "what is the ruling on",
    "how to perform", "what are the conditions for",
    "who is obligated to", "when should one",
    "what invalidates", "what is recommended before",
]

topics_en = [
    "tayammum", "ghusl after menstruation", "witr prayer",
    "funeral prayer", "rain prayer", "eclipse prayer",
    "prostration of forgetfulness", "prostration of recitation",
    "itikaf in the last ten days", "paying fidyah",
    "combining prayers while traveling", "shortening prayers",
    "praying on an airplane", "praying in a hospital",
    "fasting while pregnant", "fasting while breastfeeding",
    "giving zakat to non-Muslims", "paying zakat on jewelry",
    "performing hajj on behalf of someone", "slaughtering on Eid",
    "reciting Quran during menstruation", "touching Quran without wudu",
    "making dua in sajdah", "raising hands in dua",
]

for verb in verbs_en:
    for topic in topics_en:
        add("en_verb_topic", f"{verb} {topic}", None, "en")

# ═══ C: Arabic verb pattern × noun ═══

ar_verbs = ["حكم", "شروط", "أركان", "واجبات", "سنن", "مبطلات", "موانع", "آداب"]
ar_nouns = [
    "الوضوء", "الغسل", "التيمم", "الصلاة", "الجمعة",
    "الجنازة", "العيدين", "الاستسقاء", "الكسوف", "الخسوف",
    "الصيام", "الاعتكاف", "الزكاة", "الحج", "العمرة",
    "الطواف", "السعي", "الوقوف بعرفة", "المبيت بمزدلفة",
    "رمي الجمرات", "النكاح", "الطلاق", "الخلع", "الرجعة",
    "الإيلاء", "الظهار", "اللعان", "العدة", "الرضاع",
    "البيع", "الإجارة", "الشركة", "المضاربة", "الوكالة",
    "الكفالة", "الرهن", "الحوالة", "الهبة", "الوقف",
    "الوصية", "القضاء", "الشهادة", "الإقرار", "اليمين",
]

for verb in ar_verbs:
    for noun in ar_nouns:
        add("ar_verb_noun", f"{verb} {noun}", None, "ar")

# ═══ D: Numbers + Islamic concepts ═══

number_queries = [
    "5 rukun islam", "6 rukun iman", "4 mazhab fiqih",
    "7 anggota sujud", "13 rukun shalat", "8 syarat shalat",
    "6 syarat wajib shalat", "12 syarat sah shalat menurut syafii",
    "10 hal yang membatalkan wudhu", "6 hal yang mewajibkan mandi",
    "28 huruf hijaiyah", "30 juz quran", "114 surat quran",
    "6 kitab hadits utama", "40 hadits nawawi",
    "99 asmaul husna", "25 nabi yang disebutkan dalam quran",
    "10 sahabat yang dijamin masuk surga",
    "4 khalifah rasyidin", "5 imam mazhab terkenal",
    "3 tingkatan ihsan", "7 lapisan langit",
    "7 lapisan bumi", "8 pintu surga", "7 pintu neraka",
    "3 pertanyaan kubur", "10 tanda kiamat besar",
    "5 shalat wajib dan waktunya", "2 shalat ied",
    "3 jenis air dalam bersuci", "7 benda najis menurut syafii",
    "4 jenis talak", "3 macam iddah",
]
for q in number_queries:
    add("angka", q)

# ═══ E: "Menurut kitab X" ═══

kitab_queries = [
    "menurut kitab ihya ulumuddin", "menurut kitab riyadhus shalihin",
    "menurut kitab bulughul maram", "menurut kitab fathul bari",
    "menurut kitab al umm", "menurut kitab al mughni",
    "menurut kitab al majmu syarh muhadzdzab",
    "menurut kitab minhaj at thalibin", "menurut kitab fathul qadir",
    "menurut kitab bidayatul mujtahid", "menurut kitab al muqaddimah",
    "menurut kitab tuhfatul muhtaj", "menurut kitab nihayatul muhtaj",
    "menurut kitab raudhatul thalibin", "menurut kitab al muhalla",
]

kitab_topics = [
    "hukum shalat berjamaah", "hukum musik", "syarat nikah",
    "pembagian waris", "hukum merokok", "niat puasa",
]

for kitab in kitab_queries:
    for topic in kitab_topics:
        add("kitab_ref", f"{topic} {kitab}")

# ═══ ASSEMBLE ═══

for i, q in enumerate(queries):
    q["id"] = f"q{i+1:05d}"

print(f"Total: {len(queries)}")
cats = {}
for q in queries:
    cats[q["category"]] = cats.get(q["category"], 0) + 1
for cat, count in sorted(cats.items(), key=lambda x: -x[1])[:15]:
    print(f"  {cat}: {count}")
print(f"  ... and {len(cats)-15} more categories")

with open("queries_10k.json", "w", encoding="utf-8") as f:
    json.dump(queries, f, ensure_ascii=False, indent=1)

eval_queries = [{"id": q["id"], "text": q["text"]} for q in queries]
with open("queries_10k_eval.json", "w", encoding="utf-8") as f:
    json.dump(eval_queries, f, ensure_ascii=False, indent=1)

print(f"Saved {len(queries)} queries")
