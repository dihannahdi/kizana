<script>
  import { onMount } from 'svelte';
  import { getStatus } from '$lib/api.js';

  let stats = $state(null);

  onMount(async () => {
    try {
      stats = await getStatus();
    } catch { /* ignore */ }
  });
</script>

<svelte:head>
  <title>Tentang — Bahtsul Masail</title>
</svelte:head>

<div class="about-page">
  <!-- Hero -->
  <section class="hero">
    <h1 class="hero-title">بحث المسائل</h1>
    <p class="hero-subtitle">Bahtsul Masail — Mesin Pencari Khazanah Turats Islam Klasik</p>
    <p class="hero-desc">
      Menyediakan akses ke <strong>{stats ? stats.total_books?.toLocaleString('id-ID') : '7.872'} kitab</strong> klasik Arab 
      yang mencakup fikih, tafsir, hadits, aqidah, tasawuf, dan berbagai disiplin ilmu Islam lainnya.
    </p>
  </section>

  <!-- Misi -->
  <section class="section">
    <h2 class="section-title">Misi Kami</h2>
    <div class="card-grid">
      <div class="info-card">
        <div class="card-icon">📖</div>
        <h3>Membuka Akses Khazanah</h3>
        <p>Ribuan kitab turats yang selama ini hanya bisa diakses oleh pembaca Arab kini tersedia untuk dicari dalam Bahasa Indonesia dan Inggris.</p>
      </div>
      <div class="info-card">
        <div class="card-icon">🔍</div>
        <h3>Pencarian Cerdas</h3>
        <p>Teknologi NLP menerjemahkan pertanyaan awam menjadi istilah fikih Arab yang tepat — sehingga pertanyaan "hukum foto makhluk hidup" bisa menemukan pembahasan تصوير ذوات الأرواح.</p>
      </div>
      <div class="info-card">
        <div class="card-icon">🤖</div>
        <h3>Sintesis AI Beradab</h3>
        <p>Jawaban AI selalu disertai sumber kitab dan halaman. Tidak ada fatwa tanpa dalil — sistem menerapkan prinsip <em>isnad</em> dan menampilkan khilaf ulama.</p>
      </div>
      <div class="info-card">
        <div class="card-icon">🕌</div>
        <h3>Untuk Semua Kalangan</h3>
        <p>Dari santri pesantren hingga akademisi, dari ustadz hingga masyarakat awam — semua bisa bertanya dalam bahasa mereka sendiri.</p>
      </div>
    </div>
  </section>

  <!-- CTA -->
  <section class="section section-cta">
    <h2>"العلم نور" — Ilmu adalah Cahaya</h2>
    <p>Mulai pencarian Anda sekarang.</p>
    <a href="/" class="btn btn-primary cta-btn">Mulai Mencari</a>
  </section>
</div>

<style>
  .about-page {
    direction: ltr;
    color: var(--color-text);
  }

  .hero {
    text-align: center;
    padding: 64px 24px 48px;
    background: linear-gradient(135deg, var(--color-surface) 0%, var(--color-bg) 100%);
    border-bottom: 1px solid var(--color-border);
  }

  .hero-title {
    font-family: 'Amiri', 'Noto Naskh Arabic', serif;
    font-size: 3rem;
    color: var(--color-primary);
    margin-bottom: 8px;
  }

  .hero-subtitle {
    font-size: 1.2rem;
    font-weight: 600;
    color: var(--color-text);
    margin-bottom: 12px;
  }

  .hero-desc {
    max-width: 640px;
    margin: 0 auto;
    color: var(--color-text-light);
    line-height: 1.7;
    font-size: 1rem;
  }

  .section {
    padding: 48px 24px;
    max-width: 1000px;
    margin: 0 auto;
  }

  .section-title {
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--color-text);
    margin-bottom: 28px;
    text-align: center;
  }

  .card-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 20px;
  }

  .info-card {
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 12px;
    padding: 24px;
    text-align: center;
    transition: transform 0.2s, box-shadow 0.2s;
  }

  .info-card:hover {
    transform: translateY(-2px);
    box-shadow: 0 4px 16px rgba(0,0,0,0.08);
  }

  .card-icon {
    font-size: 2rem;
    margin-bottom: 12px;
  }

  .info-card h3 {
    font-size: 1rem;
    font-weight: 600;
    margin-bottom: 8px;
    color: var(--color-text);
  }

  .info-card p {
    font-size: 0.88rem;
    color: var(--color-text-light);
    line-height: 1.6;
  }

  /* CTA */
  .section-cta {
    text-align: center;
    padding: 48px 24px;
  }

  .section-cta h2 {
    font-family: 'Amiri', 'Noto Naskh Arabic', serif;
    font-size: 1.5rem;
    margin-bottom: 8px;
    color: var(--color-text);
  }

  .section-cta p {
    color: var(--color-text-light);
    margin-bottom: 20px;
  }

  .cta-btn {
    display: inline-block;
    padding: 12px 32px;
    font-size: 1rem;
    text-decoration: none;
  }

  @media (max-width: 768px) {
    .hero {
      padding: 40px 16px 32px;
    }
    .hero-title {
      font-size: 2.2rem;
    }
    .section {
      padding: 32px 16px;
    }
  }
</style>
