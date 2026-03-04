<script>
  let openFaq = $state(-1);

  function toggleFaq(i) {
    openFaq = openFaq === i ? -1 : i;
  }

  const faqs = [
    {
      q: "Bagaimana cara menggunakan Bahtsul Masail?",
      a: "Cukup ketik pertanyaan Anda di kotak pencarian menggunakan Bahasa Indonesia, English, atau العربية. Sistem akan secara otomatis menerjemahkan ke istilah Arab yang tepat dan mencari di 7.800+ kitab klasik. Anda juga bisa menggunakan shortcut Ctrl+K untuk langsung fokus ke kotak pencarian."
    },
    {
      q: "Bahasa apa saja yang didukung?",
      a: "Bahtsul Masail mendukung pencarian dalam tiga bahasa: Bahasa Indonesia, English, dan العربية. Anda tidak perlu menulis dalam bahasa Arab — sistem akan secara otomatis menerjemahkan terminologi fikih ke bahasa kitab asli."
    },
    {
      q: "Apa itu jawaban AI dan seberapa akurat?",
      a: "Jawaban AI adalah sintesis dari hasil pencarian kitab yang ditemukan. Setiap jawaban SELALU merujuk ke sumber kitab yang jelas (nama kitab dan halaman). Kami tidak pernah memberikan jawaban tanpa sumber — ini prinsip adab keilmuan kami. Namun, jawaban AI tetaplah alat bantu, bukan pengganti fatwa ulama."
    },
    {
      q: "Bagaimana cara mengekspor hasil pencarian?",
      a: "Setelah mendapatkan hasil pencarian, klik tombol export di bagian atas hasil. Anda dapat mengekspor ke format Markdown (.md), Word (.docx), PDF, atau Plain Text (.txt)."
    },
    {
      q: "Apa itu API Key dan bagaimana mendapatkannya?",
      a: "API Key memungkinkan Anda mengakses Bahtsul Masail secara programatik — misalnya dari AI agent, script, atau aplikasi lain. Buat API key di halaman API Documentation, lalu gunakan header X-API-Key di setiap request."
    },
    {
      q: "Kitab apa saja yang tersedia?",
      a: "Bahtsul Masail mengindeks 7.872 kitab Islam klasik dari berbagai bidang ilmu: fikih (hukum Islam), tafsir (penjelasan Al-Quran), hadits, aqidah, tasawuf, akhlak, bahasa Arab, dan lainnya. Meliputi karya ulama dari berbagai mazhab dan era."
    },
    {
      q: "Apakah gratis?",
      a: "Ya, Bahtsul Masail gratis untuk digunakan. API access juga tersedia secara gratis dengan rate limit yang wajar (30 request/menit)."
    },
    {
      q: "Bagaimana jika pencarian tidak menemukan hasil?",
      a: "Coba variasikan kata kunci atau gunakan istilah yang lebih umum. Misalnya, daripada 'boleh gak pakai masker saat shalat', coba 'hukum menutup wajah dalam shalat'. Anda juga bisa menggunakan Quick Action di halaman utama untuk inspirasi pertanyaan."
    }
  ];
</script>

<svelte:head>
  <title>Bantuan — Bahtsul Masail</title>
</svelte:head>

<div class="help-page">
  <div class="help-container">
    <header class="help-header">
      <h1>❓ Bantuan</h1>
      <p>Pertanyaan yang sering diajukan dan panduan penggunaan</p>
    </header>

    <!-- Quick Start -->
    <section class="help-section">
      <h2>🚀 Panduan Cepat</h2>
      <div class="steps-grid">
        <div class="step-card">
          <div class="step-num">1</div>
          <h3>Ketik Pertanyaan</h3>
          <p>Tulis pertanyaan fikih dalam bahasa Anda — Indonesia, English, atau Arab.</p>
        </div>
        <div class="step-card">
          <div class="step-num">2</div>
          <h3>Baca Jawaban AI</h3>
          <p>AI akan mensintesis jawaban dari kitab-kitab yang ditemukan, lengkap dengan sumber.</p>
        </div>
        <div class="step-card">
          <div class="step-num">3</div>
          <h3>Telusuri Sumber</h3>
          <p>Klik referensi di jawaban AI untuk langsung ke teks asli kitab.</p>
        </div>
        <div class="step-card">
          <div class="step-num">4</div>
          <h3>Ekspor Hasil</h3>
          <p>Simpan hasil riset dalam format Markdown, Word, PDF, atau teks.</p>
        </div>
      </div>
    </section>

    <!-- Keyboard Shortcuts -->
    <section class="help-section">
      <h2>⌨️ Shortcut Keyboard</h2>
      <div class="shortcuts-list">
        <div class="shortcut-item"><kbd>Ctrl</kbd>+<kbd>K</kbd><span>Fokus pencarian</span></div>
        <div class="shortcut-item"><kbd>Ctrl</kbd>+<kbd>N</kbd><span>Percakapan baru</span></div>
        <div class="shortcut-item"><kbd>Ctrl</kbd>+<kbd>.</kbd><span>Buka riwayat</span></div>
        <div class="shortcut-item"><kbd>Ctrl</kbd>+<kbd>/</kbd><span>Tampilkan shortcuts</span></div>
        <div class="shortcut-item"><kbd>Esc</kbd><span>Tutup panel/modal</span></div>
      </div>
    </section>

    <!-- FAQ -->
    <section class="help-section">
      <h2>📋 FAQ</h2>
      <div class="faq-list">
        {#each faqs as faq, i}
          <div class="faq-item" class:open={openFaq === i}>
            <button class="faq-question" onclick={() => toggleFaq(i)}>
              <span>{faq.q}</span>
              <span class="faq-arrow">{openFaq === i ? '▲' : '▼'}</span>
            </button>
            {#if openFaq === i}
              <div class="faq-answer">
                <p>{faq.a}</p>
              </div>
            {/if}
          </div>
        {/each}
      </div>
    </section>

    <!-- Contact -->
    <section class="help-section">
      <h2>📬 Hubungi Kami</h2>
      <p>Tidak menemukan jawaban? Hubungi kami:</p>
      <ul>
        <li>Email: <strong>admin@bahtsulmasail.tech</strong></li>
        <li>Website: <a href="https://bahtsulmasail.tech">bahtsulmasail.tech</a></li>
      </ul>
    </section>

    <footer class="help-footer">
      <a href="/">← Kembali ke Pencarian</a>
    </footer>
  </div>
</div>

<style>
  .help-page {
    max-width: 780px;
    margin: 0 auto;
    padding: 32px 24px;
    font-family: var(--font-ui);
  }

  .help-header {
    margin-bottom: 32px;
  }

  .help-header h1 {
    color: var(--color-primary);
    font-size: 1.8rem;
    margin-bottom: 4px;
  }

  .help-header p {
    color: var(--color-text-muted);
  }

  .help-section {
    margin-bottom: 36px;
    padding-bottom: 28px;
    border-bottom: 1px solid var(--color-border);
  }

  .help-section h2 {
    font-size: 1.25rem;
    color: var(--color-primary);
    margin-bottom: 16px;
  }

  .steps-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
    gap: 16px;
  }

  .step-card {
    padding: 16px;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    text-align: center;
  }

  .step-num {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: var(--color-primary);
    color: #fff;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-weight: 700;
    font-size: 1.1rem;
    margin-bottom: 8px;
  }

  .step-card h3 {
    font-size: 0.95rem;
    margin-bottom: 4px;
  }

  .step-card p {
    font-size: 0.82rem;
    color: var(--color-text-muted);
    line-height: 1.5;
  }

  .shortcuts-list {
    display: grid;
    gap: 8px;
  }

  .shortcut-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
    background: var(--color-surface);
    border-radius: var(--radius-sm);
    font-size: 0.88rem;
  }

  .shortcut-item span {
    margin-left: auto;
    color: var(--color-text-muted);
  }

  kbd {
    background: var(--color-bg-alt);
    border: 1px solid var(--color-border);
    border-radius: 4px;
    padding: 2px 8px;
    font-size: 0.82rem;
    font-family: monospace;
    box-shadow: 0 1px 2px rgba(0,0,0,0.05);
  }

  .faq-list {
    display: grid;
    gap: 0;
  }

  .faq-item {
    border-bottom: 1px solid var(--color-border);
  }

  .faq-question {
    width: 100%;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 0;
    background: none;
    border: none;
    cursor: pointer;
    text-align: left;
    font-size: 0.92rem;
    font-weight: 500;
    color: var(--color-text);
    gap: 12px;
  }

  .faq-question:hover {
    color: var(--color-primary);
  }

  .faq-arrow {
    flex-shrink: 0;
    font-size: 0.7rem;
    color: var(--color-text-muted);
  }

  .faq-answer {
    padding: 0 0 14px;
    animation: fadeIn 0.2s ease;
  }

  .faq-answer p {
    font-size: 0.88rem;
    line-height: 1.7;
    color: var(--color-text-muted);
  }

  @keyframes fadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }

  .help-section ul {
    padding-left: 20px;
    margin-top: 8px;
  }

  .help-section li {
    margin-bottom: 6px;
    font-size: 0.9rem;
  }

  .help-section a {
    color: var(--color-primary);
  }

  .help-footer {
    text-align: center;
    margin-top: 24px;
  }

  .help-footer a {
    color: var(--color-primary);
    font-size: 0.9rem;
  }

  @media (max-width: 768px) {
    .help-page { padding: 16px 12px; }
    .steps-grid { grid-template-columns: repeat(2, 1fr); }
  }
</style>
