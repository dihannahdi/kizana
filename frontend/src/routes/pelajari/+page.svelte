<svelte:head>
  <title>Pelajari Selengkapnya — Bahtsul Masail</title>
</svelte:head>

<div class="learn-page">
  <div class="learn-container">
    <header class="learn-header">
      <h1>📚 Pelajari Selengkapnya</h1>
      <p>Memahami bagaimana Bahtsul Masail bekerja dan mengapa kami membangunnya</p>
    </header>

    <!-- Mission -->
    <section class="learn-section">
      <h2>Misi Kami</h2>
      <div class="highlight-box">
        <p>Bahtsul Masail lahir dari kebutuhan nyata komunitas pesantren Indonesia: <strong>bagaimana menjembatani khazanah turats Islam — 7.872 kitab klasik Arab — kepada mereka yang bertanya dalam bahasa sendiri.</strong></p>
      </div>
      <p>Selama ini, untuk mencari rujukan fikih dari kitab kuning, seseorang harus: menguasai bahasa Arab, mengetahui nama kitab yang relevan, dan membuka satu per satu halaman. Bahtsul Masail mengubah ini: cukup ketik pertanyaan Anda, dalam bahasa apa pun, dan sistem akan mencarikan jawabannya dari ribuan kitab.</p>
    </section>

    <!-- How It Works -->
    <section class="learn-section">
      <h2>Cara Kerja</h2>
      <div class="process-flow">
        <div class="process-step">
          <div class="process-icon">🗣️</div>
          <h3>1. Deteksi Bahasa</h3>
          <p>Sistem mendeteksi bahasa pertanyaan Anda (Indonesia, English, العربية) dan register (awam, santri, akademik).</p>
        </div>
        <div class="process-arrow">→</div>
        <div class="process-step">
          <div class="process-icon">🔄</div>
          <h3>2. Translasi Konsep</h3>
          <p>Istilah fikih diterjemahkan ke bahasa Arab kitab: "nikah siri" → نكاح بغير ولي, "puasa" → صيام.</p>
        </div>
        <div class="process-arrow">→</div>
        <div class="process-step">
          <div class="process-icon">🔍</div>
          <h3>3. Pencarian BM25</h3>
          <p>Mesin Tantivy mencari di 3.4 juta+ entri daftar isi dan halaman kitab dengan ranking relevansi.</p>
        </div>
        <div class="process-arrow">→</div>
        <div class="process-step">
          <div class="process-icon">🤖</div>
          <h3>4. Sintesis AI</h3>
          <p>AI mensintesis jawaban dari sumber yang ditemukan — lengkap dengan rujukan kitab dan halaman.</p>
        </div>
      </div>
    </section>

    <!-- Data -->
    <section class="learn-section">
      <h2>Data & Cakupan</h2>
      <div class="stats-grid">
        <div class="stat-box">
          <span class="stat-number">7.872</span>
          <span class="stat-label">Kitab Klasik</span>
        </div>
        <div class="stat-box">
          <span class="stat-number">3.4M+</span>
          <span class="stat-label">Entri Terindeks</span>
        </div>
        <div class="stat-box">
          <span class="stat-number">20 GB</span>
          <span class="stat-label">Database</span>
        </div>
        <div class="stat-box">
          <span class="stat-number">4+</span>
          <span class="stat-label">Mazhab</span>
        </div>
      </div>
      <p>Koleksi meliputi karya-karya dari berbagai bidang ilmu Islam:</p>
      <div class="domain-tags">
        <span class="tag">Fikih</span>
        <span class="tag">Tafsir</span>
        <span class="tag">Hadits</span>
        <span class="tag">Aqidah</span>
        <span class="tag">Tasawuf</span>
        <span class="tag">Akhlak</span>
        <span class="tag">Nahwu & Sharaf</span>
        <span class="tag">Ushul Fiqh</span>
        <span class="tag">Sirah</span>
        <span class="tag">Balaghah</span>
      </div>
    </section>

    <!-- Principles -->
    <section class="learn-section">
      <h2>Prinsip Keilmuan</h2>
      <div class="principles-list">
        <div class="principle">
          <h3>📖 Isnad Mind — Setiap Klaim Bersumber</h3>
          <p>Seperti ulama hadits yang tidak menerima riwayat tanpa sanad, Bahtsul Masail <strong>tidak memberikan jawaban tanpa menunjuk ke kitab dan halaman</strong>. Ini bukan sekadar fitur — ini adab keilmuan.</p>
        </div>
        <div class="principle">
          <h3>⚖️ Khilaf adalah Rahmat</h3>
          <p>Ketika kitab menunjukkan pendapat berbeda, kami menampilkan <strong>keduanya</strong>. Khilaf ulama adalah kekayaan intelektual, bukan bug yang harus di-fix.</p>
        </div>
        <div class="principle">
          <h3>🤲 Tawadhu' Epistemik</h3>
          <p>Ketika sistem tidak cukup yakin, kami mengatakan: "Tidak ditemukan referensi yang cukup. Silakan tanyakan kepada ulama setempat." Ini bukan kelemahan — ini adab thalabul ilmi.</p>
        </div>
        <div class="principle">
          <h3>🌐 Bahasa Awam = Pertanyaan Valid</h3>
          <p>"Boleh gak makan nasi goreng yang dimasak pakai mirin?" adalah pertanyaan hukum yang serius, setara dengan "ما حكم الطبخ بالخمر". Kami melayani semua level pertanyaan dengan hormat.</p>
        </div>
      </div>
    </section>

    <!-- Tech Stack -->
    <section class="learn-section">
      <h2>Teknologi</h2>
      <div class="tech-stack">
        <div class="tech-item">
          <strong>Backend</strong>
          <span>Rust (Actix-Web) — Performa tinggi, keamanan memori</span>
        </div>
        <div class="tech-item">
          <strong>Search Engine</strong>
          <span>Tantivy BM25 — Pencarian leksikal berkecepatan tinggi</span>
        </div>
        <div class="tech-item">
          <strong>Database</strong>
          <span>SQLite — 20GB data kitab + TOC hierarki</span>
        </div>
        <div class="tech-item">
          <strong>Frontend</strong>
          <span>SvelteKit — Reactive UI dengan RTL Arabic support</span>
        </div>
        <div class="tech-item">
          <strong>AI</strong>
          <span>Large Language Model — Sintesis jawaban multibahasa</span>
        </div>
        <div class="tech-item">
          <strong>Caching</strong>
          <span>Redis — Response cache untuk kecepatan akses</span>
        </div>
      </div>
    </section>

    <footer class="learn-footer">
      <p><em>"العلم نور" — Ilmu adalah cahaya</em></p>
      <p><a href="/">← Kembali ke Pencarian</a> · <a href="/tentang">Tentang</a> · <a href="/api-docs">API</a></p>
    </footer>
  </div>
</div>

<style>
  .learn-page {
    max-width: 820px;
    margin: 0 auto;
    padding: 32px 24px;
    font-family: var(--font-ui);
  }

  .learn-header {
    text-align: center;
    margin-bottom: 40px;
  }

  .learn-header h1 {
    color: var(--color-primary);
    font-size: 2rem;
    margin-bottom: 8px;
  }

  .learn-header p {
    color: var(--color-text-muted);
    font-size: 1.05rem;
  }

  .learn-section {
    margin-bottom: 40px;
    padding-bottom: 32px;
    border-bottom: 1px solid var(--color-border);
  }

  .learn-section h2 {
    color: var(--color-primary);
    font-size: 1.35rem;
    margin-bottom: 16px;
  }

  .learn-section p {
    line-height: 1.75;
    margin-bottom: 8px;
  }

  .highlight-box {
    background: linear-gradient(135deg, var(--color-primary), #2a7f52);
    color: #fff;
    padding: 20px 24px;
    border-radius: var(--radius);
    margin-bottom: 16px;
    font-size: 1.05rem;
    line-height: 1.7;
  }

  .process-flow {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    overflow-x: auto;
    padding: 8px 0;
  }

  .process-step {
    flex: 1;
    min-width: 160px;
    padding: 16px;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
    text-align: center;
  }

  .process-icon {
    font-size: 2rem;
    margin-bottom: 8px;
  }

  .process-step h3 {
    font-size: 0.88rem;
    margin-bottom: 4px;
    color: var(--color-primary);
  }

  .process-step p {
    font-size: 0.78rem;
    color: var(--color-text-muted);
    line-height: 1.5;
  }

  .process-arrow {
    display: flex;
    align-items: center;
    font-size: 1.5rem;
    color: var(--color-secondary);
    padding-top: 32px;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px;
    margin-bottom: 16px;
  }

  .stat-box {
    text-align: center;
    padding: 16px;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
  }

  .stat-number {
    display: block;
    font-size: 1.8rem;
    font-weight: 700;
    color: var(--color-primary);
  }

  .stat-label {
    font-size: 0.78rem;
    color: var(--color-text-muted);
  }

  .domain-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 12px;
  }

  .tag {
    padding: 4px 14px;
    background: var(--color-bg-alt);
    border: 1px solid var(--color-border);
    border-radius: 20px;
    font-size: 0.82rem;
    color: var(--color-text);
  }

  .principles-list {
    display: grid;
    gap: 16px;
  }

  .principle {
    padding: 16px 20px;
    background: var(--color-surface);
    border-left: 3px solid var(--color-secondary);
    border-radius: 0 var(--radius) var(--radius) 0;
  }

  .principle h3 {
    font-size: 0.95rem;
    margin-bottom: 6px;
  }

  .principle p {
    font-size: 0.88rem;
    color: var(--color-text-muted);
  }

  .tech-stack {
    display: grid;
    gap: 8px;
  }

  .tech-item {
    display: flex;
    gap: 12px;
    padding: 10px 14px;
    background: var(--color-surface);
    border-radius: var(--radius-sm);
    font-size: 0.88rem;
    align-items: baseline;
  }

  .tech-item strong {
    min-width: 110px;
    color: var(--color-primary);
    flex-shrink: 0;
  }

  .tech-item span {
    color: var(--color-text-muted);
  }

  .learn-footer {
    text-align: center;
    padding-top: 16px;
    color: var(--color-text-muted);
    font-size: 0.9rem;
  }

  .learn-footer a {
    color: var(--color-primary);
  }

  .learn-footer em {
    font-family: var(--font-arabic);
    font-size: 1rem;
  }

  @media (max-width: 768px) {
    .learn-page { padding: 16px 12px; }
    .learn-header h1 { font-size: 1.5rem; }
    .process-flow { flex-direction: column; align-items: stretch; }
    .process-arrow { transform: rotate(90deg); justify-content: center; padding: 0; }
    .stats-grid { grid-template-columns: repeat(2, 1fr); }
    .tech-item { flex-direction: column; gap: 2px; }
    .tech-item strong { min-width: auto; }
  }
</style>
