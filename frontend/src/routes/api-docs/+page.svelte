<script>
  import { auth } from '$lib/stores.js';
  import { getApiKeys, createApiKey, revokeApiKey } from '$lib/api.js';

  let isAuth = $state(false);
  let authUser = $state(null);
  let apiKeys = $state([]);
  let newKeyName = $state('');
  let newKeyCreated = $state(null);
  let loading = $state(false);
  let error = $state('');

  auth.subscribe(v => { isAuth = v.isAuthenticated; authUser = v.user; });

  async function loadKeys() {
    if (!isAuth) return;
    try {
      apiKeys = await getApiKeys();
    } catch (e) {
      // ignore
    }
  }

  async function handleCreateKey() {
    if (!newKeyName.trim()) return;
    loading = true;
    error = '';
    try {
      const result = await createApiKey(newKeyName.trim());
      newKeyCreated = result;
      newKeyName = '';
      await loadKeys();
    } catch (e) {
      error = e.message;
    } finally {
      loading = false;
    }
  }

  async function handleRevoke(keyId) {
    if (!confirm('Cabut API key ini? Aksi ini tidak dapat dibatalkan.')) return;
    try {
      await revokeApiKey(keyId);
      await loadKeys();
    } catch (e) {
      error = e.message;
    }
  }

  $effect(() => {
    if (isAuth) loadKeys();
  });

  const curlExample = `curl -X POST https://bahtsulmasail.tech/api/v1/search \\
  -H "Content-Type: application/json" \\
  -H "X-API-Key: bm_your_api_key_here" \\
  -d '{"query": "hukum shalat jamak qasar"}'`;

  const requestExample = JSON.stringify({
    query: "hukum nikah siri menurut mazhab syafii",
    max_results: 10,
    include_ai: true
  }, null, 2);

  const responseExample = JSON.stringify({
    query: "hukum nikah siri",
    result_count: 5,
    detected_language: "Indonesian",
    detected_domain: "Munakahat",
    translated_terms: ["نكاح سري", "نكاح بغير ولي"],
    ai_answer: "Berdasarkan kitab-kitab...",
    results: [{
      book_id: 1234,
      title: "باب النكاح",
      content_snippet: "...",
      page: "45",
      score: 85.2,
      book_name: "المجموع شرح المهذب",
      author_name: "الإمام النووي",
      source_type: "kitab"
    }]
  }, null, 2);

  const mcpExample = JSON.stringify({
    mcpServers: {
      bahtsulmasail: {
        url: "https://bahtsulmasail.tech/api/v1/search",
        headers: { "X-API-Key": "bm_your_api_key_here" },
        description: "Search 7,800+ classical Islamic texts (kitab kuning)"
      }
    }
  }, null, 2);
</script>

<svelte:head>
  <title>API Documentation — Bahtsul Masail</title>
</svelte:head>

<div class="api-docs">
  <div class="docs-container">
    <header class="docs-header">
      <h1>🔌 API Documentation</h1>
      <p class="docs-subtitle">Bahtsul Masail API v1 — Akses programatik ke 7.800+ kitab Islam klasik</p>
    </header>

    <!-- Overview -->
    <section class="docs-section">
      <h2>Ikhtisar</h2>
      <p>API Bahtsul Masail memungkinkan AI agents, aplikasi, dan developer mengakses mesin pencari kitab Islam klasik secara programatik. Gunakan API ini untuk:</p>
      <ul>
        <li>Mencari referensi fikih dari ribuan kitab</li>
        <li>Mendapatkan sintesis AI dari hasil pencarian</li>
        <li>Membaca halaman kitab secara langsung</li>
        <li>Mengintegrasikan dengan tools AI (MCP, LangChain, dsb.)</li>
      </ul>
    </section>

    <!-- Authentication -->
    <section class="docs-section">
      <h2>Autentikasi</h2>
      <p>Semua request API menggunakan API key via header <code>X-API-Key</code>:</p>
      <div class="code-block">
        <pre><code>{curlExample}</code></pre>
      </div>
    </section>

    <!-- Endpoints -->
    <section class="docs-section">
      <h2>Endpoints</h2>

      <div class="endpoint">
        <div class="endpoint-header">
          <span class="method post">POST</span>
          <code>/api/v1/search</code>
        </div>
        <p>Cari referensi di kitab-kitab Islam klasik dengan AI synthesis.</p>
        
        <h4>Request Body</h4>
        <div class="code-block">
          <pre><code>{requestExample}</code></pre>
        </div>

        <table class="params-table">
          <thead><tr><th>Parameter</th><th>Tipe</th><th>Wajib</th><th>Deskripsi</th></tr></thead>
          <tbody>
            <tr><td><code>query</code></td><td>string</td><td>Ya</td><td>Pertanyaan pencarian (Bahasa Indonesia, English, atau العربية)</td></tr>
            <tr><td><code>max_results</code></td><td>integer</td><td>Tidak</td><td>Jumlah maksimal hasil (1-50, default: 10)</td></tr>
            <tr><td><code>include_ai</code></td><td>boolean</td><td>Tidak</td><td>Sertakan jawaban AI (default: true)</td></tr>
          </tbody>
        </table>

        <h4>Response</h4>
        <div class="code-block">
          <pre><code>{responseExample}</code></pre>
        </div>
      </div>
    </section>

    <!-- MCP Integration -->
    <section class="docs-section">
      <h2>MCP Integration (AI Agents)</h2>
      <p>Bahtsul Masail dapat digunakan sebagai MCP (Model Context Protocol) server untuk AI agents:</p>
      <div class="code-block">
        <pre><code>{mcpExample}</code></pre>
      </div>
      <p>Atau gunakan langsung via HTTP request di tool definition AI agent Anda.</p>
    </section>

    <!-- Rate Limits -->
    <section class="docs-section">
      <h2>Rate Limits</h2>
      <table class="params-table">
        <thead><tr><th>Plan</th><th>Rate Limit</th><th>Max Results</th></tr></thead>
        <tbody>
          <tr><td>Default</td><td>30 req/menit</td><td>50 per query</td></tr>
        </tbody>
      </table>
      <p>Rate limit headers disertakan di setiap response. Jika melebihi batas, Anda akan menerima HTTP 429.</p>
    </section>

    <!-- Error Codes -->
    <section class="docs-section">
      <h2>Error Codes</h2>
      <table class="params-table">
        <thead><tr><th>Status</th><th>Arti</th></tr></thead>
        <tbody>
          <tr><td><code>200</code></td><td>Sukses</td></tr>
          <tr><td><code>400</code></td><td>Parameter tidak valid</td></tr>
          <tr><td><code>401</code></td><td>API key tidak valid atau hilang</td></tr>
          <tr><td><code>403</code></td><td>Tidak memiliki permission</td></tr>
          <tr><td><code>429</code></td><td>Rate limit terlampaui</td></tr>
          <tr><td><code>500</code></td><td>Kesalahan server</td></tr>
        </tbody>
      </table>
    </section>

    <!-- API Keys Management -->
    <section class="docs-section">
      <h2>Kelola API Keys</h2>
      
      {#if !isAuth}
        <div class="auth-notice">
          <p>🔒 Silakan <a href="/">masuk</a> terlebih dahulu untuk membuat dan mengelola API keys.</p>
        </div>
      {:else}
        <!-- Create new key -->
        <div class="key-create">
          <h3>Buat API Key Baru</h3>
          <div class="key-form">
            <input type="text" class="input" bind:value={newKeyName} placeholder="Nama key (mis. 'Development', 'Production')" />
            <button class="btn btn-primary" onclick={handleCreateKey} disabled={loading || !newKeyName.trim()}>
              {loading ? 'Membuat...' : '+ Buat Key'}
            </button>
          </div>

          {#if error}
            <p class="error-msg">{error}</p>
          {/if}

          {#if newKeyCreated}
            <div class="key-reveal">
              <p><strong>⚠️ Salin API key ini sekarang!</strong> Key ini hanya ditampilkan sekali.</p>
              <div class="key-display">
                <code>{newKeyCreated.api_key}</code>
                <button class="btn btn-ghost btn-sm" onclick={() => { navigator.clipboard.writeText(newKeyCreated.api_key); }}>📋 Salin</button>
              </div>
              <button class="btn btn-ghost btn-sm" onclick={() => newKeyCreated = null}>Tutup</button>
            </div>
          {/if}
        </div>

        <!-- Existing keys -->
        <div class="key-list">
          <h3>API Keys Anda</h3>
          {#if apiKeys.length === 0}
            <p class="empty">Belum ada API key. Buat satu untuk mulai menggunakan API.</p>
          {:else}
            <table class="params-table">
              <thead><tr><th>Nama</th><th>Prefix</th><th>Status</th><th>Rate Limit</th><th>Terakhir Digunakan</th><th>Aksi</th></tr></thead>
              <tbody>
                {#each apiKeys as key}
                  <tr>
                    <td>{key.name}</td>
                    <td><code>{key.key_prefix}...</code></td>
                    <td><span class="status-badge" class:active={key.is_active} class:revoked={!key.is_active}>{key.is_active ? 'Aktif' : 'Dicabut'}</span></td>
                    <td>{key.rate_limit}/min</td>
                    <td>{key.last_used_at ? new Date(key.last_used_at).toLocaleDateString('id-ID') : 'Belum pernah'}</td>
                    <td>
                      {#if key.is_active}
                        <button class="btn btn-ghost btn-sm" onclick={() => handleRevoke(key.id)}>Cabut</button>
                      {:else}
                        <span class="text-muted">—</span>
                      {/if}
                    </td>
                  </tr>
                {/each}
              </tbody>
            </table>
          {/if}
        </div>
      {/if}
    </section>

    <footer class="docs-footer">
      <p>Butuh bantuan? Hubungi <strong>admin@bahtsulmasail.tech</strong></p>
      <p><a href="/">← Kembali ke Pencarian</a></p>
    </footer>
  </div>
</div>

<style>
  .api-docs {
    max-width: 860px;
    margin: 0 auto;
    padding: 32px 24px;
    font-family: var(--font-ui);
  }

  .docs-header {
    text-align: center;
    margin-bottom: 40px;
  }

  .docs-header h1 {
    color: var(--color-primary);
    font-size: 2rem;
    margin-bottom: 8px;
  }

  .docs-subtitle {
    color: var(--color-text-light);
    font-size: 1.1rem;
  }

  .docs-section {
    margin-bottom: 40px;
    padding-bottom: 32px;
    border-bottom: 1px solid var(--color-border);
  }

  .docs-section h2 {
    color: var(--color-primary);
    font-size: 1.4rem;
    margin-bottom: 12px;
  }

  .docs-section h3 {
    font-size: 1.1rem;
    color: var(--color-text);
    margin: 16px 0 8px;
  }

  .docs-section h4 {
    font-size: 0.95rem;
    color: var(--color-text-muted);
    margin: 12px 0 6px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .docs-section p {
    line-height: 1.7;
    margin-bottom: 8px;
  }

  .docs-section ul {
    padding-left: 24px;
    margin-bottom: 12px;
  }

  .docs-section li {
    margin-bottom: 6px;
    line-height: 1.6;
  }

  code {
    background: var(--color-bg-alt);
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 0.88rem;
    font-family: 'Fira Code', 'Consolas', monospace;
  }

  .code-block {
    background: #1e1e2e;
    color: #cdd6f4;
    border-radius: var(--radius);
    padding: 16px 20px;
    overflow-x: auto;
    margin: 8px 0 16px;
  }

  .code-block pre { margin: 0; }
  .code-block code {
    background: none;
    padding: 0;
    color: inherit;
    font-size: 0.85rem;
    line-height: 1.6;
  }

  .params-table {
    width: 100%;
    border-collapse: collapse;
    margin: 8px 0;
    font-size: 0.88rem;
  }

  .params-table th {
    text-align: left;
    padding: 8px 12px;
    background: var(--color-bg-alt);
    border-bottom: 2px solid var(--color-border);
    font-size: 0.82rem;
    text-transform: uppercase;
    letter-spacing: 0.3px;
    color: var(--color-text-muted);
  }

  .params-table td {
    padding: 8px 12px;
    border-bottom: 1px solid var(--color-border);
  }

  .endpoint {
    margin: 16px 0;
    padding: 16px;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius);
  }

  .endpoint-header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 8px;
  }

  .method {
    padding: 3px 10px;
    border-radius: 4px;
    font-weight: 700;
    font-size: 0.78rem;
    text-transform: uppercase;
  }

  .method.post { background: #e8f5e9; color: #2e7d32; }

  .auth-notice {
    padding: 16px;
    background: #fff3cd;
    border: 1px solid #ffc107;
    border-radius: var(--radius-sm);
  }

  .key-create {
    margin: 12px 0 24px;
  }

  .key-form {
    display: flex;
    gap: 8px;
    margin-bottom: 8px;
  }

  .key-form .input {
    flex: 1;
  }

  .key-reveal {
    background: #e8f5e9;
    border: 1px solid #2e7d32;
    border-radius: var(--radius-sm);
    padding: 12px 16px;
    margin-top: 8px;
  }

  .key-display {
    display: flex;
    gap: 8px;
    align-items: center;
    margin: 8px 0;
    font-family: monospace;
    word-break: break-all;
  }

  .key-display code {
    background: #1e1e2e;
    color: #a6e3a1;
    padding: 8px 12px;
    border-radius: 4px;
    flex: 1;
    font-size: 0.85rem;
  }

  .key-list {
    margin-top: 16px;
  }

  .status-badge {
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 0.75rem;
    font-weight: 600;
  }

  .status-badge.active { background: #e8f5e9; color: #2e7d32; }
  .status-badge.revoked { background: #fce4ec; color: #c62828; }

  .error-msg { color: #c62828; font-size: 0.88rem; }
  .empty { color: var(--color-text-muted); font-size: 0.88rem; }
  .text-muted { color: var(--color-text-muted); }

  .docs-footer {
    text-align: center;
    padding-top: 24px;
    color: var(--color-text-muted);
    font-size: 0.9rem;
  }

  .docs-footer a {
    color: var(--color-primary);
  }

  @media (max-width: 768px) {
    .api-docs { padding: 16px 12px; }
    .docs-header h1 { font-size: 1.5rem; }
    .key-form { flex-direction: column; }
    .params-table { font-size: 0.8rem; }
    .params-table th, .params-table td { padding: 6px 8px; }
  }
</style>
