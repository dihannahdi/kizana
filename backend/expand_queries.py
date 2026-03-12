#!/usr/bin/env python3
"""
Expand the 10K query suite with more combinatorial and parametric patterns.
Appends to the existing queries_10k.json.
"""
import json
import random
random.seed(42)

# Load existing queries
with open("queries_10k.json", "r", encoding="utf-8") as f:
    queries = json.load(f)

seen = {q["text"].strip().lower() for q in queries}

def add(category, text, expected_domain=None, lang="id"):
    key = text.strip().lower()
    if key in seen:
        return
    seen.add(key)
    queries.append({
        "id": "",
        "text": text,
        "category": category,
        "expected_domain": expected_domain,
        "lang": lang
    })

# ═══════════════════════════════════════════════════════════════
# MEGA EXPANSION 1: Cross-product of actors × actions × objects
# ═══════════════════════════════════════════════════════════════

actors = [
    "orang sakit", "musafir", "wanita hamil", "ibu menyusui",
    "anak kecil", "orang tua renta", "orang buta",
    "orang lumpuh", "orang gila", "orang mabuk",
    "muallaf", "kafir dzimmi", "budak", "anak yatim",
    "orang junub", "wanita haid", "wanita nifas",
    "wanita istihadzah", "imam shalat", "makmum",
    "khotib jumat", "muadzin", "pengantin baru",
    "jemaah haji", "orang yang ber-i'tikaf",
    "orang yang berpuasa", "orang yang sedang ihram",
]

actions_ibadah = [
    "shalat", "berpuasa", "membayar zakat", "menunaikan haji",
    "melakukan umrah", "membaca quran", "shalat jumat",
    "shalat berjamaah", "adzan", "membaca dzikir",
    "melakukan i'tikaf", "menyembelih qurban",
]

for actor in actors:
    for action in actions_ibadah:
        add("actor_action", f"hukum {actor} {action}")
        add("actor_action", f"bolehkah {actor} {action}")

# ═══════════════════════════════════════════════════════════════
# MEGA EXPANSION 2: Hukum X dalam keadaan Y
# ═══════════════════════════════════════════════════════════════

ibadah_acts = [
    "shalat", "puasa", "wudhu", "tayammum", "mandi junub",
    "i'tikaf", "membaca quran", "masuk masjid",
    "tawaf", "sa'i", "melempar jumrah", "ihram",
]

keadaan = [
    "dalam perjalanan", "saat sakit", "saat darurat",
    "di atas pesawat", "di dalam air", "saat hujan",
    "di tempat najis", "di tempat sempit", "saat perang",
    "di rumah sakit", "di penjara", "saat gempa bumi",
    "saat banjir", "saat pandemi", "di luar negeri",
    "di negara non muslim", "saat kerja",
    "di sekolah", "di kantor", "di mall",
]

for act in ibadah_acts:
    for state in keadaan:
        add("keadaan", f"hukum {act} {state}")

# ═══════════════════════════════════════════════════════════════
# MEGA EXPANSION 3: Detailed verse & chapter references
# ═══════════════════════════════════════════════════════════════

surahs = [
    "al baqarah", "ali imran", "an nisa", "al maidah",
    "al an'am", "al a'raf", "al anfal", "at taubah",
    "yunus", "hud", "yusuf", "ar ra'd", "ibrahim",
    "al hijr", "an nahl", "al isra", "al kahfi",
    "maryam", "thaha", "al anbiya", "al hajj",
    "al mu'minun", "an nur", "al furqan",
    "asy syu'ara", "an naml", "al qashash", "al ankabut",
    "ar rum", "luqman", "as sajdah", "al ahzab",
    "saba", "fathir", "yasin", "ash shaffat",
    "shad", "az zumar", "ghafir",
    "fushshilat", "asy syura", "az zukhruf", "ad dukhan",
    "al jatsiyah", "al ahqaf", "muhammad",
    "al fath", "al hujurat", "qaf", "adz dzariyat",
    "ath thur", "an najm", "al qamar", "ar rahman",
    "al waqiah", "al hadid", "al mujadilah",
    "al hasyr", "al mumtahanah", "ash shaff",
    "al jumuah", "al munafiqun", "at taghabun",
    "ath thalaq", "at tahrim", "al mulk",
    "al qalam", "al haqqah", "al ma'arij",
    "nuh", "al jinn", "al muzzammil", "al muddatstsir",
    "al qiyamah", "al insan", "al mursalat",
    "an naba", "an naziat", "abasa", "at takwir",
    "al infithar", "al muthaffifin", "al insyiqaq",
    "al buruj", "ath thariq", "al a'la",
    "al ghasyiyah", "al fajr", "al balad",
    "asy syams", "al lail", "adh dhuha",
    "al insyirah", "at tin", "al alaq",
    "al qadr", "al bayyinah", "al zalzalah",
    "al adiyat", "al qari'ah", "at takatsur",
    "al ashr", "al humazah", "al fil",
    "quraisy", "al ma'un", "al kautsar",
    "al kafirun", "an nashr", "al lahab",
    "al ikhlas", "al falaq", "an nas",
]

for surah in surahs:
    add("tafsir_surah", f"tafsir surat {surah}", "تفسير")

# ═══════════════════════════════════════════════════════════════
# MEGA EXPANSION 4: Specific kitab queries
# ═══════════════════════════════════════════════════════════════

kitab_names = [
    "ihya ulumuddin", "al umm", "bidayatul mujtahid",
    "fathul qarib", "fathul mu'in", "kifayatul akhyar",
    "tuhfatul muhtaj", "nihayatul muhtaj", "mughnil muhtaj",
    "al majmu syarh muhadzdzab", "raudhatut thalibin",
    "minhaj at thalibin", "al fiqh al islami",
    "riyadhus shalihin", "bulughul maram",
    "subulussalam", "nailul authar",
    "tafsir ibnu katsir", "tafsir jalalain",
    "tafsir qurthubi", "tafsir ath thabari",
    "shahih bukhari", "shahih muslim",
    "sunan abu dawud", "sunan tirmidzi",
    "sunan nasa'i", "sunan ibnu majah",
    "musnad ahmad", "muwattha malik",
]

for kitab in kitab_names:
    add("kitab", f"kitab {kitab} tentang apa", None)
    add("kitab", f"pengarang kitab {kitab}", None)
    add("kitab", f"isi kitab {kitab}", None)

# ═══════════════════════════════════════════════════════════════
# MEGA EXPANSION 5: Complex scenario queries
# ═══════════════════════════════════════════════════════════════

scenarios = [
    # Shalat scenarios
    "imam lupa rakaat bagaimana makmum", "makmum salah gerakan apakah batal",
    "imam batal wudhu saat shalat", "shalat berjamaah hanya dua orang",
    "shalat berdua suami istri", "imam anak kecil yang belum baligh",
    "makmum datang saat imam sujud", "makmum datang saat imam tasyahud akhir",
    "imam membaca surat terlalu panjang", "imam membaca surat terlalu pendek",
    "shalat berjamaah laki dan perempuan", "saf putus karena tiang",
    "shalat dalam kegelapan tanpa cahaya", "shalat dengan penerangan lilin",

    # Wudhu scenarios
    "wudhu tapi ada cat di tangan", "wudhu dengan air mineral kemasan",
    "wudhu tapi kuku panjang ada kutek", "wudhu dengan air bekas cuci",
    "wudhu tapi ada plester luka", "wudhu di pesawat di toilet kecil",
    "wudhu dengan air es beku yang dicairkan",
    "wudhu tapi ada ring cincin di jari",

    # Puasa scenarios
    "puasa tapi tidak bisa tidur semalaman", "puasa tapi mimpi basah",
    "puasa tapi lupa makan di siang hari", "puasa tapi terpaksa makan karena sakit",
    "puasa tapi pergi ke dokter gigi", "puasa tapi harus cabut gigi",
    "puasa tapi harus operasi", "puasa tapi memakai obat tetes mata",
    "puasa tapi masuk air saat berenang", "puasa tapi berciuman suami istri",
    "puasa tapi makan sebelum waktu maghrib", "puasa tapi tidak niat dari malam",

    # Nikah scenarios
    "nikah dengan mengandung anak orang lain", "nikah wanita hamil zina",
    "menikahi mantannya teman", "menikahi janda yang masih iddah",
    "menikahi wanita yang di-talak tiga orang lain",
    "nikah setelah zina dengan perempuan yang sama",
    "wali nikah sudah meninggal siapa penggantinya",
    "wali nikah kakek bukan ayah", "wali nikah paman",
    "ijab qabul tidak dalam satu majelis", "mas kawin hutang belum dibayar",

    # Muamalat scenarios
    "beli barang lalu harga turun minta balikin selisih",
    "jual barang tapi ternyata cacat", "beli rumah ternyata berhantu",
    "pinjam uang ke teman lalu hilang kontak",
    "titip uang ke orang lalu orangnya meninggal",
    "arisan bubar sebelum semua dapat giliran",
    "kontrak kerja tidak sesuai", "gaji dipotong tidak sesuai perjanjian",
    "usaha patungan salah satu pihak rugi",
    "sewa rumah tapi rumahnya rusak", "gadai emas tapi emasnya hilang",

    # Waris scenarios
    "suami meninggal punya istri 2 dan 3 anak", "istri meninggal suami dan orang tua",
    "pewaris hanya meninggalkan hutang", "pewaris punya anak angkat",
    "pewaris beda agama dengan ahli waris", "pewaris membunuh",
    "wasiat melebihi sepertiga harta", "hibah sebelum meninggal",

    # Modern scenarios
    "zakat cryptocurrency yang nilainya turun naik",
    "akad nikah via zoom saat pandemi",
    "shalat jumat di rumah saat lockdown",
    "wudhu dengan hand sanitizer", "tayammum di gedung bertingkat",
    "shalat di ruangan virtual reality", "zakat NFT digital art",
    "puasa di negara yang mataharinya tidak tenggelam",
    "puasa di kutub utara", "kiblat dari luar angkasa",
    "shalat di stasiun luar angkasa", "wudhu tanpa gravitasi",
    "menikah dengan AI atau robot hukumnya",
    "jual beli di metaverse", "akad di dunia virtual",
]

for scenario in scenarios:
    add("skenario", scenario)

# ═══════════════════════════════════════════════════════════════
# MEGA EXPANSION 6: Fiqh terms as standalone queries
# ═══════════════════════════════════════════════════════════════

fiqh_terms = [
    "istinja", "istibra", "istinjak", "istihalah", "istihsan",
    "istishab", "istisqa", "istikharah", "i'tikaf", "iftitah",
    "ibtida", "ijab", "qabul", "ijma", "ijtihad", "ikhtilaf",
    "ikhtilat", "ila'", "ilhad", "iman", "imsak",
    "inhiraf", "iqamah", "iqrar", "irsal", "irtidad",
    "isbal", "isnad", "israf", "istiazah", "istigfar",
    "jama' taqdim", "jama' takhir", "janabah", "janazah",
    "jarh", "jihad", "jizyah", "junub",
    "kabirah", "kafan", "kafalah", "kafarat", "kafa'ah",
    "kalam", "khalwat", "khamr", "kharaj", "khauf",
    "khiyar", "khul'", "khusyu'", "kiblat",
    "li'an", "luqathah",
    "madzmumah", "mafsadah", "mahr", "mahram", "makruh",
    "mandub", "mani'", "mansukh", "maqashid", "maqam",
    "masbuq", "maslahah", "ma'mum", "ma'ruf",
    "miqat", "mu'allaf", "mualaf", "muamalah", "mubah",
    "mudharabah", "muhrim", "muhshan", "mujmal",
    "mukallaf", "mumin", "munafiq", "munfarid",
    "muqayyad", "mursal", "murtad", "musafir",
    "mushaf", "musnad", "mustahab", "mustahik",
    "mustahadhah", "mutawatir", "mutlaq", "muzakki",
    "nadb", "nafaqah", "nafilah", "nafs", "najis",
    "nasab", "nasikh", "nawafil", "nazar", "nifaq",
    "nifas", "nikah", "niqab", "niyyah", "nusyuz",
    "qada'", "qadar", "qadhi", "qaidah", "qana'ah",
    "qardh", "qarin", "qasam", "qasar", "qatl",
    "qiblah", "qira'ah", "qisas", "qiyas", "qunut",
    "rabb", "radha'ah", "radd", "raja'", "rajam",
    "rakaat", "ramadhan", "rawi", "riba", "riddah",
    "riya'", "rukhsah", "ruku'", "ruqyah",
    "sabab", "sabr", "sadaqah", "saf", "sahih",
    "sahur", "sajdah", "salam", "salat", "sanad",
    "shaghirah", "shahid", "shari'ah", "shiyam",
    "shuf'ah", "shulh", "shurah", "sujud", "sunnah",
    "sutr", "syafa'ah", "syahadah", "syar'i", "syirik",
    "ta'awun", "ta'dib", "ta'dil", "ta'liq",
    "ta'wil", "ta'zir", "tabarruk", "tabdzir", "tabi'in",
    "tafsir", "tahajjud", "tahallul", "taharah",
    "tahkim", "tahnik", "tajwid", "takbir", "takdir",
    "takfir", "taklid", "taklif", "talak", "talaq",
    "tamlik", "taqdir", "taqlid", "taqwa", "tarawi",
    "targib", "tarhib", "tasawuf", "tashahhud",
    "tasyahud", "tasyri'", "tatawwu'", "tathahur",
    "taubah", "tawadu'", "tawaf", "tawakal",
    "tawassul", "tayammum", "thahir", "thayyib",
    "ulama", "ummah", "umrah", "urf", "ushul", "udhiyah",
    "wadi'ah", "wahyu", "wakalah", "walimah", "wali",
    "waqf", "wara'", "wasiat", "wudu", "wukuf",
    "yamin", "yatim",
    "zakah", "zhalim", "zhahar", "zihar",
    "zikir", "zina", "zuhud",
]

for term in fiqh_terms:
    add("term", f"pengertian {term}")
    add("term", f"hukum {term}")

# ═══════════════════════════════════════════════════════════════
# MEGA EXPANSION 7: Q&A format queries (natural questions)
# ═══════════════════════════════════════════════════════════════

qa_queries = [
    "apakah kucing suci dalam islam",
    "berapa kali sujud dalam shalat subuh",
    "siapa yang berhak menjadi imam shalat",
    "kapan waktu shalat dhuha berakhir",
    "dimana tempat shalat yang paling utama",
    "mengapa riba diharamkan dalam islam",
    "bagaimana cara taubat dari dosa besar",
    "apakah dosa bisa diampuni semua",
    "siapa saja yang termasuk mahram",
    "berapa lama masa iddah cerai mati",
    "kapan zakat fitrah harus dibayarkan",
    "dimana miqat haji untuk jemaah Indonesia",
    "mengapa wanita tidak boleh jadi imam shalat",
    "bagaimana cara membagi warisan yang adil",
    "apakah emas perhiasan wajib dizakati",
    "siapa yang berhak menerima zakat",
    "berapa nisab zakat emas",
    "kapan waktu terbaik untuk berdoa",
    "dimana tempat terbaik untuk shalat tahajjud",
    "mengapa puasa ramadhan diwajibkan",
    "bagaimana jika lupa niat puasa",
    "apakah boleh puasa tanpa sahur",
    "siapa yang wajib berpuasa",
    "berapa hari puasa ramadhan",
    "kapan malam lailatul qadr",
    "mengapa haji wajib sekali seumur hidup",
    "bagaimana tata cara ihram",
    "apakah anak kecil boleh umrah",
    "siapa yang wajib haji",
    "berapa biaya dam haji",
    "kapan waktu melempar jumrah",
    "bagaimana cara wukuf di arafah",
    "apakah wanita boleh haji sendiri",
    "mengapa tawaf harus berlawanan arah jarum jam",
    "dimana tempat mabit di muzdalifah",
    "apakah nikah mutah halal",
    "bagaimana prosedur khuluk",
    "siapa yang berhak menjadi wali nikah",
    "berapa minimal mahar dalam islam",
    "apakah talak bisa dicabut",
    "mengapa poligami diizinkan",
    "kapan iddah dimulai",
    "dimana sebaiknya akad nikah dilakukan",
    "bagaimana cara rujuk yang sah",
    "apakah wanita boleh meminta cerai",
    "siapa yang menanggung nafkah anak setelah cerai",
    "berapa lama masa iddah talak raj'i",
    "bagaimana hukum nikah hamil",
    "mengapa saksi nikah harus laki-laki",
]
for q in qa_queries:
    add("qa_natural", q)

# ═══════════════════════════════════════════════════════════════
# MEGA EXPANSION 8: Real pesantren-style discussions
# ═══════════════════════════════════════════════════════════════

pesantren_queries = [
    "ta'bir masalah orang shalat pakai masker",
    "ibarat kitab tentang shalat di kendaraan",
    "pendapat ulama tentang qunut subuh",
    "dalil nash tentang wajibnya shalat jumat",
    "qoul mu'tamad tentang menyentuh wanita",
    "pendapat muktamad mazhab syafii tentang niat",
    "ibarah fathul qarib bab shalat",
    "syarah fathul mu'in bab thaharah",
    "hasyiah ianah ath thalibin bab nikah",
    "kitab al umm imam syafii bab jual beli",
    "majmu syarh muhadzdzab nawawi tentang puasa",
    "ihya ulumuddin ghazali bab taubat",
    "minhajut thalibin bab waris",
    "tuhfatul muhtaj bab haji",
    "nihayatul muhtaj bab gadai",
    "hasyiah bajuri bab zakat",
    "kifayatul akhyar bab thaharah",
    "fathul wahhab bab nikah",
    "mughnil muhtaj bab jual beli",
    "raudhatut thalibin bab qadha",
    "hasyiah qalyubi wa umairah tentang shalat",
    "tahrir tanqih al lubab tentang puasa",
    "sullam taufiq tentang iman",
    "safinah an najah tentang wudhu",
    "matn taqrib tentang shalat",
    "syarh sittin masalah tentang wudhu",
    "risalah jamiah tentang ushuluddin",
    "fathul majid syarh kitab tauhid",
    "aqidah awam nadhom",
    "tijan ad darari tentang tauhid",
]
for q in pesantren_queries:
    add("pesantren", q)

# ═══════════════════════════════════════════════════════════════
# MEGA EXPANSION 9: Arabic term variations with diacritics
# ═══════════════════════════════════════════════════════════════

arabic_term_queries = [
    "صلاة الجمعة", "صلاة الجنازة", "صلاة التهجد",
    "صلاة الضحى", "صلاة التراويح", "صلاة الوتر",
    "صلاة العيدين", "صلاة الكسوف", "صلاة الخسوف",
    "صلاة الاستخارة", "صلاة الاستسقاء",
    "الوضوء وأحكامه", "التيمم وشروطه",
    "الغسل الواجب", "إزالة النجاسة",
    "الحيض والنفاس", "الاستحاضة وأحكامها",
    "صيام رمضان", "صيام التطوع", "صيام النذر",
    "مفطرات الصيام", "الإفطار في رمضان",
    "زكاة الذهب والفضة", "زكاة التجارة",
    "زكاة الزروع والثمار", "زكاة الأنعام",
    "مصارف الزكاة", "نصاب الزكاة",
    "مناسك الحج", "العمرة وأحكامها",
    "الإحرام وشروطه", "الطواف وأنواعه",
    "السعي بين الصفا والمروة", "الوقوف بعرفة",
    "المبيت بمزدلفة", "رمي الجمرات",
    "فدية الحج", "هدي التمتع",
    "عقد النكاح", "شروط النكاح وأركانه",
    "المهر وأحكامه", "الولي في النكاح",
    "الشهادة في النكاح", "الطلاق الرجعي",
    "الطلاق البائن", "الخلع وأحكامه",
    "العدة وأنواعها", "الرجعة وشروطها",
    "النفقة الزوجية", "حضانة الأطفال",
    "الميراث وقسمته", "أصحاب الفروض",
    "العصبات في الميراث", "الحجب في الإرث",
    "الوصية وأحكامها", "الوقف وشروطه",
    "البيع وأحكامه", "عقد السلم",
    "عقد الاستصناع", "عقد الإجارة",
    "عقد المضاربة", "عقد المشاركة",
    "الربا وأنواعه", "الغرر في المعاملات",
    "خيار المجلس", "خيار الشرط",
    "القصاص في الإسلام", "الدية وأحكامها",
    "حد الزنا", "حد السرقة", "حد القذف",
    "حد الحرابة", "التعزير وأنواعه",
    "الجهاد وأحكامه", "الأمر بالمعروف والنهي عن المنكر",
]
for q in arabic_term_queries:
    add("arabic_ext", q, None, "ar")

# ═══════════════════════════════════════════════════════════════
# MEGA EXPANSION 10: English scholarly queries
# ═══════════════════════════════════════════════════════════════

en_scholarly = [
    "Islamic jurisprudence on organ donation",
    "Shafi'i school ruling on witr prayer",
    "Hanafi opinion on touching women invalidating wudu",
    "Maliki school position on eating frog",
    "Hanbali view on music and singing",
    "difference between qiyas and istihsan",
    "maqasid al shariah five necessities",
    "legal maxim al yaqin la yazul",
    "Islamic law of contracts",
    "prohibition of gharar in transactions",
    "rules of bay al salam forward sale",
    "Islamic finance murabaha explained",
    "sukuk Islamic bonds ruling",
    "takaful Islamic insurance",
    "waqf endowment conditions",
    "rules of inheritance in Islam detailed",
    "concept of asbab al nuzul",
    "abrogation nasikh mansukh Quran",
    "classification of hadith sciences",
    "jarh wa tadil narrator criticism",
    "conditions for valid ijtihad",
    "taqlid following a madhab", "imam ghazali ihya ulum al din",
    "ibn taymiyyah fatawa on tawassul",
    "imam nawawi riyad al salihin", "imam shafii al umm contents",
    "bulugh al maram ibn hajar",
    "bidayat al mujtahid ibn rushd",
    "sharh sahih muslim by nawawi",
    "fath al bari ibn hajar explanation",
    "defense of the sunnah", "criticism of weak hadith",
    "Islamic penal code hudud implementation",
    "blood money diyah calculation",
    "rules of jihad and its types",
    "Islamic law of war and peace",
    "non Muslim rights in Islamic state",
    "jizya tax on non Muslims",
    "rules of dhimmi in Islamic law",
    "apostasy ruling scholarly debate",
    "blasphemy in Islamic jurisprudence",
    "stoning punishment evidence debate",
]
for q in en_scholarly:
    add("english_scholarly", q, None, "en")

# ═══════════════════════════════════════════════════════════════
# MEGA EXPANSION 11: Topical deep-dives
# ═══════════════════════════════════════════════════════════════

# Prayer (shalat) deep-dive: every possible sub-question
shalat_deep = [
    "adzan hukumnya wajib atau sunnah", "iqamah hukumnya",
    "lafaz adzan lengkap", "lafaz iqamah lengkap",
    "tarji' dalam adzan", "tatswib dalam adzan subuh",
    "muadzin syarat", "jawab adzan hukumnya",
    "doa setelah adzan", "doa antara adzan dan iqamah",
    "shalat wajib lima waktu dalilnya",
    "waktu shalat subuh awal dan akhir", "waktu shalat dzuhur",
    "waktu shalat ashar awal dan akhir", "waktu shalat maghrib",
    "waktu shalat isya awal dan akhir",
    "waktu makruh shalat", "waktu terlarang shalat",
    "shalat setelah subuh sebelum matahari terbit",
    "shalat setelah ashar sebelum maghrib",
    "shalat saat matahari tepat di atas kepala",
    "shalat saat matahari terbenam", "shalat saat matahari terbit",
    "takbiratul ihram wajib atau rukun", "niat shalat tempatnya dimana",
    "berdiri dalam shalat wajib", "membaca fatihah wajib bagi imam dan makmum",
    "fatihah di rakaat ketiga dan keempat", "membaca surat setelah fatihah",
    "surat yang dibaca di shalat subuh", "surat pendek untuk shalat",
    "ruku' tuma'ninah", "i'tidal tuma'ninah",
    "sujud tuma'ninah berapa lama", "duduk di antara dua sujud",
    "tasyahud awal duduk iftirasy", "tasyahud akhir duduk tawarruk",
    "bacaan tasyahud akhir lengkap", "shalawat dalam tasyahud",
    "doa sebelum salam", "salam ke kanan dan kiri",
    "tertib dalam rukun shalat", "tumakninah pengertiannya",
    "sunnah ab'adh shalat", "sunnah hai'at shalat",
    "doa iftitah macam-macamnya", "ta'awwudz sebelum fatihah",
    "amin setelah fatihah", "bacaan ruku lengkap",
    "bacaan sujud lengkap", "bacaan i'tidal lengkap",
    "bacaan duduk di antara dua sujud",
    "qunut nazilah", "qunut witir bacaannya",
    "shalat nafilah duduk", "shalat sunnah berapa rakaat",
    "shalat rawatib qabliyah ba'diyah",
    "shalat sunah sebelum dzuhur berapa rakaat",
    "shalat sunnah setelah dzuhur berapa rakaat",
    "shalat sunnah sebelum ashar", "shalat sunnah setelah maghrib",
    "shalat sunnah sebelum isya", "shalat sunnah setelah isya",
]
for q in shalat_deep:
    add("shalat_deep", q, "عبادات")

# Nikah deep-dive
nikah_deep = [
    "meminang wanita yang sudah dipinang orang lain",
    "meminang wanita dalam masa iddah", "melihat wanita sebelum menikah",
    "batas melihat calon istri", "ta'aruf tanpa pacaran",
    "syarat calon suami dalam islam", "syarat calon istri",
    "istri shalihah ciri-cirinya", "suami shalih ciri-cirinya",
    "mahar yang paling utama", "mahar termahal dalam sejarah islam",
    "mahar paling sederhana", "mahar berupa hafalan quran",
    "mahar berupa mengajarkan quran", "mahar berupa cincin besi",
    "walimah undangan wajib datang", "menghadiri walimah hukumnya",
    "menolak undangan walimah", "walimah sederhana sunnah",
    "hak dan kewajiban suami istri", "nafkah wajib minimal",
    "nafkah anak setelah perceraian siapa yang menanggung",
    "musyawarah suami istri", "istri bekerja di luar rumah",
    "suami menyuruh istri bekerja", "istri menolak hubungan suami istri",
    "nusyuz suami", "nusyuz istri hukumnya dan penanganan",
    "syiqaq perselisihan suami istri", "hakam dalam syiqaq",
    "poligami izin istri pertama", "poligami tanpa izin istri",
    "giliran poligami", "adil dalam poligami pengertiannya",
]
for q in nikah_deep:
    add("nikah_deep", q, "مناكحات")

# ═══════════════════════════════════════════════════════════════
# MEGA EXPANSION 12: Awam-style colloquial questions (Indonesian)
# ═══════════════════════════════════════════════════════════════

awam_queries = [
    "kalo kentut udah wudhu ulang gak", "shalat tapi gak hafal surat",
    "puasa tapi minum ga sengaja", "zakat berapa duitnya",
    "haji mahal banget gimana kalo gak mampu",
    "nikah siri itu sah gak sih menurut agama",
    "cerai tiga kali bisa balik gak", "riba itu apa sih sebenarnya",
    "kerja di bank dosa gak", "cicilan motor itu riba bukan",
    "kalo gak shalat dosa besar ya", "mandi junub wajib gak setelah mimpi",
    "kenapa sih cewek gak boleh jadi imam", "cadar wajib gak",
    "jenggot harus dipanjangkan ya", "musik haram kata siapa",
    "valentine haram serius", "natal boleh ucapin gak",
    "hallowen halloween haram ya", "ulang tahun boleh gak",
    "pelihara anjing kenapa haram", "kucing sunnah ya",
    "jin itu ada beneran", "santet itu nyata menurut islam",
    "ruqyah kok kayak kesurupan", "dukun haram ya",
    "horoskop zodiak haram ya", "feng shui haram menurut islam",
    "yoga haram menurut MUI", "meditasi haram gak",
    "hipnotis haram gak", "NLP haram gak",
    "MLM haram semua atau sebagian", "judi online haram ya",
    "taruhan bola dosa gak", "game gacha termasuk judi",
    "loot box game termasuk gambling gak",
    "nonton drama korea dosa gak", "baca komik manga dosa",
    "cosplay pake jilbab", "gambar anime haram menurut siapa",
    "bikin webtoon isinya islami boleh gak",
    "jadi youtuber boleh gak", "tiktok haram karena joget",
    "main PUBG haram ya katanya", "main ML boleh gak",
    "streaming haram gak", "chatting lawan jenis dosa",
    "LDR pacaran jarak jauh haram", "kencan online tinder haram ya",
]
for q in awam_queries:
    add("awam", q)

# ═══════════════════════════════════════════════════════════════
# MEGA EXPANSION 13: More systematic "tentang" queries
# ═══════════════════════════════════════════════════════════════

tentang_topics = [
    "tentang shalat fardu", "tentang shalat sunnah",
    "tentang puasa wajib", "tentang puasa sunnah",
    "tentang zakat harta", "tentang zakat fitrah",
    "tentang haji wajib", "tentang umrah sunnah",
    "tentang nikah dan pernikahan", "tentang talak dan perceraian",
    "tentang jual beli halal", "tentang riba dan bunga",
    "tentang warisan dan faraid", "tentang wasiat",
    "tentang wakaf", "tentang hibah", "tentang gadai",
    "tentang sewa menyewa", "tentang bagi hasil",
    "tentang pinjaman", "tentang hutang piutang",
    "tentang qurban", "tentang aqiqah",
    "tentang jenazah", "tentang pemakaman",
    "tentang sumpah", "tentang nadzar",
    "tentang kafarat", "tentang fidyah",
    "tentang jihad", "tentang da'wah",
    "tentang amar ma'ruf nahi munkar",
    "tentang hubungan muslim non muslim",
    "tentang toleransi beragama",
    "tentang tasamuh dalam islam",
]
for topic in tentang_topics:
    add("tentang", f"hukum islam {topic}")

# ═══════════════════════════════════════════════════════════════
# MEGA EXPANSION 14: Verb-based queries
# ═══════════════════════════════════════════════════════════════

verbs_topics = [
    ("membunuh", ["semut", "nyamuk", "ular", "kalajengking", "tikus", "cicak", "kecoa"]),
    ("memakan", ["daging mentah", "darah", "bangkai", "daging babi", "daging unta"]),
    ("menjual", ["barang haram", "minuman keras", "rokok", "daging babi", "obat terlarang"]),
    ("meminjam", ["uang berbunga", "dari rentenir", "dari bank konvensional"]),
    ("memelihara", ["anjing", "kucing", "burung dalam sangkar", "ular", "ikan"]),
    ("menonton", ["film kekerasan", "film dewasa", "konser musik", "standup comedy"]),
    ("bermain", ["judi", "catur", "kartu", "game online", "lotere"]),
    ("berdagang", ["saham", "forex", "kripto", "emas", "barang antik"]),
]

for verb, objects in verbs_topics:
    for obj in objects:
        add("verb_obj", f"hukum {verb} {obj}")
        add("verb_obj", f"bolehkah {verb} {obj}")

# ═══════════════════════════════════════════════════════════════
# MEGA EXPANSION 15: Negation/prohibition queries
# ═══════════════════════════════════════════════════════════════

prohibitions = [
    "larangan dalam shalat",
    "larangan dalam puasa",
    "larangan saat ihram",
    "larangan bagi orang junub",
    "larangan bagi wanita haid",
    "larangan dalam jual beli",
    "larangan dalam nikah",
    "larangan memakan binatang",
    "larangan bagi musafir",
    "larangan di bulan ramadhan",
    "larangan di hari jumat",
    "larangan di malam hari",
    "larangan bagi imam shalat",
    "larangan bagi pengantin",
    "larangan saat adzan",
    "yang tidak boleh dilakukan saat wudhu",
    "yang tidak boleh dilakukan saat shalat",
    "yang tidak boleh dilakukan saat puasa",
    "yang tidak boleh dilakukan saat haji",
    "yang haram dilakukan suami terhadap istri",
    "yang haram dilakukan istri terhadap suami",
]
for q in prohibitions:
    add("larangan", q)

# ═══════════════════════════════════════════════════════════════
# MEGA EXPANSION 16: Number-parametric queries
# ═══════════════════════════════════════════════════════════════

for n in range(1, 11):
    add("numbered", f"{n} syarat wudhu")
    add("numbered", f"{n} rukun shalat")
    add("numbered", f"{n} pembatal puasa")
    add("numbered", f"{n} golongan penerima zakat")
    add("numbered", f"{n} rukun islam")
    add("numbered", f"{n} rukun iman")

# ═══════════════════════════════════════════════════════════════
# MEGA EXPANSION 17: Comparative religion perspective
# ═══════════════════════════════════════════════════════════════

comparative_queries = [
    "perbedaan islam dan kristen tentang isa",
    "pandangan islam tentang trinitas",
    "sikap islam terhadap agama lain",
    "ahl al kitab siapa mereka", "menikahi wanita ahli kitab",
    "makanan ahli kitab halal", "sembelihan ahli kitab",
    "jizyah terhadap non muslim", "hubungan muslim non muslim",
    "dialog antar agama dalam islam", "toleransi dalam islam",
    "tidak ada paksaan dalam agama", "la ikraha fi ad din",
    "kebebasan beragama menurut islam", "pluralisme agama perspektif islam",
]
for q in comparative_queries:
    add("comparative", q)

# ═══════════════════════════════════════════════════════════════
# MEGA EXPANSION 18: Women-specific fiqh issues
# ═══════════════════════════════════════════════════════════════

women_fiqh = [
    "shalat wanita haid", "wanita haid masuk masjid",
    "wanita haid membaca quran", "wanita haid puasa",
    "wanita haid tawaf", "wanita haid berhubungan suami",
    "iddah wanita cerai", "iddah wanita cerai mati",
    "iddah wanita hamil", "iddah wanita yang tidak haid",
    "wanita karir dalam islam", "wanita bekerja di luar rumah",
    "wanita menjadi pemimpin", "wanita menjadi hakim",
    "wanita menjadi saksi", "wanita menjadi wali nikah",
    "aurat wanita di depan wanita", "aurat wanita di depan mahram",
    "aurat wanita di depan non mahram", "aurat wanita dalam shalat",
    "jilbab syar'i bentuknya", "warna jilbab yang disunnahkan",
    "cadar wajib atau tidak", "wanita berkendara sendiri",
    "wanita bepergian tanpa mahram", "wanita shalat di masjid",
    "wanita shalat berjamaah di rumah", "wanita shalat jumat",
    "wanita adzan", "wanita menjadi imam bagi wanita",
    "perawatan wajah wanita dalam islam", "make up halal",
    "operasi kecantikan wanita", "suntik botox wanita",
]
for q in women_fiqh:
    add("women_fiqh", q)

# ═══════════════════════════════════════════════════════════════
# MEGA EXPANSION 19: Medical & health-related fiqh
# ═══════════════════════════════════════════════════════════════

medical_fiqh = [
    "berobat dengan obat haram", "berobat ke dukun",
    "pengobatan alternatif hukumnya", "bekam hijamah hukumnya",
    "akupunktur hukumnya", "hipnoterapi hukumnya",
    "obat mengandung alkohol", "obat dari bahan haram",
    "transfusi darah hukumnya", "donor organ setelah mati",
    "donor organ saat hidup", "jual beli organ tubuh",
    "euthanasia dalam islam", "mencabut alat bantu napas",
    "autopsi jenazah hukumnya", "bedah mayat untuk ilmu",
    "bayi tabung in vitro", "inseminasi buatan suami",
    "inseminasi buatan donor", "bank sperma hukumnya",
    "bank sel telur hukumnya", "surrogate mother ibu pengganti",
    "tes DNA untuk nasab", "tes kehamilan sebelum nikah",
    "sterilisasi vasektomi tubektomi", "KB spiral IUD",
    "KB pil dan suntik", "kondom hukumnya",
    "sunat perempuan hukumnya", "sunat laki-laki wajib",
    "menyusui anak orang lain", "ASI donor hukumnya",
    "imunisasi anak wajib", "obat bius hukumnya",
    "ganja untuk pengobatan", "alkohol untuk sterilisasi",
]
for q in medical_fiqh:
    add("medical", q)

# ═══════════════════════════════════════════════════════════════
# MEGA EXPANSION 20: Environmental & animal welfare fiqh
# ═══════════════════════════════════════════════════════════════

env_queries = [
    "menjaga lingkungan dalam islam", "hukum membuang sampah sembarangan",
    "hukum merusak alam", "menebang pohon hukumnya",
    "membakar hutan hukumnya", "polusi udara tanggung jawab",
    "hewan langka dilindungi hukumnya", "berburu hewan hukumnya",
    "menyiksa hewan dosa", "menyembelih hewan cara islami",
    "stunning sebelum sembelih", "kurban hewan yang cacat",
    "hewan kurban syarat", "aqiqah kambing jantan betina",
    "memandikan hewan peliharaan", "kucing masuk masjid",
    "anjing penjaga rumah", "meracuni hewan liar",
    "peternakan massal hukumnya", "ternak ayam broiler",
    "telur ayam pejantan", "madu lebah halal cara ambilnya",
]
for q in env_queries:
    add("environment", q)

# ═══════════════════════════════════════════════════════════════
# ASSEMBLE FINAL
# ═══════════════════════════════════════════════════════════════

# Reassign sequential IDs
for i, q in enumerate(queries):
    q["id"] = f"q{i+1:05d}"

print(f"Total unique queries: {len(queries)}")
cats = {}
for q in queries:
    cats[q["category"]] = cats.get(q["category"], 0) + 1
for cat, count in sorted(cats.items()):
    print(f"  {cat}: {count}")

# Save final
with open("queries_10k.json", "w", encoding="utf-8") as f:
    json.dump(queries, f, ensure_ascii=False, indent=1)

eval_queries = [{"id": q["id"], "text": q["text"]} for q in queries]
with open("queries_10k_eval.json", "w", encoding="utf-8") as f:
    json.dump(eval_queries, f, ensure_ascii=False, indent=1)

print(f"\nSaved {len(queries)} queries to queries_10k.json and queries_10k_eval.json")
