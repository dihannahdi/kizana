<script>
  import { onMount } from 'svelte';
  import { getStatus } from '$lib/api.js';

  let stats = $state(null);
  let loading = $state(true);
  let errorMsg = $state('');

  onMount(async () => {
    try {
      stats = await getStatus();
    } catch (e) {
      errorMsg = 'Gagal memuat data statistik.';
    } finally {
      loading = false;
    }
  });

  function formatNumber(n) {
    if (!n && n !== 0) return '—';
    return n.toLocaleString('id-ID');
  }
</script>

<svelte:head>
  <title>Statistik — Bahtsul Masail</title>
</svelte:head>

<div class="stats-page">
  <section class="hero">
    <h1 class="hero-title">Statistik Sistem</h1>
    <p class="hero-desc">Data real-time tentang corpus kitab dan status mesin pencari Bahtsul Masail.</p>
  </section>

  {#if loading}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>Memuat statistik...</p>
    </div>
  {:else if errorMsg}
    <div class="error-state">
      <p>{errorMsg}</p>
      <button class="btn btn-primary" onclick={() => location.reload()}>Coba Lagi</button>
    </div>
  {:else if stats}
    <section class="stats-grid">
      <div class="stat-card primary">
        <div class="stat-icon">
          <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H20v20H6.5a2.5 2.5 0 0 1 0-5H20"/></svg>
        </div>
        <div class="stat-value">{formatNumber(stats.total_books)}</div>
        <div class="stat-label">Total Kitab</div>
        <div class="stat-detail">Koleksi kitab Islam klasik Arab dari berbagai disiplin ilmu</div>
      </div>

      <div class="stat-card accent">
        <div class="stat-icon">
          <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg>
        </div>
        <div class="stat-value">{formatNumber(stats.total_docs)}</div>
        <div class="stat-label">Entri Terindeks</div>
        <div class="stat-detail">Daftar isi dan bab yang sudah diindeks dan siap dicari</div>
      </div>

      <div class="stat-card success">
        <div class="stat-icon">
          <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>
        </div>
        <div class="stat-value status-badge" class:online={stats.status === 'ok'}>
          {stats.status === 'ok' ? 'Online' : stats.status}
        </div>
        <div class="stat-label">Status Sistem</div>
        <div class="stat-detail">Mesin pencari aktif dan siap menerima permintaan</div>
      </div>

      <div class="stat-card info">
        <div class="stat-icon">
          <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="20" x2="18" y2="10"/><line x1="12" y1="20" x2="12" y2="4"/><line x1="6" y1="20" x2="6" y2="14"/></svg>
        </div>
        <div class="stat-value">{formatNumber(stats.indexed)}</div>
        <div class="stat-label">Dokumen Diindeks</div>
        <div class="stat-detail">Total dokumen yang berhasil masuk ke mesin pencari Tantivy</div>
      </div>
    </section>

    <!-- Coverage info -->
    <section class="coverage">
      <h2 class="section-title">Cakupan Keilmuan</h2>
      <div class="domain-grid">
        <div class="domain-card">
          <span class="domain-icon">⚖️</span>
          <h3>Fikih</h3>
          <p>Ibadah, muamalat, munakahat, jinayat — hukum Islam praktis dari empat mazhab</p>
        </div>
        <div class="domain-card">
          <span class="domain-icon">📜</span>
          <h3>Tafsir</h3>
          <p>Penafsiran Al-Quran dari berbagai perspektif tafsir klasik</p>
        </div>
        <div class="domain-card">
          <span class="domain-icon">📖</span>
          <h3>Hadits</h3>
          <p>Kutub al-Sittah dan koleksi hadits lainnya beserta syarah</p>
        </div>
        <div class="domain-card">
          <span class="domain-icon">🕋</span>
          <h3>Aqidah</h3>
          <p>Tauhid, kalam, dan pembahasan pokok-pokok keimanan</p>
        </div>
        <div class="domain-card">
          <span class="domain-icon">💎</span>
          <h3>Tasawuf</h3>
          <p>Akhlak, adab, dan perjalanan spiritual menurut para sufi</p>
        </div>
        <div class="domain-card">
          <span class="domain-icon">📚</span>
          <h3>Lainnya</h3>
          <p>Ushul fiqh, sirah, tarikh, nahwu, balaghah, dan lebih banyak lagi</p>
        </div>
      </div>
    </section>

    <!-- Back to search -->
    <section class="cta-section">
      <a href="/" class="btn btn-primary cta-btn">Mulai Pencarian</a>
    </section>
  {/if}
</div>

<style>
  .stats-page {
    direction: ltr;
    color: var(--color-text);
  }

  .hero {
    text-align: center;
    padding: 48px 24px 32px;
    background: linear-gradient(135deg, var(--color-surface) 0%, var(--color-bg) 100%);
    border-bottom: 1px solid var(--color-border);
  }

  .hero-title {
    font-size: 2rem;
    font-weight: 700;
    margin-bottom: 8px;
  }

  .hero-desc {
    color: var(--color-text-light);
    max-width: 500px;
    margin: 0 auto;
  }

  .loading-state, .error-state {
    text-align: center;
    padding: 64px 24px;
    color: var(--color-text-light);
  }

  .spinner {
    width: 36px;
    height: 36px;
    border: 3px solid var(--color-border);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin: 0 auto 16px;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  /* Stats Grid */
  .stats-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 20px;
    max-width: 1000px;
    margin: 0 auto;
    padding: 40px 24px;
  }

  .stat-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 16px;
    padding: 28px 24px;
    text-align: center;
    transition: transform 0.2s, box-shadow 0.2s;
    position: relative;
    overflow: hidden;
  }

  .stat-card:hover {
    transform: translateY(-3px);
    box-shadow: 0 8px 24px rgba(0,0,0,0.08);
  }

  .stat-card::before {
    content: '';
    position: absolute;
    top: 0;
    right: 0;
    left: 0;
    height: 4px;
  }

  .stat-card.primary::before { background: var(--color-primary); }
  .stat-card.accent::before { background: #8b5cf6; }
  .stat-card.success::before { background: #10b981; }
  .stat-card.info::before { background: #3b82f6; }

  .stat-icon {
    margin-bottom: 12px;
    color: var(--color-text-light);
  }

  .stat-card.primary .stat-icon { color: var(--color-primary); }
  .stat-card.accent .stat-icon { color: #8b5cf6; }
  .stat-card.success .stat-icon { color: #10b981; }
  .stat-card.info .stat-icon { color: #3b82f6; }

  .stat-value {
    font-size: 2.4rem;
    font-weight: 800;
    margin-bottom: 4px;
    color: var(--color-text);
    font-feature-settings: 'tnum';
    font-variant-numeric: tabular-nums;
  }

  .status-badge {
    font-size: 1.5rem;
  }

  .status-badge.online {
    color: #10b981;
  }

  .stat-label {
    font-size: 0.9rem;
    font-weight: 600;
    color: var(--color-text-light);
    margin-bottom: 8px;
  }

  .stat-detail {
    font-size: 0.8rem;
    color: var(--color-text-light);
    opacity: 0.7;
    line-height: 1.4;
  }

  /* Coverage */
  .coverage {
    max-width: 1000px;
    margin: 0 auto;
    padding: 40px 24px;
  }

  .section-title {
    font-size: 1.4rem;
    font-weight: 700;
    text-align: center;
    margin-bottom: 24px;
  }

  .domain-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 16px;
  }

  .domain-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 12px;
    padding: 20px;
    text-align: center;
    transition: transform 0.2s;
  }

  .domain-card:hover {
    transform: translateY(-2px);
  }

  .domain-icon {
    font-size: 1.8rem;
    display: block;
    margin-bottom: 8px;
  }

  .domain-card h3 {
    font-size: 0.95rem;
    font-weight: 600;
    margin-bottom: 6px;
  }

  .domain-card p {
    font-size: 0.8rem;
    color: var(--color-text-light);
    line-height: 1.5;
  }

  /* CTA */
  .cta-section {
    text-align: center;
    padding: 32px 24px 48px;
  }

  .cta-btn {
    display: inline-block;
    padding: 12px 32px;
    font-size: 1rem;
    text-decoration: none;
  }

  @media (max-width: 768px) {
    .hero {
      padding: 32px 16px 24px;
    }
    .hero-title {
      font-size: 1.6rem;
    }
    .stats-grid {
      padding: 24px 16px;
    }
    .stat-value {
      font-size: 1.8rem;
    }
    .coverage {
      padding: 24px 16px;
    }
  }
</style>
