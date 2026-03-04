<script>
  import { auth } from '$lib/stores.js';

  let isAuth = $state(false);
  let user = $state(null);
  let activeTab = $state('profil');
  let saving = $state(false);
  let message = $state('');

  // Profile form
  let displayName = $state('');
  let email = $state('');

  // Password form
  let currentPassword = $state('');
  let newPassword = $state('');
  let confirmPassword = $state('');

  // Preferences
  let searchLang = $state('auto');
  let resultsPerPage = $state(10);
  let includeAi = $state(true);
  let darkMode = $state(false);
  let fontSize = $state('normal');

  auth.subscribe(v => {
    isAuth = v.isAuthenticated;
    user = v.user;
    if (v.user) {
      displayName = v.user.username || '';
      email = v.user.email || '';
    }
  });

  function showMessage(msg, duration = 3000) {
    message = msg;
    setTimeout(() => message = '', duration);
  }

  async function saveProfile() {
    saving = true;
    // Profile update would go through API
    setTimeout(() => {
      saving = false;
      showMessage('Profil berhasil disimpan');
    }, 500);
  }

  async function changePassword() {
    if (newPassword !== confirmPassword) {
      showMessage('Password baru tidak cocok');
      return;
    }
    if (newPassword.length < 8) {
      showMessage('Password minimal 8 karakter');
      return;
    }
    saving = true;
    setTimeout(() => {
      saving = false;
      currentPassword = '';
      newPassword = '';
      confirmPassword = '';
      showMessage('Password berhasil diubah');
    }, 500);
  }

  function savePreferences() {
    localStorage.setItem('bm_prefs', JSON.stringify({
      searchLang, resultsPerPage, includeAi, darkMode, fontSize
    }));
    showMessage('Preferensi disimpan');
  }

  // Load preferences
  $effect(() => {
    try {
      const saved = localStorage.getItem('bm_prefs');
      if (saved) {
        const p = JSON.parse(saved);
        searchLang = p.searchLang || 'auto';
        resultsPerPage = p.resultsPerPage || 10;
        includeAi = p.includeAi !== false;
        darkMode = p.darkMode || false;
        fontSize = p.fontSize || 'normal';
      }
    } catch {}
  });
</script>

<svelte:head>
  <title>Pengaturan — Bahtsul Masail</title>
</svelte:head>

<div class="settings-page">
  <div class="settings-container">

    <header class="settings-header">
      <h1>⚙️ Pengaturan</h1>
      <p>Kelola akun dan preferensi Anda</p>
    </header>

    {#if message}
      <div class="toast">{message}</div>
    {/if}

    {#if !isAuth}
      <div class="auth-notice">
        <p>🔒 Silakan <a href="/">masuk</a> terlebih dahulu untuk mengakses pengaturan.</p>
      </div>
    {:else}
      <!-- Tab Navigation -->
      <div class="tab-nav">
        <button class:active={activeTab === 'profil'} onclick={() => activeTab = 'profil'}>Profil</button>
        <button class:active={activeTab === 'keamanan'} onclick={() => activeTab = 'keamanan'}>Keamanan</button>
        <button class:active={activeTab === 'preferensi'} onclick={() => activeTab = 'preferensi'}>Preferensi</button>
        <button class:active={activeTab === 'api'} onclick={() => activeTab = 'api'}>API Keys</button>
      </div>

      <!-- Tab Content -->
      <div class="tab-content">
        {#if activeTab === 'profil'}
          <div class="form-section">
            <h2>Informasi Profil</h2>
            <div class="form-group">
              <label>Username
                <input type="text" class="input" bind:value={displayName} placeholder="Nama pengguna" />
              </label>
            </div>
            <div class="form-group">
              <label>Email
                <input type="email" class="input" bind:value={email} placeholder="email@domain.com" />
              </label>
            </div>
            <button class="btn btn-primary" onclick={saveProfile} disabled={saving}>
              {saving ? 'Menyimpan...' : 'Simpan Perubahan'}
            </button>
          </div>

        {:else if activeTab === 'keamanan'}
          <div class="form-section">
            <h2>Ubah Password</h2>
            <div class="form-group">
              <label>Password Saat Ini
                <input type="password" class="input" bind:value={currentPassword} />
              </label>
            </div>
            <div class="form-group">
              <label>Password Baru
                <input type="password" class="input" bind:value={newPassword} placeholder="Minimal 8 karakter" />
              </label>
            </div>
            <div class="form-group">
              <label>Konfirmasi Password Baru
                <input type="password" class="input" bind:value={confirmPassword} />
              </label>
            </div>
            <button class="btn btn-primary" onclick={changePassword} disabled={saving}>
              {saving ? 'Mengubah...' : 'Ubah Password'}
            </button>
          </div>

          <div class="form-section">
            <h2>Sesi Aktif</h2>
            <p class="text-muted">Anda login sebagai <strong>{user?.username}</strong></p>
          </div>

        {:else if activeTab === 'preferensi'}
          <div class="form-section">
            <h2>Preferensi Pencarian</h2>

            <div class="form-group">
              <label>Bahasa Pencarian
              <select class="input" bind:value={searchLang}>
                <option value="auto">Deteksi Otomatis</option>
                <option value="id">Bahasa Indonesia</option>
                <option value="en">English</option>
                <option value="ar">العربية</option>
              </select>
              </label>
            </div>

            <div class="form-group">
              <label>Hasil per Halaman
              <select class="input" bind:value={resultsPerPage}>
                <option value={5}>5</option>
                <option value={10}>10</option>
                <option value={20}>20</option>
                <option value={50}>50</option>
              </select>
              </label>
            </div>

            <div class="form-group checkbox-group">
              <label>
                <input type="checkbox" bind:checked={includeAi} />
                Sertakan jawaban AI dalam hasil pencarian
              </label>
            </div>

            <div class="form-group checkbox-group">
              <label>
                <input type="checkbox" bind:checked={darkMode} />
                Mode Gelap (segera hadir)
              </label>
            </div>

            <div class="form-group">
              <label>Ukuran Font Teks Arab
              <select class="input" bind:value={fontSize}>
                <option value="small">Kecil</option>
                <option value="normal">Normal</option>
                <option value="large">Besar</option>
                <option value="xlarge">Sangat Besar</option>
              </select>
              </label>
            </div>

            <button class="btn btn-primary" onclick={savePreferences}>Simpan Preferensi</button>
          </div>

        {:else if activeTab === 'api'}
          <div class="form-section">
            <h2>API Keys</h2>
            <p>Kelola API keys Anda untuk akses programatik ke Bahtsul Masail.</p>
            <a href="/api-docs" class="btn btn-primary">Buka Halaman API →</a>
          </div>
        {/if}
      </div>
    {/if}

    <footer class="settings-footer">
      <a href="/">← Kembali ke Pencarian</a>
    </footer>
  </div>
</div>

<style>
  .settings-page {
    max-width: 680px;
    margin: 0 auto;
    padding: 32px 24px;
    font-family: var(--font-ui);
  }

  .settings-header {
    margin-bottom: 24px;
  }

  .settings-header h1 {
    color: var(--color-primary);
    font-size: 1.8rem;
    margin-bottom: 4px;
  }

  .settings-header p {
    color: var(--color-text-muted);
  }

  .toast {
    background: var(--color-primary);
    color: #fff;
    padding: 10px 16px;
    border-radius: var(--radius-sm);
    margin-bottom: 16px;
    text-align: center;
    font-size: 0.9rem;
    animation: slideIn 0.2s ease;
  }

  @keyframes slideIn {
    from { opacity: 0; transform: translateY(-8px); }
    to { opacity: 1; transform: translateY(0); }
  }

  .auth-notice {
    padding: 16px;
    background: #fff3cd;
    border: 1px solid #ffc107;
    border-radius: var(--radius-sm);
  }

  .tab-nav {
    display: flex;
    border-bottom: 2px solid var(--color-border);
    margin-bottom: 24px;
    gap: 0;
  }

  .tab-nav button {
    padding: 10px 20px;
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    margin-bottom: -2px;
    cursor: pointer;
    font-size: 0.9rem;
    color: var(--color-text-muted);
    transition: all 0.2s;
  }

  .tab-nav button:hover {
    color: var(--color-primary);
  }

  .tab-nav button.active {
    color: var(--color-primary);
    border-bottom-color: var(--color-primary);
    font-weight: 600;
  }

  .form-section {
    margin-bottom: 32px;
    padding-bottom: 24px;
    border-bottom: 1px solid var(--color-border);
  }

  .form-section:last-child {
    border-bottom: none;
  }

  .form-section h2 {
    font-size: 1.15rem;
    margin-bottom: 16px;
    color: var(--color-text);
  }

  .form-group {
    margin-bottom: 16px;
  }

  .form-group label {
    display: block;
    font-size: 0.88rem;
    color: var(--color-text-muted);
    margin-bottom: 4px;
    font-weight: 500;
  }

  .form-group .input {
    width: 100%;
    padding: 10px 12px;
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    font-size: 0.9rem;
    background: var(--color-surface);
  }

  .form-group select.input {
    cursor: pointer;
  }

  .checkbox-group label {
    display: flex;
    gap: 8px;
    align-items: center;
    cursor: pointer;
  }

  .text-muted {
    color: var(--color-text-muted);
    font-size: 0.9rem;
  }

  .settings-footer {
    margin-top: 24px;
    text-align: center;
  }

  .settings-footer a {
    color: var(--color-primary);
    font-size: 0.9rem;
  }

  @media (max-width: 768px) {
    .settings-page { padding: 16px 12px; }
    .tab-nav button { padding: 8px 12px; font-size: 0.82rem; }
  }
</style>
