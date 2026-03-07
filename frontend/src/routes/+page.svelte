<script>
  import { auth, searchResults, aiAnswer, isLoading, error, showReader, bookData, currentSession } from '$lib/stores.js';
  import { sendQuery, sendQueryStream, readBook, login as apiLogin, register as apiRegister, getSessions, getSession, deleteSession, renameSession } from '$lib/api.js';
  import { onMount, onDestroy } from 'svelte';
  import { marked } from 'marked';
  import DOMPurify from 'dompurify';

  // Configure marked for safe rendering
  marked.setOptions({ breaks: true, gfm: true });

  let query = $state('');
  let showAuth = $state(false);
  let authMode = $state('login');
  let email = $state('');
  let password = $state('');
  let displayName = $state('');
  let authError = $state('');
  let editingSessionId = $state(null);
  let editingSessionTitle = $state('');
  let sessions = $state([]);
  let showHistory = $state(false);
  let readerBookId = $state(null);
  let readerData = $state(null);
  let readerPage = $state(null);
  let readerLoading = $state(false);
  let readerBookName = $state('');
  let highlightSnippet = $state('');
  let showMobileToc = $state(false);

  let isAuth = $state(false);
  let authUser = $state(null);
  let results = $state([]);
  let answer = $state('');
  let loading = $state(false);
  let errorMsg = $state('');
  let showReaderPanel = $state(false);
  let sessionId = $state(null);
  let detectedLanguage = $state('');
  let detectedDomain = $state('');
  let translatedTerms = $state([]);
  let isStreaming = $state(false);
  let streamingAnswer = $state('');

  // ─── New Features State ───
  // Task 1: Resizable panels
  let panelRatio = $state(0.42); // chat panel takes 42% by default
  let isResizing = $state(false);
  let layoutEl = $state(null);

  // Task 9: Chat history search
  let historySearch = $state('');
  let filteredSessions = $derived(
    historySearch.trim()
      ? sessions.filter(s => (s.title || '').toLowerCase().includes(historySearch.toLowerCase()))
      : sessions
  );

  // Task 10: Projects
  let projects = $state([]);
  let activeProject = $state(null);
  let showProjectMenu = $state(false);
  let newProjectName = $state('');

  // Task 13: Keyboard shortcuts
  let showShortcuts = $state(false);

  // Task 6: Export
  let showExportMenu = $state(false);
  let exportLoading = $state(false);

  // Task 8: Quick actions
  let showQuickActions = $state(false);
  const quickActionSuggestions = [
    { label: 'Hukum shalat jamak qasar', icon: '🕌' },
    { label: 'Syarat sah jual beli', icon: '💰' },
    { label: 'Tata cara tayammum', icon: '💧' },
    { label: 'Hukum nikah siri', icon: '💍' },
    { label: 'Pembagian waris', icon: '📜' },
    { label: 'Hukum riba dalam Islam', icon: '🏦' },
  ];

  // Subscribe to stores
  auth.subscribe(v => { isAuth = v.isAuthenticated; authUser = v.user; });
  searchResults.subscribe(v => results = v);
  aiAnswer.subscribe(v => answer = v);
  isLoading.subscribe(v => loading = v);
  error.subscribe(v => errorMsg = v);
  showReader.subscribe(v => showReaderPanel = v);

  function renderMarkdown(text) {
    if (!text) return '';
    // Make references clickable: [Kitab X, hal. Y] → clickable link
    let processed = text.replace(
      /\[([^\]]*?(?:Kitab|kitab|كتاب)[^\]]*?)\]/g,
      '<span class="ai-ref" title="Klik untuk detail">📖 $1</span>'
    );
    const html = marked.parse(processed);
    return DOMPurify.sanitize(html, {
      ALLOWED_TAGS: ['p', 'br', 'strong', 'em', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
                     'ul', 'ol', 'li', 'blockquote', 'code', 'pre', 'hr',
                     'span', 'mark', 'a', 'table', 'thead', 'tbody', 'tr', 'th', 'td', 'div', 'sup', 'sub'],
      ALLOWED_ATTR: ['class', 'href', 'title', 'dir', 'id'],
      FORBID_ATTR: ['onclick', 'onerror', 'onload', 'onmouseover']
    });
  }

  async function handleAuth() {
    authError = '';
    try {
      let data;
      if (authMode === 'login') {
        data = await apiLogin(email, password);
      } else {
        data = await apiRegister(email, password, displayName || undefined);
      }
      auth.login(data.token, data.user);
      showAuth = false;
      email = '';
      password = '';
      displayName = '';
      loadSessions();
    } catch (e) {
      authError = e.message;
    }
  }

  async function handleQuery() {
    if (!query.trim()) return;
    if (!isAuth) {
      showAuth = true;
      return;
    }

    isLoading.set(true);
    error.set('');
    showQuickActions = false;
    aiAnswer.set('');
    streamingAnswer = '';
    isStreaming = true;

    try {
      await sendQueryStream(query, sessionId, {
        onResults(data) {
          searchResults.set(data.results);
          detectedLanguage = data.detected_language || '';
          detectedDomain = data.detected_domain || '';
          translatedTerms = data.translated_terms || [];
          // Results arrived — stop full loading spinner, keep streaming indicator
          isLoading.set(false);
        },
        onChunk(content) {
          streamingAnswer += content;
        },
        onDone(data) {
          // Final complete answer + session
          aiAnswer.set(data.ai_answer || streamingAnswer);
          sessionId = data.session_id;
          isStreaming = false;
          streamingAnswer = '';
          currentSession.set(data);
          loadSessions();
        }
      });
    } catch (e) {
      error.set(e.message);
      // Fallback to non-streaming
      try {
        const data = await sendQuery(query, sessionId);
        searchResults.set(data.results);
        aiAnswer.set(data.ai_answer);
        sessionId = data.session_id;
        detectedLanguage = data.detected_language || '';
        detectedDomain = data.detected_domain || '';
        translatedTerms = data.translated_terms || [];
        currentSession.set(data);
        loadSessions();
      } catch (e2) {
        error.set(e2.message);
      }
    } finally {
      isLoading.set(false);
      isStreaming = false;
    }
  }

  async function loadSessions() {
    try {
      sessions = await getSessions();
    } catch (e) {
      // ignore
    }
  }

  // Load a specific session and restore its chat data
  async function loadSession(sid) {
    isLoading.set(true);
    error.set('');
    try {
      const session = await getSession(sid);
      sessionId = session.id;
      currentSession.set(session);

      // Find the last user query to display
      const userMsgs = (session.messages || []).filter(m => m.role === 'user');
      const assistantMsgs = (session.messages || []).filter(m => m.role === 'assistant');
      const lastUserMsg = userMsgs.length > 0 ? userMsgs[userMsgs.length - 1] : null;
      const lastAssistantMsg = assistantMsgs.length > 0 ? assistantMsgs[assistantMsgs.length - 1] : null;

      if (lastAssistantMsg) {
        aiAnswer.set(lastAssistantMsg.content);
      }

      // Re-run the last query to get search results
      if (lastUserMsg) {
        query = lastUserMsg.content;
        const data = await sendQuery(lastUserMsg.content, session.id);
        searchResults.set(data.results);
        aiAnswer.set(data.ai_answer);
        detectedLanguage = data.detected_language || '';
        detectedDomain = data.detected_domain || '';
        translatedTerms = data.translated_terms || [];
      } else {
        searchResults.set([]);
      }
    } catch (e) {
      error.set('Gagal memuat sesi: ' + e.message);
    } finally {
      isLoading.set(false);
    }
  }

  // ─── Projects (Task 10) ───
  function loadProjects() {
    if (typeof window !== 'undefined') {
      const stored = localStorage.getItem('bm_projects');
      if (stored) {
        try { projects = JSON.parse(stored); } catch { projects = []; }
      }
    }
  }
  function saveProjects() {
    if (typeof window !== 'undefined') {
      localStorage.setItem('bm_projects', JSON.stringify(projects));
    }
  }
  function createProject() {
    if (!newProjectName.trim()) return;
    projects = [...projects, { id: Date.now().toString(), name: newProjectName.trim(), sessionIds: [] }];
    saveProjects();
    newProjectName = '';
  }
  function addSessionToProject(projectId, sessId) {
    projects = projects.map(p => {
      if (p.id === projectId && !p.sessionIds.includes(sessId)) {
        return { ...p, sessionIds: [...p.sessionIds, sessId] };
      }
      return p;
    });
    saveProjects();
  }
  function deleteProject(projectId) {
    projects = projects.filter(p => p.id !== projectId);
    saveProjects();
  }

  async function openBook(bookId, page, bookName = '', snippet = '') {
    readerLoading = true;
    readerBookId = bookId;
    readerBookName = bookName;
    highlightSnippet = snippet;
    try {
      readerData = await readBook(bookId, page);
      readerPage = page;
      if (readerData.book_name) {
        readerBookName = readerData.book_name;
      }
      showReader.set(true);
      if (snippet) {
        setTimeout(() => {
          const mark = document.querySelector('.reader-content .search-highlight');
          if (mark) mark.scrollIntoView({ behavior: 'smooth', block: 'center' });
        }, 200);
      }
    } catch (e) {
      error.set('Gagal memuat kitab: ' + e.message);
    } finally {
      readerLoading = false;
    }
  }

  function scrollToResult(index) {
    const el = document.getElementById(`result-${index}`);
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'center' });
      el.classList.add('result-highlighted');
      setTimeout(() => el.classList.remove('result-highlighted'), 2000);
    }
  }

  function highlightContent(html) {
    if (!highlightSnippet || !html) return html;
    const snippet = highlightSnippet.replace(/<[^>]*>/g, '').trim();
    if (snippet.length < 5) return html;
    const segments = [];
    const fullSnip = snippet.substring(0, 80);
    if (fullSnip.length >= 10) segments.push(fullSnip);
    for (let i = 0; i < snippet.length && segments.length < 5; i += 30) {
      const seg = snippet.substring(i, i + 30).trim();
      if (seg.length >= 8) segments.push(seg);
    }
    let result = html;
    for (const seg of segments) {
      const escaped = seg.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      try {
        const re = new RegExp(`(${escaped})`, 'g');
        result = result.replace(re, '<mark class="search-highlight">$1</mark>');
      } catch { /* ignore invalid regex */ }
    }
    return DOMPurify.sanitize(result, {
      ALLOWED_TAGS: ['mark', 'br', 'span', 'p', 'div', 'b', 'i', 'em', 'strong', 'sup', 'sub'],
      ALLOWED_ATTR: ['class', 'dir', 'data-type']
    });
  }

  async function navigatePage(page) {
    if (!readerBookId) return;
    readerLoading = true;
    highlightSnippet = '';
    try {
      readerData = await readBook(readerBookId, page);
      readerPage = page;
      showMobileToc = false;
    } catch (e) {
      // ignore
    } finally {
      readerLoading = false;
    }
  }

  // ─── Resizable Panel (Task 1) ───
  function startResize(e) {
    e.preventDefault();
    isResizing = true;
    document.body.style.cursor = 'col-resize';
    document.body.style.userSelect = 'none';
    window.addEventListener('mousemove', onResize);
    window.addEventListener('mouseup', stopResize);
  }
  function onResize(e) {
    if (!isResizing || !layoutEl) return;
    const rect = layoutEl.getBoundingClientRect();
    let ratio = (e.clientX - rect.left) / rect.width;
    ratio = Math.max(0.25, Math.min(0.75, ratio));
    panelRatio = ratio;
  }
  function stopResize() {
    isResizing = false;
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
    window.removeEventListener('mousemove', onResize);
    window.removeEventListener('mouseup', stopResize);
  }

  // ─── Keyboard Shortcuts (Task 13) ───
  function handleGlobalKeydown(e) {
    // Ctrl+K or Cmd+K → Focus search
    if ((e.ctrlKey || e.metaKey) && e.key === 'k') {
      e.preventDefault();
      const input = document.querySelector('.search-input');
      if (input) input.focus();
    }
    // Ctrl+. → Toggle history sidebar
    if ((e.ctrlKey || e.metaKey) && e.key === '.') {
      e.preventDefault();
      if (isAuth) {
        if (!showHistory) loadSessions();
        showHistory = !showHistory;
      }
    }
    // Ctrl+/ → Show shortcuts
    if ((e.ctrlKey || e.metaKey) && e.key === '/') {
      e.preventDefault();
      showShortcuts = !showShortcuts;
    }
    // Ctrl+N → New chat
    if ((e.ctrlKey || e.metaKey) && e.key === 'n') {
      e.preventDefault();
      newChat();
    }
    // Escape → Close modals/panels
    if (e.key === 'Escape') {
      if (showShortcuts) { showShortcuts = false; return; }
      if (showAuth) { showAuth = false; return; }
      if (showHistory) { showHistory = false; return; }
      if (showQuickActions) { showQuickActions = false; return; }
    }
  }

  function handleKeydown(e) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleQuery();
    }
  }

  function handleLogout() {
    auth.logout();
    searchResults.set([]);
    aiAnswer.set('');
    sessions = [];
    sessionId = null;
    showReader.set(false);
  }

  function newChat() {
    searchResults.set([]);
    aiAnswer.set('');
    query = '';
    sessionId = null;
    showReader.set(false);
  }

  async function handleDeleteSession(id, event) {
    event.stopPropagation();
    if (!confirm('Hapus riwayat pencarian ini?')) return;
    try {
      await deleteSession(id);
      sessions = sessions.filter(s => s.id !== id);
      if (sessionId === id) {
        newChat();
      }
    } catch (e) {
      error.set('Gagal menghapus: ' + e.message);
    }
  }

  function startRenameSession(id, title, event) {
    event.stopPropagation();
    editingSessionId = id;
    editingSessionTitle = title || '';
  }

  async function handleRenameSession(id) {
    if (!editingSessionTitle.trim()) {
      editingSessionId = null;
      return;
    }
    try {
      await renameSession(id, editingSessionTitle.trim());
      sessions = sessions.map(s => s.id === id ? { ...s, title: editingSessionTitle.trim() } : s);
    } catch (e) {
      error.set('Gagal mengubah nama: ' + e.message);
    }
    editingSessionId = null;
  }

  function handleQuickAction(text) {
    query = text;
    showQuickActions = false;
    handleQuery();
  }

  // ─── Export (Task 6) ───
  function buildExportContent() {
    const lines = [];
    lines.push('# Bahtsul Masail — Hasil Pencarian');
    lines.push(`**Pertanyaan:** ${query}`);
    lines.push(`**Tanggal:** ${new Date().toLocaleString('id-ID')}`);
    if (detectedLanguage) lines.push(`**Bahasa terdeteksi:** ${detectedLanguage}`);
    if (detectedDomain) lines.push(`**Domain:** ${detectedDomain}`);
    if (translatedTerms.length > 0) lines.push(`**Istilah Arab:** ${translatedTerms.join(', ')}`);
    lines.push('');

    if (answer) {
      lines.push('---');
      lines.push('## Jawaban AI');
      lines.push('');
      lines.push(answer);
      lines.push('');
    }

    if (results.length > 0) {
      lines.push('---');
      lines.push(`## Hasil Pencarian (${results.length} referensi)`);
      lines.push('');
      results.forEach((r, i) => {
        lines.push(`### [${i + 1}] ${r.title || 'Tanpa judul'}`);
        lines.push(`- **Kitab:** ${r.book_name || `Kitab ${r.book_id}`}`);
        if (r.author_name) lines.push(`- **Pengarang:** ${r.author_name}`);
        if (r.category) lines.push(`- **Kategori:** ${r.category}`);
        lines.push(`- **Halaman:** ${r.page}`);
        lines.push(`- **Relevansi:** ${Math.round(r.score)}%`);
        if (r.hierarchy && r.hierarchy.length > 0) {
          lines.push(`- **Bab:** ${r.hierarchy.join(' › ')}`);
        }
        if (r.content_snippet) {
          lines.push('');
          lines.push(`> ${r.content_snippet}`);
        }
        lines.push('');
      });
    }

    lines.push('---');
    lines.push('*Diekspor dari bahtsulmasail.tech — Mesin pencari khazanah turats Islam*');
    return lines.join('\n');
  }

  function exportMarkdown() {
    const content = buildExportContent();
    const blob = new Blob([content], { type: 'text/markdown;charset=utf-8' });
    downloadBlob(blob, `bahtsul-masail-${Date.now()}.md`);
    showExportMenu = false;
  }

  function exportPlainText() {
    const content = buildExportContent()
      .replace(/#{1,6}\s/g, '')
      .replace(/\*\*/g, '')
      .replace(/\*/g, '')
      .replace(/^>\s/gm, '  ');
    const blob = new Blob([content], { type: 'text/plain;charset=utf-8' });
    downloadBlob(blob, `bahtsul-masail-${Date.now()}.txt`);
    showExportMenu = false;
  }

  async function exportDocx() {
    exportLoading = true;
    showExportMenu = false;
    try {
      const content = buildExportContent();
      // Use simple HTML-to-DOCX approach via Blob
      const htmlContent = `
        <html xmlns:o='urn:schemas-microsoft-com:office:office' 
              xmlns:w='urn:schemas-microsoft-com:office:word' 
              xmlns='http://www.w3.org/TR/REC-html40'>
        <head><meta charset="utf-8">
        <style>
          body { font-family: 'Calibri', sans-serif; direction: ltr; line-height: 1.8; }
          h1 { color: #1a5f3a; font-size: 18pt; }
          h2 { color: #1a5f3a; font-size: 14pt; margin-top: 16pt; }
          h3 { font-size: 12pt; margin-top: 12pt; }
          blockquote { border-left: 3px solid #c9a84c; padding-left: 12px; color: #555; font-family: 'Amiri', serif; }
          .meta { color: #666; font-size: 10pt; }
          hr { border: none; border-top: 1px solid #ddd; }
        </style></head><body>`;
      
      const htmlBody = DOMPurify.sanitize(marked.parse(content));
      const fullHtml = htmlContent + htmlBody + '</body></html>';
      const blob = new Blob(['\ufeff', fullHtml], { type: 'application/msword;charset=utf-8' });
      downloadBlob(blob, `bahtsul-masail-${Date.now()}.doc`);
    } catch (e) {
      error.set('Gagal export: ' + e.message);
    } finally {
      exportLoading = false;
    }
  }

  function exportPdf() {
    // Use browser print with custom styling
    const content = buildExportContent();
    const htmlBody = DOMPurify.sanitize(marked.parse(content));
    const printWindow = window.open('', '_blank');
    if (!printWindow) {
      error.set('Popup blocked. Izinkan popup untuk export PDF.');
      return;
    }
    printWindow.document.write(`<!DOCTYPE html><html><head><meta charset="utf-8">
      <title>Bahtsul Masail — Hasil Pencarian</title>
      <link href="https://fonts.googleapis.com/css2?family=Amiri&family=Cairo:wght@400;600;700&display=swap" rel="stylesheet">
      <style>
        body { font-family: 'Cairo', 'Calibri', sans-serif; max-width: 700px; margin: 40px auto; line-height: 1.9; color: #333; padding: 0 20px; }
        h1 { color: #1a5f3a; border-bottom: 2px solid #c9a84c; padding-bottom: 8px; }
        h2 { color: #1a5f3a; margin-top: 24px; }
        h3 { color: #2e7d32; }
        blockquote { border-left: 3px solid #c9a84c; margin: 8px 0; padding: 8px 16px; background: #fffdf5; font-family: 'Amiri', serif; font-size: 1.05em; line-height: 2.2; }
        hr { border: none; border-top: 1px solid #ddd; margin: 16px 0; }
        strong { color: #1a5f3a; }
        em { color: #666; }
        @media print { body { margin: 0; padding: 20px; } }
      </style></head><body>${htmlBody}</body></html>`);
    printWindow.document.close();
    setTimeout(() => { printWindow.print(); }, 500);
    showExportMenu = false;
  }

  function downloadBlob(blob, filename) {
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = filename;
    a.click();
    URL.revokeObjectURL(url);
  }

  // When auth state changes (e.g. after restore), load sessions
  let prevAuth = false;
  $effect(() => {
    if (isAuth && !prevAuth) {
      loadSessions();
      loadProjects();
    }
    prevAuth = isAuth;
  });

  onMount(() => {
    if (isAuth) { loadSessions(); loadProjects(); }
    window.addEventListener('keydown', handleGlobalKeydown);
  });

  onDestroy(() => {
    if (typeof window !== 'undefined') {
      window.removeEventListener('keydown', handleGlobalKeydown);
    }
  });
</script>

<svelte:head>
  <title>Bahtsul Masail — bahtsulmasail.tech</title>
</svelte:head>

<!-- Keyboard Shortcuts Modal (Task 13) -->
{#if showShortcuts}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="modal-overlay" onclick={() => showShortcuts = false} onkeydown={(e) => { if (e.key === 'Escape') showShortcuts = false; }} role="dialog" tabindex="-1">
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class="modal shortcuts-modal" onclick={(e) => e.stopPropagation()} onkeydown={() => {}} role="document">
      <div class="shortcuts-header">
        <h2>Pintasan Keyboard</h2>
        <button class="btn-icon" onclick={() => showShortcuts = false}>✕</button>
      </div>
      <div class="shortcuts-grid">
        <div class="shortcut-group">
          <h4>Navigasi</h4>
          <div class="shortcut-item"><span class="shortcut-keys"><kbd>Ctrl</kbd> + <kbd>K</kbd></span><span>Fokus pencarian</span></div>
          <div class="shortcut-item"><span class="shortcut-keys"><kbd>Ctrl</kbd> + <kbd>N</kbd></span><span>Obrolan baru</span></div>
          <div class="shortcut-item"><span class="shortcut-keys"><kbd>Ctrl</kbd> + <kbd>.</kbd></span><span>Toggle riwayat</span></div>
          <div class="shortcut-item"><span class="shortcut-keys"><kbd>Ctrl</kbd> + <kbd>/</kbd></span><span>Pintasan keyboard</span></div>
        </div>
        <div class="shortcut-group">
          <h4>Pencarian</h4>
          <div class="shortcut-item"><span class="shortcut-keys"><kbd>Enter</kbd></span><span>Kirim pertanyaan</span></div>
          <div class="shortcut-item"><span class="shortcut-keys"><kbd>Shift</kbd> + <kbd>Enter</kbd></span><span>Baris baru</span></div>
          <div class="shortcut-item"><span class="shortcut-keys"><kbd>Esc</kbd></span><span>Tutup panel/modal</span></div>
        </div>
      </div>
    </div>
  </div>
{/if}

<!-- Auth Modal -->
{#if showAuth}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <div class="modal-overlay" onclick={() => showAuth = false} onkeydown={(e) => { if (e.key === 'Escape') showAuth = false; }} role="dialog" tabindex="-1">
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div class="modal" onclick={(e) => e.stopPropagation()} onkeydown={() => {}} role="document">
      <h2>{authMode === 'login' ? 'Masuk' : 'Daftar Akun'}</h2>
      
      {#if authError}
        <div class="alert alert-error">{authError}</div>
      {/if}

      <form onsubmit={(e) => { e.preventDefault(); handleAuth(); }}>
        {#if authMode === 'register'}
          <div class="form-group">
            <label for="displayName">Nama Tampilan</label>
            <input id="displayName" type="text" class="input" bind:value={displayName} placeholder="Nama Anda (opsional)" />
          </div>
        {/if}
        <div class="form-group">
          <label for="email">Email</label>
          <input id="email" type="email" class="input" bind:value={email} placeholder="email@example.com" required />
        </div>
        <div class="form-group">
          <label for="password">Password</label>
          <input id="password" type="password" class="input" bind:value={password} placeholder="••••••" required minlength="6" />
        </div>
        <button type="submit" class="btn btn-primary" style="width:100%">
          {authMode === 'login' ? 'Masuk' : 'Daftar'}
        </button>
      </form>

      <p class="auth-toggle">
        {#if authMode === 'login'}
          Belum punya akun? <button class="btn-ghost" onclick={() => authMode = 'register'}>Daftar</button>
        {:else}
          Sudah punya akun? <button class="btn-ghost" onclick={() => authMode = 'login'}>Masuk</button>
        {/if}
      </p>
    </div>
  </div>
{/if}

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="layout" class:split-view={showReaderPanel} bind:this={layoutEl}>
  <!-- Chat Panel (Task 5: controlled max-width) -->
  <div class="chat-panel" class:with-reader={showReaderPanel} style={showReaderPanel ? `flex: 0 0 ${panelRatio * 100}%` : ''}>
    <!-- Header (Task 4: Redesigned) -->
    <header class="header">
      <div class="header-brand">
        <button class="logo-btn" onclick={newChat} title="Obrolan baru">
          <span class="logo-text">بحث المسائل</span>
        </button>
        <div class="header-meta">
          <span class="brand-label">bahtsulmasail.tech</span>
        </div>
      </div>
      <div class="header-actions">
        {#if isAuth}
          <button class="header-action-btn" onclick={() => showShortcuts = true} title="Pintasan (Ctrl+/)">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="4" width="20" height="16" rx="2"/><path d="M6 8h.01M10 8h.01M14 8h.01M18 8h.01M8 12h.01M12 12h.01M16 12h.01M7 16h10"/></svg>
          </button>
          <button class="header-action-btn" onclick={() => { if (!showHistory) loadSessions(); showHistory = !showHistory; }} title="Riwayat (Ctrl+.)">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
          </button>
          <div class="user-badge" title={authUser?.email}>
            <span class="user-avatar">{(authUser?.display_name || authUser?.email || '?')[0].toUpperCase()}</span>
            <span class="user-name">{authUser?.display_name || authUser?.email}</span>
          </div>
          <button class="btn btn-ghost btn-sm" onclick={handleLogout}>Keluar</button>
        {:else}
          <button class="btn btn-primary btn-sm" onclick={() => showAuth = true}>Masuk</button>
        {/if}
      </div>
    </header>

    <!-- Chat History Sidebar (Task 9: with search, Task 10: with projects) -->
    {#if showHistory && isAuth}
      <div class="history-sidebar fade-in">
        <div class="history-header">
          <h3>Riwayat</h3>
          <div class="history-header-actions">
            <button class="btn btn-primary btn-sm" onclick={newChat}>+ Baru</button>
            <button class="btn-icon" onclick={() => showHistory = false} title="Tutup">✕</button>
          </div>
        </div>

        <!-- Search (Task 9) -->
        <div class="history-search">
          <svg class="search-icon-sm" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
          <input type="text" class="input input-sm" bind:value={historySearch} placeholder="Cari riwayat..." />
        </div>

        <!-- Projects (Task 10) -->
        <div class="projects-section">
          <button class="projects-toggle" onclick={() => showProjectMenu = !showProjectMenu}>
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
            Proyek ({projects.length})
            <svg class="chevron" class:rotated={showProjectMenu} width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="6 9 12 15 18 9"/></svg>
          </button>
          {#if showProjectMenu}
            <div class="projects-list">
              {#each projects as project}
                <div class="project-item" class:active={activeProject === project.id}>
                  <button class="project-name" onclick={() => activeProject = activeProject === project.id ? null : project.id}>
                    📁 {project.name} ({project.sessionIds.length})
                  </button>
                  <button class="btn-icon btn-xs" onclick={() => deleteProject(project.id)} title="Hapus">🗑️</button>
                </div>
              {/each}
              <div class="project-add">
                <input type="text" class="input input-sm" bind:value={newProjectName} placeholder="Nama proyek baru..." onkeydown={(e) => { if (e.key === 'Enter') createProject(); }} />
                <button class="btn btn-ghost btn-xs" onclick={createProject}>+</button>
              </div>
            </div>
          {/if}
        </div>

        <!-- Sessions list -->
        <div class="history-list">
          {#each filteredSessions as session}
            <div class="history-item" class:active={sessionId === session.id}>
              {#if editingSessionId === session.id}
                <div class="history-edit" onclick={(e) => e.stopPropagation()} onkeydown={(e) => { if (e.key === 'Enter') handleRenameSession(session.id); if (e.key === 'Escape') editingSessionId = null; }} role="textbox" tabindex="-1">
                  <input type="text" class="input input-sm" bind:value={editingSessionTitle} onblur={() => handleRenameSession(session.id)} />
                  <button class="btn btn-ghost btn-xs" onclick={() => handleRenameSession(session.id)}>✓</button>
                </div>
              {:else}
                <div class="history-content" onclick={() => { loadSession(session.id); showHistory = false; }} onkeydown={(e) => { if (e.key === 'Enter') { loadSession(session.id); showHistory = false; }}} role="button" tabindex="0">
                  <span class="history-title">{session.title || 'Tanpa judul'}</span>
                  <span class="history-meta">{session.message_count} pesan</span>
                </div>
                <div class="history-actions">
                  {#if projects.length > 0}
                    <div class="project-assign">
                      <select class="select-sm" onchange={(e) => { if (e.target.value) addSessionToProject(e.target.value, session.id); e.target.value = ''; }}>
                        <option value="">📁</option>
                        {#each projects as p}
                          <option value={p.id}>{p.name}</option>
                        {/each}
                      </select>
                    </div>
                  {/if}
                  <button class="btn-icon" title="Ubah nama" onclick={(e) => startRenameSession(session.id, session.title, e)}>✏️</button>
                  <button class="btn-icon" title="Hapus" onclick={(e) => handleDeleteSession(session.id, e)}>🗑️</button>
                </div>
              {/if}
            </div>
          {/each}
          {#if filteredSessions.length === 0}
            <p class="empty-state">{historySearch ? 'Tidak ditemukan' : 'Belum ada riwayat pencarian'}</p>
          {/if}
        </div>
      </div>
    {/if}

    <!-- Welcome / Search -->
    {#if results.length === 0 && !answer && !loading}
      <div class="welcome fade-in">
        <div class="welcome-icon">
          <svg width="64" height="64" viewBox="0 0 64 64" fill="none">
            <rect width="64" height="64" rx="16" fill="var(--color-primary)" opacity="0.1"/>
            <text x="32" y="42" text-anchor="middle" font-family="serif" font-size="28" fill="var(--color-primary)">بم</text>
          </svg>
        </div>
        <h2>Bahtsul Masail</h2>
        <p>Cari referensi dari 7.800+ kitab Islam klasik</p>
        <p class="welcome-sub">Tanyakan masalah fikih dalam Bahasa Indonesia, English, atau العربية</p>
        
        <!-- Quick Actions (Task 8) -->
        <div class="quick-actions">
          <p class="qa-label">Coba tanyakan:</p>
          <div class="qa-grid">
            {#each quickActionSuggestions as suggestion}
              <button class="qa-chip" onclick={() => handleQuickAction(suggestion.label)}>
                <span class="qa-icon">{suggestion.icon}</span>
                {suggestion.label}
              </button>
            {/each}
          </div>
        </div>
      </div>
    {/if}

    <!-- Results -->
    {#if results.length > 0 || answer}
      <div class="results-container fade-in">
        <!-- Export buttons (Task 6) -->
        <div class="export-bar">
          <div class="export-bar-left">
            <span class="export-label">💾 Ekspor hasil:</span>
          </div>
          <div class="export-buttons">
            <button class="export-btn" onclick={exportMarkdown} title="Download Markdown">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>
              .md
            </button>
            <button class="export-btn" onclick={exportDocx} title="Download Word Document">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/><line x1="16" y1="17" x2="8" y2="17"/></svg>
              .docx
            </button>
            <button class="export-btn" onclick={exportPdf} title="Export sebagai PDF">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="12" y1="18" x2="12" y2="12"/><polyline points="9 15 12 18 15 15"/></svg>
              .pdf
            </button>
            <button class="export-btn" onclick={exportPlainText} title="Download Plain Text">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14 2 14 8 20 8"/><line x1="16" y1="13" x2="8" y2="13"/></svg>
              .txt
            </button>
          </div>
        </div>

        <!-- AI Answer (Task 2: deduplicated, with clickable refs) -->
        {#if answer || isStreaming}
          <div class="ai-answer card" class:streaming={isStreaming}>
            <div class="ai-header">
              <div class="ai-header-left">
                <span class="ai-icon">
                  <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2a4 4 0 0 1 4 4v2a4 4 0 0 1-8 0V6a4 4 0 0 1 4-4z"/><path d="M16 14a4 4 0 0 0-8 0v4h8v-4z"/><circle cx="9" cy="9" r="1" fill="currentColor"/><circle cx="15" cy="9" r="1" fill="currentColor"/></svg>
                </span>
                <h3>{isStreaming ? 'Menyintesis jawaban...' : 'Jawaban AI'}</h3>
                {#if isStreaming}
                  <span class="streaming-indicator">
                    <span class="dot"></span><span class="dot"></span><span class="dot"></span>
                  </span>
                {/if}
              </div>
              {#if results.length > 0}
                <span class="ai-source-count">Dari {results.length} referensi</span>
              {/if}
            </div>
            <div class="ai-content markdown-body">
              {#if isStreaming && streamingAnswer}
                {@html renderMarkdown(streamingAnswer)}
                <span class="typing-cursor">▊</span>
              {:else if answer}
                {@html renderMarkdown(answer)}
              {:else if isStreaming}
                <div class="ai-thinking">
                  <span class="thinking-text">Menganalisis ibaroh dari kitab-kitab...</span>
                </div>
              {/if}
            </div>
            {#if !isStreaming && results.length > 0}
              <div class="ai-refs">
                <span class="ai-refs-label">Sumber:</span>
                {#each results.slice(0, 5) as result, i}
                  <button class="ai-ref-chip" onclick={() => scrollToResult(i)} title="Lihat referensi #{i+1}">
                    [{i+1}] {result.book_name || `Kitab ${result.book_id}`}, hal. {result.page}
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {/if}

        <!-- Search Results (Task 2: with IDs for scrolling) -->
        {#if results.length > 0}
          {#if translatedTerms.length > 0}
            <div class="query-understood">
              <span class="query-understood-label">🔍</span>
              <span class="query-understood-text">{translatedTerms.slice(0, 5).join(' · ')}</span>
            </div>
          {/if}
          <h3 class="results-title">Hasil Pencarian ({results.length} referensi)</h3>
          <div class="results-list">
            {#each results as result, i}
              <div
                id="result-{i}"
                class="result-card card"
                class:produk-hukum={result.source_type === 'produk_hukum'}
                onclick={() => openBook(result.book_id, result.page, result.book_name, result.content_snippet || result.title || '')}
                onkeydown={(e) => { if (e.key === 'Enter') openBook(result.book_id, result.page, result.book_name, result.content_snippet || result.title || ''); }}
                role="button"
                tabindex="0"
                style="animation-delay: {i * 0.05}s"
              >
                <div class="result-header">
                  <span class="result-num">[{i+1}]</span>
                  <span class="result-score" class:high={result.score >= 70} class:mid={result.score >= 40 && result.score < 70} class:low={result.score < 40} title="Relevansi: {Math.round(result.score)}%">
                    {result.score >= 70 ? '●●●' : result.score >= 40 ? '●●○' : '●○○'}
                  </span>
                  {#if result.source_type === 'produk_hukum'}
                    <span class="source-badge badge-produk">📋 Produk Hukum</span>
                  {:else}
                    <span class="source-badge badge-kitab">📚 Kitab</span>
                  {/if}
                  <h4 class="result-title arabic-text">{result.title}</h4>
                </div>
                
                {#if result.hierarchy && result.hierarchy.length > 0}
                  <div class="result-hierarchy">
                    {#each result.hierarchy as h, j}
                      <span class="hierarchy-item">{h}</span>
                      {#if j < result.hierarchy.length - 1}
                        <span class="hierarchy-sep">›</span>
                      {/if}
                    {/each}
                  </div>
                {/if}

                {#if result.content_snippet}
                  <p class="result-snippet arabic-text">{result.content_snippet}</p>
                {/if}

                <div class="result-meta">
                  <span>📖 {result.book_name || `Kitab ${result.book_id}`}</span>
                  {#if result.author_name}
                    <span>👤 {result.author_name}</span>
                  {/if}
                  {#if result.category}
                    <span>🏷️ {result.category}</span>
                  {/if}
                  <span>📄 Hal. {result.page}</span>
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>
    {/if}

    <!-- Loading -->
    {#if loading}
      <div class="loading-container">
        <div class="loading-spinner"></div>
        <p>Sedang mencari di kitab-kitab...</p>
      </div>
    {/if}

    <!-- Error -->
    {#if errorMsg}
      <div class="alert alert-error fade-in">{errorMsg}</div>
    {/if}

    <!-- Search Input -->
    <div class="search-bar">
      <div class="search-input-wrapper">
        <textarea
          class="search-input"
          bind:value={query}
          placeholder="Tanyakan masalah fikih... (Ctrl+K)"
          onkeydown={handleKeydown}
          rows="1"
          onfocus={() => { if (!query && results.length === 0 && !answer) showQuickActions = true; }}
          onblur={() => setTimeout(() => showQuickActions = false, 200)}
        ></textarea>
        <button
          class="btn btn-primary search-btn"
          onclick={handleQuery}
          disabled={loading || !query.trim()}
        >
          {#if loading}
            <svg class="spin" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12a9 9 0 1 1-6.219-8.56"/></svg>
          {:else}
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="22" y1="2" x2="11" y2="13"/><polygon points="22 2 15 22 11 13 2 9 22 2"/></svg>
          {/if}
        </button>
      </div>
      <div class="search-hint">
        <span>Ctrl+K fokus</span>
        <span>Enter kirim</span>
        <span>Shift+Enter baris baru</span>
      </div>
    </div>
  </div>

  <!-- Resize Handle (Task 1) -->
  {#if showReaderPanel && readerData}
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="resize-handle" onmousedown={startResize} class:active={isResizing}>
      <div class="resize-handle-line"></div>
    </div>
  {/if}

  <!-- Book Reader Panel -->
  {#if showReaderPanel && readerData}
    <div class="reader-panel fade-in">
      <div class="reader-header">
        <button class="btn btn-ghost close-btn" onclick={() => showReader.set(false)}>✕ Tutup</button>
        <button class="btn btn-ghost toc-toggle-btn" onclick={() => showMobileToc = !showMobileToc}>📑 Daftar Isi</button>
        <h3 class="reader-title arabic-text">
          {readerBookName || `Kitab ${readerBookId}`}
          <span class="reader-page-badge">Hal. {readerPage || '1'}</span>
        </h3>
      </div>
      
      <div class="reader-layout">
        {#if showMobileToc}
          <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
          <div class="toc-backdrop" onclick={() => showMobileToc = false} onkeydown={() => {}} role="presentation"></div>
        {/if}
        <div class="reader-toc" class:show-mobile={showMobileToc}>
          <h4>Daftar Isi</h4>
          <div class="toc-tree">
            {#each readerData.toc as node}
              <div class="toc-node">
                <button class="toc-item" class:active={readerPage === node.page} onclick={() => navigatePage(node.page)}>
                  {node.content}
                </button>
                {#if node.children && node.children.length > 0}
                  <div class="toc-children">
                    {#each node.children as child}
                      <button class="toc-item toc-child" class:active={readerPage === child.page} onclick={() => navigatePage(child.page)}>
                        {child.content}
                      </button>
                      {#if child.children && child.children.length > 0}
                        <div class="toc-children">
                          {#each child.children as grandchild}
                            <button class="toc-item toc-grandchild" class:active={readerPage === grandchild.page} onclick={() => navigatePage(grandchild.page)}>
                              {grandchild.content}
                            </button>
                          {/each}
                        </div>
                      {/if}
                    {/each}
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        </div>

        <div class="reader-content">
          {#if readerLoading}
            <div class="loading-container">
              <div class="loading-spinner"></div>
            </div>
          {:else}
            {#each readerData.pages as page}
              <div class="page-content arabic-text">
                {@html highlightContent(page.content.replace(/\^M/g, '<br/>'))}
              </div>
            {/each}
            
            <div class="page-nav">
              <button class="btn btn-secondary" onclick={() => navigatePage(String(Math.max(1, parseInt(readerPage || '1') - 1)))} disabled={readerPage === '1'}>
                ← Sebelumnya
              </button>
              <span class="page-info">Halaman {readerPage || '1'} dari {readerData.total_pages}</span>
              <button class="btn btn-secondary" onclick={() => navigatePage(String(parseInt(readerPage || '1') + 1))}>
                Berikutnya →
              </button>
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  /* ─── Layout ─── */
  .layout {
    display: flex;
    min-height: 100vh;
    transition: none;
  }

  .chat-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    max-width: 780px;
    margin: 0 auto;
    width: 100%;
    padding: 0 24px;
    transition: none;
  }

  .chat-panel.with-reader {
    max-width: 100%;
    margin: 0;
    min-width: 320px;
    border-right: none;
  }

  .split-view {
    gap: 0;
  }

  /* ─── Resize Handle (Task 1) ─── */
  .resize-handle {
    width: 6px;
    cursor: col-resize;
    background: var(--color-border);
    position: relative;
    flex-shrink: 0;
    transition: background 0.15s;
    z-index: 10;
  }

  .resize-handle:hover,
  .resize-handle.active {
    background: var(--color-primary);
  }

  .resize-handle-line {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    width: 2px;
    height: 32px;
    background: var(--color-text-muted);
    border-radius: 2px;
    opacity: 0.5;
  }

  .resize-handle:hover .resize-handle-line,
  .resize-handle.active .resize-handle-line {
    opacity: 1;
    background: white;
  }

  /* ─── Header (Task 4: Redesigned) ─── */
  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 0;
    border-bottom: 1px solid var(--color-border);
    gap: 12px;
  }

  .header-brand {
    display: flex;
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .logo-btn {
    background: none;
    border: none;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 8px;
    transition: background 0.15s;
  }

  .logo-btn:hover {
    background: var(--color-bg-alt);
  }

  .logo-text {
    font-family: var(--font-arabic);
    font-size: 1.5rem;
    color: var(--color-primary);
    font-weight: 700;
    white-space: nowrap;
  }

  .header-meta {
    display: flex;
    flex-direction: column;
    gap: 0;
  }

  .brand-label {
    color: var(--color-text-muted);
    font-size: 0.75rem;
    letter-spacing: 0.5px;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }

  .header-action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border: none;
    background: none;
    cursor: pointer;
    border-radius: 8px;
    color: var(--color-text-light);
    transition: all 0.15s;
  }

  .header-action-btn:hover {
    background: var(--color-bg-alt);
    color: var(--color-primary);
  }

  .user-badge {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 10px 4px 4px;
    border-radius: 20px;
    background: var(--color-bg-alt);
  }

  .user-avatar {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: var(--color-primary);
    color: white;
    font-size: 0.75rem;
    font-weight: 700;
  }

  .user-name {
    font-size: 0.8rem;
    color: var(--color-text-light);
    max-width: 120px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* ─── Welcome (Task 4, 8: with quick actions) ─── */
  .welcome {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    padding: 48px 20px 32px;
    gap: 8px;
  }

  .welcome-icon {
    margin-bottom: 12px;
  }

  .welcome h2 {
    font-family: var(--font-ui);
    font-size: 1.6rem;
    color: var(--color-primary);
    margin-bottom: 4px;
  }

  .welcome p {
    color: var(--color-text-light);
    font-size: 1rem;
  }

  .welcome-sub {
    font-size: 0.9rem !important;
    color: var(--color-text-muted) !important;
  }

  /* ─── Quick Actions (Task 8) ─── */
  .quick-actions {
    margin-top: 24px;
    width: 100%;
    max-width: 600px;
  }

  .qa-label {
    font-size: 0.8rem;
    color: var(--color-text-muted);
    margin-bottom: 12px;
  }

  .qa-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
    gap: 8px;
  }

  .qa-chip {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 10px;
    cursor: pointer;
    font-size: 0.85rem;
    color: var(--color-text);
    text-align: left;
    transition: all 0.15s;
    font-family: var(--font-ui);
  }

  .qa-chip:hover {
    border-color: var(--color-primary);
    background: #f0f9f4;
    transform: translateY(-1px);
    box-shadow: 0 2px 8px rgba(0,0,0,0.06);
  }

  .qa-icon {
    font-size: 1.1rem;
    flex-shrink: 0;
  }

  /* ─── Search Bar ─── */
  .search-bar {
    position: sticky;
    bottom: 0;
    padding: 12px 0 16px;
    background: linear-gradient(transparent, var(--color-bg) 20%);
  }

  .search-input-wrapper {
    display: flex;
    gap: 10px;
    align-items: flex-end;
    background: var(--color-surface);
    border: 2px solid var(--color-border);
    border-radius: var(--radius);
    padding: 8px 12px;
    box-shadow: var(--shadow-lg);
    transition: border-color 0.15s, box-shadow 0.15s;
  }

  .search-input-wrapper:focus-within {
    border-color: var(--color-primary);
    box-shadow: 0 0 0 3px rgba(26, 95, 58, 0.15), var(--shadow-lg);
  }

  .search-input {
    flex: 1;
    border: none;
    outline: none;
    font-family: var(--font-ui);
    font-size: 0.95rem;
    resize: none;
    line-height: 1.5;
    background: transparent;
    direction: ltr;
    min-height: 24px;
    max-height: 120px;
  }

  .search-btn {
    padding: 8px 14px;
    border-radius: var(--radius-sm);
    font-size: 1rem;
    min-width: 44px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .search-hint {
    display: flex;
    gap: 12px;
    justify-content: center;
    margin-top: 6px;
    font-size: 0.7rem;
    color: var(--color-text-muted);
    opacity: 0.6;
  }

  .search-hint span {
    display: flex;
    align-items: center;
    gap: 4px;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .spin {
    animation: spin 0.8s linear infinite;
  }

  /* ─── Results ─── */
  .results-container {
    flex: 1;
    padding: 20px 0;
    overflow-y: auto;
  }

  .results-title {
    margin-bottom: 12px;
    color: var(--color-text-light);
    font-size: 0.95rem;
  }

  .results-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .result-card {
    cursor: pointer;
    border-left: 4px solid var(--color-primary);
    animation: fadeIn 0.3s ease-out both;
    transition: all 0.2s;
  }

  .result-card:hover {
    border-left-color: var(--color-secondary);
    transform: translateX(3px);
  }

  :global(.result-card.result-highlighted) {
    border-left-color: var(--color-secondary);
    box-shadow: 0 0 0 2px var(--color-secondary), var(--shadow-lg);
    animation: highlightPulse 2s ease-out;
  }

  .result-header {
    display: flex;
    align-items: flex-start;
    gap: 8px;
    margin-bottom: 6px;
    flex-wrap: wrap;
  }

  .result-num {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 28px;
    height: 22px;
    border-radius: 4px;
    font-size: 0.75rem;
    font-weight: 700;
    background: var(--color-bg-alt);
    color: var(--color-text-muted);
    flex-shrink: 0;
  }

  .result-score {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 32px;
    height: 22px;
    border-radius: 4px;
    font-weight: 700;
    font-size: 0.85rem;
    letter-spacing: 1px;
    flex-shrink: 0;
    cursor: default;
  }

  .result-score.high { color: #2a7d4f; }
  .result-score.mid { color: #8a6500; }
  .result-score.low { color: #999; }

  /* ─── Source Type Badges ─── */
  .source-badge {
    display: inline-flex;
    align-items: center;
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 0.7rem;
    font-weight: 600;
    flex-shrink: 0;
    white-space: nowrap;
  }

  .badge-kitab { background: #e8f5e9; color: #2e7d32; }
  .badge-produk { background: #e3f2fd; color: #1565c0; }
  .result-card.produk-hukum { border-left-color: #1565c0; }

  .result-title {
    font-size: 1.05rem;
    line-height: 1.8;
  }

  .result-hierarchy {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-bottom: 6px;
    font-size: 0.78rem;
    color: var(--color-text-muted);
  }

  .hierarchy-item {
    background: var(--color-bg-alt);
    padding: 1px 8px;
    border-radius: 4px;
  }

  .hierarchy-sep { color: var(--color-border); }

  .result-snippet {
    color: var(--color-text-light);
    font-size: 0.93rem;
    line-height: 1.8;
    margin-bottom: 6px;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .result-meta {
    display: flex;
    gap: 12px;
    font-size: 0.78rem;
    color: var(--color-text-muted);
    flex-wrap: wrap;
  }

  /* ─── Query Understood Chip ─── */
  .query-understood {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    margin-bottom: 8px;
    padding: 4px 10px;
    background: var(--color-bg-alt);
    border-radius: 20px;
    font-size: 0.8rem;
    color: var(--color-text-muted);
    direction: ltr;
  }

  .query-understood-label { font-size: 0.9rem; }
  .query-understood-text {
    font-family: 'Amiri', 'Scheherazade New', serif;
    direction: rtl;
    unicode-bidi: embed;
  }

  /* ─── AI Answer (Task 2: deduplicated with ref chips) ─── */

  /* ─── Export Bar (Task 6) ─── */
  .export-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 14px;
    padding: 8px 14px;
    background: var(--color-bg-alt);
    border-radius: var(--radius-sm);
    flex-wrap: wrap;
  }

  .export-label {
    font-size: 0.8rem;
    color: var(--color-text-muted);
    font-weight: 500;
  }

  .export-buttons {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .export-btn {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    font-size: 0.75rem;
    font-family: var(--font-ui);
    color: var(--color-text-light);
    cursor: pointer;
    transition: all 0.15s;
  }

  .export-btn:hover {
    background: var(--color-primary);
    color: white;
    border-color: var(--color-primary);
  }

  .ai-answer {
    margin-bottom: 20px;
    border-left: 4px solid var(--color-secondary);
    background: linear-gradient(135deg, #fffdf5, #fff);
  }

  .ai-answer.streaming {
    border-left-color: var(--color-primary);
    animation: streamPulse 2s ease-in-out infinite;
  }

  @keyframes streamPulse {
    0%, 100% { box-shadow: 0 0 0 0 rgba(var(--color-primary-rgb, 30, 86, 49), 0); }
    50% { box-shadow: 0 0 12px 2px rgba(30, 86, 49, 0.08); }
  }

  .streaming-indicator {
    display: inline-flex;
    gap: 3px;
    margin-left: 6px;
  }

  .streaming-indicator .dot {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--color-primary);
    animation: dotBounce 1.2s ease-in-out infinite;
  }

  .streaming-indicator .dot:nth-child(2) { animation-delay: 0.2s; }
  .streaming-indicator .dot:nth-child(3) { animation-delay: 0.4s; }

  @keyframes dotBounce {
    0%, 80%, 100% { opacity: 0.3; transform: scale(0.8); }
    40% { opacity: 1; transform: scale(1.2); }
  }

  .typing-cursor {
    display: inline;
    animation: cursorBlink 0.8s step-end infinite;
    color: var(--color-primary);
    font-weight: bold;
    margin-left: 1px;
  }

  @keyframes cursorBlink {
    0%, 100% { opacity: 1; }
    50% { opacity: 0; }
  }

  .ai-thinking {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 0;
  }

  .thinking-text {
    color: var(--color-text-muted);
    font-style: italic;
    font-size: 0.9rem;
    animation: fadeInOut 2s ease-in-out infinite;
  }

  @keyframes fadeInOut {
    0%, 100% { opacity: 0.5; }
    50% { opacity: 1; }
  }

  .ai-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 12px;
  }

  .ai-header-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .ai-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: 8px;
    background: var(--color-primary);
    color: white;
  }

  .ai-icon svg { width: 16px; height: 16px; }

  .ai-header h3 {
    color: var(--color-primary);
    font-size: 1rem;
  }

  .ai-source-count {
    font-size: 0.75rem;
    color: var(--color-text-muted);
    background: var(--color-bg-alt);
    padding: 2px 10px;
    border-radius: 12px;
  }

  .ai-content { line-height: 2; }

  .ai-refs {
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid var(--color-border);
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    align-items: center;
  }

  .ai-refs-label {
    font-size: 0.78rem;
    color: var(--color-text-muted);
    font-weight: 600;
  }

  .ai-ref-chip {
    display: inline-flex;
    align-items: center;
    padding: 3px 10px;
    background: var(--color-bg-alt);
    border: 1px solid var(--color-border);
    border-radius: 6px;
    font-size: 0.72rem;
    color: var(--color-primary);
    cursor: pointer;
    transition: all 0.15s;
    font-family: var(--font-ui);
  }

  .ai-ref-chip:hover {
    background: var(--color-primary);
    color: white;
    border-color: var(--color-primary);
  }

  /* ─── Markdown Rendering ─── */
  .markdown-body {
    font-family: var(--font-ui);
    color: var(--color-text);
  }

  .markdown-body :global(h2) {
    font-size: 1.2rem;
    color: var(--color-primary);
    margin: 0 0 10px 0;
    padding-bottom: 6px;
    border-bottom: 2px solid var(--color-bg-alt);
    font-family: var(--font-ui);
  }

  .markdown-body :global(h3) {
    font-size: 1.05rem;
    color: var(--color-primary-dark, var(--color-primary));
    margin: 16px 0 6px 0;
    font-family: var(--font-arabic);
    line-height: 1.8;
  }

  .markdown-body :global(p) {
    margin: 0 0 10px 0;
    line-height: 1.9;
  }

  .markdown-body :global(strong) {
    color: var(--color-primary-dark, var(--color-primary));
  }

  .markdown-body :global(blockquote) {
    margin: 6px 0 14px 0;
    padding: 10px 14px;
    border-left: 3px solid var(--color-secondary);
    background: var(--color-bg-alt);
    border-radius: var(--radius-sm) 0 0 var(--radius-sm);
    font-family: var(--font-arabic);
    font-size: 1rem;
    line-height: 2.2;
    color: var(--color-text-light);
  }

  .markdown-body :global(blockquote p) { margin: 0; }

  .markdown-body :global(hr) {
    border: none;
    border-top: 1px solid var(--color-border);
    margin: 14px 0;
  }

  .markdown-body :global(ul), .markdown-body :global(ol) {
    padding-left: 20px;
    margin-bottom: 10px;
  }

  .markdown-body :global(li) {
    margin-bottom: 4px;
    line-height: 1.8;
  }

  .markdown-body :global(em) {
    color: var(--color-text-muted);
  }

  .markdown-body :global(.ai-ref) {
    display: inline;
    background: #f0f9f4;
    padding: 2px 6px;
    border-radius: 4px;
    color: var(--color-primary);
    cursor: pointer;
    font-size: 0.85rem;
    transition: background 0.15s;
  }

  .markdown-body :global(.ai-ref:hover) {
    background: #d4edda;
  }

  /* ─── Loading ─── */
  .loading-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 40px;
    gap: 16px;
  }

  .loading-spinner {
    width: 36px;
    height: 36px;
    border: 3px solid var(--color-border);
    border-top-color: var(--color-primary);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  /* ─── Modal ─── */
  .modal-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(4px);
  }

  .modal {
    background: var(--color-surface);
    border-radius: var(--radius);
    padding: 28px;
    width: 100%;
    max-width: 420px;
    box-shadow: var(--shadow-lg);
    animation: fadeIn 0.2s ease-out;
  }

  .modal h2 {
    text-align: center;
    margin-bottom: 20px;
    color: var(--color-primary);
    font-family: var(--font-ui);
  }

  .form-group {
    margin-bottom: 14px;
  }

  .form-group label {
    display: block;
    margin-bottom: 4px;
    font-weight: 500;
    font-size: 0.88rem;
  }

  .auth-toggle {
    text-align: center;
    margin-top: 14px;
    font-size: 0.88rem;
    color: var(--color-text-light);
  }

  /* ─── Keyboard Shortcuts Modal (Task 13) ─── */
  .shortcuts-modal {
    max-width: 520px;
  }

  .shortcuts-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 20px;
  }

  .shortcuts-header h2 {
    margin: 0;
    text-align: left;
  }

  .shortcuts-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 24px;
  }

  .shortcut-group h4 {
    font-size: 0.85rem;
    color: var(--color-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 10px;
  }

  .shortcut-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 6px 0;
    font-size: 0.85rem;
  }

  .shortcut-keys {
    display: flex;
    gap: 4px;
    align-items: center;
  }

  .shortcut-keys kbd {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 2px 6px;
    background: var(--color-bg-alt);
    border: 1px solid var(--color-border);
    border-radius: 4px;
    font-size: 0.75rem;
    font-family: var(--font-ui);
    min-width: 24px;
    box-shadow: 0 1px 0 var(--color-border);
  }

  /* ─── Alerts ─── */
  .alert {
    padding: 10px 14px;
    border-radius: var(--radius-sm);
    margin: 10px 0;
  }

  .alert-error {
    background: #f8d7da;
    color: #721c24;
    border: 1px solid #f5c6cb;
  }

  .empty-state {
    text-align: center;
    padding: 20px;
    color: var(--color-text-muted);
    font-size: 0.85rem;
  }

  /* ─── History Sidebar (Task 9: search, Task 10: projects) ─── */
  .history-sidebar {
    position: fixed;
    top: 0;
    right: 0;
    width: 320px;
    height: 100vh;
    background: var(--color-surface);
    box-shadow: var(--shadow-lg);
    z-index: 100;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .history-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 16px 12px;
    border-bottom: 1px solid var(--color-border);
  }

  .history-header h3 { font-size: 1rem; }

  .history-header-actions {
    display: flex;
    gap: 6px;
    align-items: center;
  }

  .btn-sm {
    padding: 5px 10px;
    font-size: 0.78rem;
  }

  .history-search {
    padding: 8px 16px;
    position: relative;
    border-bottom: 1px solid var(--color-border);
  }

  .history-search .input {
    padding-left: 30px;
    font-size: 0.82rem;
    padding-top: 6px;
    padding-bottom: 6px;
  }

  .search-icon-sm {
    position: absolute;
    left: 24px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--color-text-muted);
  }

  /* ─── Projects (Task 10) ─── */
  .projects-section {
    border-bottom: 1px solid var(--color-border);
  }

  .projects-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 8px 16px;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 0.82rem;
    color: var(--color-text-light);
    font-family: var(--font-ui);
    transition: background 0.15s;
  }

  .projects-toggle:hover {
    background: var(--color-bg-alt);
  }

  .chevron {
    margin-left: auto;
    transition: transform 0.2s;
  }

  .chevron.rotated {
    transform: rotate(180deg);
  }

  .projects-list {
    padding: 4px 16px 8px;
  }

  .project-item {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-radius: 6px;
    transition: background 0.15s;
  }

  .project-item:hover {
    background: var(--color-bg-alt);
  }

  .project-item.active {
    background: var(--color-bg-alt);
  }

  .project-name {
    flex: 1;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 0.8rem;
    text-align: left;
    color: var(--color-text);
    font-family: var(--font-ui);
    padding: 2px 0;
  }

  .project-add {
    display: flex;
    gap: 4px;
    margin-top: 4px;
    align-items: center;
  }

  .project-add .input {
    font-size: 0.78rem;
    padding: 4px 8px;
  }

  .select-sm {
    font-size: 0.7rem;
    padding: 1px 4px;
    border: 1px solid var(--color-border);
    border-radius: 4px;
    background: var(--color-surface);
    cursor: pointer;
    width: 28px;
  }

  .project-assign {
    flex-shrink: 0;
  }

  /* ─── History List ─── */
  .history-list {
    flex: 1;
    overflow-y: auto;
    padding: 8px 12px;
  }

  .history-item {
    padding: 10px;
    border-radius: var(--radius-sm);
    cursor: pointer;
    margin-bottom: 2px;
    transition: background 0.15s;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .history-item:hover { background: var(--color-bg-alt); }

  .history-item.active {
    background: var(--color-bg-alt);
    border-left: 3px solid var(--color-primary);
  }

  .history-content {
    flex: 1;
    min-width: 0;
  }

  .history-actions {
    display: flex;
    gap: 2px;
    opacity: 0;
    transition: opacity 0.15s;
    flex-shrink: 0;
    align-items: center;
  }

  .history-item:hover .history-actions { opacity: 1; }

  .btn-icon {
    background: none;
    border: none;
    cursor: pointer;
    padding: 3px 5px;
    border-radius: 4px;
    font-size: 0.75rem;
    transition: background 0.15s;
  }

  .btn-icon:hover { background: var(--color-border); }

  .btn-xs {
    padding: 2px 6px;
    font-size: 0.72rem;
  }

  .history-edit {
    display: flex;
    gap: 4px;
    align-items: center;
    width: 100%;
  }

  .history-edit .input {
    flex: 1;
    padding: 3px 6px;
    font-size: 0.82rem;
  }

  .history-title {
    display: block;
    font-weight: 500;
    margin-bottom: 2px;
    font-size: 0.85rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .history-meta {
    font-size: 0.75rem;
    color: var(--color-text-muted);
  }

  /* ─── Reader Panel ─── */
  .reader-panel {
    flex: 2;
    display: flex;
    flex-direction: column;
    height: 100vh;
    position: sticky;
    top: 0;
    background: var(--color-surface);
    min-width: 0;
  }

  .reader-header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--color-border);
    background: var(--color-bg-alt);
  }

  .close-btn { flex-shrink: 0; }

  .reader-title {
    flex: 1;
    font-size: 1rem;
    line-height: 1.6;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .reader-page-badge {
    display: inline-block;
    font-size: 0.72rem;
    font-family: var(--font-ui);
    background: var(--color-primary);
    color: white;
    padding: 2px 8px;
    border-radius: 12px;
    margin-left: 8px;
    vertical-align: middle;
  }

  .reader-layout {
    display: flex;
    flex: 1;
    overflow: hidden;
    position: relative;
  }

  .toc-backdrop { display: none; }

  .reader-toc {
    width: 240px;
    border-right: 1px solid var(--color-border);
    overflow-y: auto;
    padding: 14px;
    background: var(--color-bg);
    flex-shrink: 0;
  }

  .reader-toc h4 {
    margin-bottom: 10px;
    color: var(--color-primary);
    font-family: var(--font-ui);
    font-size: 0.9rem;
  }

  .toc-tree {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .toc-item {
    display: block;
    width: 100%;
    text-align: right;
    padding: 5px 8px;
    border: none;
    background: none;
    cursor: pointer;
    font-family: var(--font-arabic);
    font-size: 0.85rem;
    color: var(--color-text);
    border-radius: 4px;
    transition: background 0.15s;
    line-height: 1.6;
  }

  .toc-item:hover {
    background: var(--color-bg-alt);
    color: var(--color-primary);
  }

  .toc-item.active {
    background: var(--color-primary);
    color: white;
  }

  .toc-children { padding-right: 14px; }
  .toc-child { font-size: 0.82rem; color: var(--color-text-light); }
  .toc-grandchild { font-size: 0.78rem; color: var(--color-text-muted); }

  .reader-content {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
  }

  .page-content {
    line-height: 2.2;
    font-size: 1.15rem;
  }

  .page-content :global(span[data-type="title"]) {
    display: block;
    font-size: 1.3rem;
    font-weight: 700;
    color: var(--color-primary);
    margin: 14px 0 6px;
  }

  .page-content :global(.search-highlight) {
    background: var(--color-highlight);
    padding: 2px 4px;
    border-radius: 4px;
    animation: highlightPulse 2.5s ease-out;
    box-shadow: 0 0 0 2px rgba(255, 193, 7, 0.3);
  }

  @keyframes highlightPulse {
    0% { background: #ffc107; box-shadow: 0 0 12px rgba(255, 193, 7, 0.6); }
    40% { background: #fff3cd; box-shadow: 0 0 6px rgba(255, 193, 7, 0.3); }
    100% { background: var(--color-highlight); box-shadow: 0 0 0 2px rgba(255, 193, 7, 0.3); }
  }

  .page-nav {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 0;
    margin-top: 20px;
    border-top: 1px solid var(--color-border);
  }

  .page-info {
    color: var(--color-text-muted);
    font-size: 0.85rem;
  }

  .toc-toggle-btn { display: none; }

  /* ─── Responsive ─── */
  @media (max-width: 768px) {
    .chat-panel {
      padding: 0 12px;
    }

    .chat-panel.with-reader {
      min-width: unset;
    }

    .header {
      padding: 8px 0;
    }

    .brand-label { display: none; }
    .user-name { display: none; }

    .logo-text { font-size: 1.2rem; }

    .header-actions { gap: 4px; }
    .header-actions .btn { padding: 5px 8px; font-size: 0.78rem; }

    .welcome { padding: 32px 12px 24px; }
    .welcome h2 { font-size: 1.25rem; }
    .welcome p { font-size: 0.9rem; }

    .qa-grid { grid-template-columns: 1fr; }

    .search-bar { padding: 8px 0 12px; }
    .search-input-wrapper { padding: 6px 8px; }
    .search-input { font-size: 0.88rem; }
    .search-btn { padding: 6px 10px; min-width: 36px; }
    .search-hint { display: none; }

    .results-container { padding: 12px 0; }
    .result-card { border-left-width: 3px; }
    .result-title { font-size: 0.95rem; }
    .result-meta { gap: 8px; font-size: 0.72rem; }

    .ai-answer { border-left-width: 3px; }

    .resize-handle { display: none; }

    .reader-panel {
      position: fixed;
      inset: 0;
      z-index: 200;
      width: 100%;
    }

    .reader-header {
      flex-wrap: wrap;
      gap: 6px;
      padding: 8px 10px;
    }

    .toc-toggle-btn {
      display: inline-flex;
      font-size: 0.78rem;
      padding: 4px 8px;
    }

    .reader-title {
      width: 100%;
      order: 3;
      font-size: 0.9rem;
      white-space: normal;
    }

    .reader-toc {
      display: none;
      position: fixed;
      top: 0;
      left: 0;
      width: 85%;
      max-width: 300px;
      height: 100vh;
      z-index: 250;
      box-shadow: 2px 0 16px rgba(0,0,0,0.2);
      border-right: none;
    }

    .reader-toc.show-mobile { display: block; }

    .toc-backdrop {
      display: block;
      position: fixed;
      inset: 0;
      background: rgba(0, 0, 0, 0.4);
      z-index: 240;
    }

    .reader-content { padding: 14px 10px; }
    .page-content { font-size: 1rem; line-height: 2; }

    .page-nav {
      flex-direction: column;
      gap: 10px;
      text-align: center;
    }

    .page-nav .btn { width: 100%; justify-content: center; }

    .history-sidebar { width: 100%; }

    .shortcuts-grid { grid-template-columns: 1fr; gap: 16px; }

    .modal { margin: 16px; padding: 20px; }
  }

  @media (max-width: 360px) {
    .chat-panel { padding: 0 8px; }
    .welcome h2 { font-size: 1.1rem; }
    .result-title { font-size: 0.9rem; }
  }
</style>
