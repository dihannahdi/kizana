<script>
  import { auth, searchResults, aiAnswer, isLoading, error, showReader, bookData, currentSession } from '$lib/stores.js';
  import { sendQuery, sendQueryStream, readBook, getProdukHukumDetail, login as apiLogin, register as apiRegister, getSessions, getSession, deleteSession, renameSession } from '$lib/api.js';
  import { onMount, onDestroy, tick } from 'svelte';
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

  // Produk Hukum viewer state
  let produkHukumData = $state(null);
  let produkHukumLoading = $state(false);
  let showProdukHukumViewer = $state(false);

  // Reader panel closing animation
  let readerClosing = $state(false);

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

  // ─── Chat Thread (Multi-Turn) ───
  let chatMessages = $state([]); // Array of { role, content, results, query, mode, confidence, translatedTerms, detectedDomain, detectedLanguage }
  let chatContainerEl = $state(null);

  async function scrollToBottom() {
    await tick();
    if (chatContainerEl) {
      chatContainerEl.scrollTo({ top: chatContainerEl.scrollHeight, behavior: 'smooth' });
    }
  }

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

  // Result filters
  let filterSource = $state('all'); // 'all' | 'kitab' | 'produk_hukum'
  let filterMinScore = $state(0); // 0 | 40 | 70

  function filterResults(resultsList) {
    if (!resultsList) return [];
    return resultsList.filter(r => {
      if (filterSource !== 'all' && r.source_type !== filterSource) return false;
      if (r.score < filterMinScore) return false;
      return true;
    });
  }

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

  // ─── Multimodal Response System ───
  // Response modes: ringkas (Q&A), ibaroh (citations), lengkap (report), bahtsul-masail (formal BM)
  let responseMode = $state('auto');
  let detectedMode = $state('ringkas');
  let activeMode = $derived(responseMode === 'auto' ? detectedMode : responseMode);
  let showFollowUps = $state(false);
  let copyToast = $state('');

  const responseModes = [
    { id: 'auto', label: 'Auto', icon: '✨', desc: 'Otomatis sesuai pertanyaan' },
    { id: 'ringkas', label: 'Ringkas', icon: '💬', desc: 'Jawaban singkat & langsung' },
    { id: 'ibaroh', label: 'Ibaroh', icon: '📜', desc: 'Kutipan teks Arab asli' },
    { id: 'lengkap', label: 'Lengkap', icon: '📋', desc: 'Analisis mendalam & terstruktur' },
    { id: 'bahtsul-masail', label: 'Bahtsul Masail', icon: '⚖️', desc: 'Format BM resmi' },
  ];

  // Intent detection from query text
  function detectQueryMode(q) {
    if (!q) return 'ringkas';
    const lower = q.toLowerCase();
    // Ibaroh mode
    if (/\b(ibar[oh]+|ibarat|nash|dalil|عبارة|نص|kutipan|rujukan|teks arab|arabic text)\b/i.test(q)) {
      return 'ibaroh';
    }
    // Bahtsul Masail mode
    if (/\b(bahtsul\s*masail|format\s*bm|rumusan\s*masalah|deskripsi\s*masalah)\b/i.test(q)) {
      return 'bahtsul-masail';
    }
    // Lengkap/Report mode
    if (/\b(analis[ia]s|bandingkan|perbandingan|jelaskan\s+secara|lengkap|pandangan\s+(empat|4)\s*madz?hab|kompar|komprehensif|detail|systematic)\b/i.test(q)) {
      return 'lengkap';
    }
    // Default: ringkas
    return 'ringkas';
  }

  // Follow-up suggestions based on current query and results
  let followUpSuggestions = $derived.by(() => {
    if (!results.length || !query) return [];
    const suggestions = [];
    const q = query.toLowerCase();

    // If not in ibaroh mode, suggest it
    if (activeMode !== 'ibaroh') {
      suggestions.push({ label: 'Tampilkan ibaroh terkait', icon: '📜', query: `ibaroh ${query}` });
    }
    // If not in lengkap mode, suggest deeper analysis
    if (activeMode !== 'lengkap') {
      suggestions.push({ label: 'Analisis lebih mendalam', icon: '📋', query: `jelaskan secara lengkap ${query}` });
    }
    // Mazhab-specific suggestions
    if (!/syafi.?i|شافعي/i.test(q)) {
      suggestions.push({ label: 'Menurut mazhab Syafi\'i', icon: '🏛️', query: `${query} menurut mazhab Syafi'i` });
    }
    if (!/hanafi|حنفي/i.test(q)) {
      suggestions.push({ label: 'Menurut mazhab Hanafi', icon: '🏛️', query: `${query} menurut mazhab Hanafi` });
    }
    // Bahtsul Masail format
    if (activeMode !== 'bahtsul-masail') {
      suggestions.push({ label: 'Format Bahtsul Masail', icon: '⚖️', query: `bahtsul masail: ${query}` });
    }
    return suggestions.slice(0, 4);
  });

  // Detect confidence tier from AI answer content 
  function detectConfidenceTier(answerText, resultsList) {
    if (!answerText || !resultsList.length) return null;
    const highScoreResults = resultsList.filter(r => r.score >= 70).length;
    const hasDirectIbaroh = /[«»「」]|📖|عبارة|نص|قال/.test(answerText);
    const hasDisclaimer = /لم (أجد|نجد)|tidak ditemukan|not found|لا يوجد/i.test(answerText);
    
    if (hasDisclaimer || resultsList.length === 0) {
      return { tier: 'ghaib', label: 'Tidak Cukup Referensi', icon: '🔴', desc: 'Tidak ditemukan referensi yang memadai' };
    }
    if (highScoreResults >= 3 && hasDirectIbaroh) {
      return { tier: 'qathi', label: 'Referensi Kuat', icon: '🟢', desc: 'Ditemukan langsung di kitab dengan tingkat relevansi tinggi' };
    }
    return { tier: 'zhanni', label: 'Berdasarkan Prinsip Umum', icon: '🟡', desc: 'Berdasarkan referensi yang relevan secara umum' };
  }

  // Copy text to clipboard with toast feedback
  async function copyToClipboard(text, label = 'Teks') {
    try {
      await navigator.clipboard.writeText(text);
      copyToast = `${label} disalin!`;
      setTimeout(() => copyToast = '', 2000);
    } catch {
      copyToast = 'Gagal menyalin';
      setTimeout(() => copyToast = '', 2000);
    }
  }

  // Parse structured AI answer into sections
  function parseAnswerSections(text) {
    if (!text) return [];
    const sections = [];
    // Split by ## headings
    const parts = text.split(/(?=^## )/m);
    for (const part of parts) {
      const headerMatch = part.match(/^## (.+?)$/m);
      if (headerMatch) {
        const title = headerMatch[1].trim();
        const content = part.replace(/^## .+$/m, '').trim();
        // Detect section type
        let type = 'general';
        if (/jawaban|answer|الجواب|✅/.test(title)) type = 'jawaban';
        else if (/ibar[oh]+|dalil|evidence|عبارات|📖/.test(title)) type = 'ibaroh';
        else if (/khilaf|perbedaan|differences|خلاف|⚖️/.test(title)) type = 'khilaf';
        else if (/kesimpulan|conclusion|خلاصة|📝/.test(title)) type = 'kesimpulan';
        sections.push({ title, content, type });
      } else if (part.trim()) {
        sections.push({ title: '', content: part.trim(), type: 'intro' });
      }
    }
    return sections;
  }

  // Extract ibaroh blocks from text (Arabic quotes with sources)
  function extractIbarohBlocks(text) {
    if (!text) return [];
    const blocks = [];
    // Match blockquote patterns with source lines
    const regex = /(?:^>\s*.+$(?:\n^>\s*.+$)*)/gm;
    const matches = text.match(regex);
    if (matches) {
      for (const m of matches) {
        const clean = m.replace(/^>\s*/gm, '').trim();
        // Try to find source reference nearby
        blocks.push({ text: clean, source: '' });
      }
    }
    return blocks;
  }

  // Subscribe to stores
  auth.subscribe(v => { isAuth = v.isAuthenticated; authUser = v.user; });
  searchResults.subscribe(v => results = v);
  aiAnswer.subscribe(v => answer = v);
  isLoading.subscribe(v => loading = v);
  error.subscribe(v => errorMsg = v);
  showReader.subscribe(v => showReaderPanel = v);

  function renderMarkdown(text) {
    if (!text) return '';
    // Make [N] references clickable — links to search result N
    let processed = text.replace(
      /\[(\d{1,2})\]/g,
      '<button class="ai-inline-ref" data-ref-index="$1" title="Buka referensi [$1]">[$1]</button>'
    );
    // Make [Kitab X, hal. Y] references clickable
    processed = processed.replace(
      /\[([^\]]*?(?:Kitab|kitab|كتاب)[^\]]*?)\]/g,
      '<span class="ai-ref" title="Klik untuk detail">📖 $1</span>'
    );
    const html = marked.parse(processed);
    return DOMPurify.sanitize(html, {
      ALLOWED_TAGS: ['p', 'br', 'strong', 'em', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6',
                     'ul', 'ol', 'li', 'blockquote', 'code', 'pre', 'hr',
                     'span', 'mark', 'a', 'table', 'thead', 'tbody', 'tr', 'th', 'td', 'div', 'sup', 'sub', 'button'],
      ALLOWED_ATTR: ['class', 'href', 'title', 'dir', 'id', 'data-ref-index'],
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

    const currentQuery = query;
    // Auto-detect response mode from query
    detectedMode = detectQueryMode(currentQuery);
    showFollowUps = false;

    // Add user message to chat thread
    chatMessages = [...chatMessages, { role: 'user', content: currentQuery }];
    scrollToBottom();

    // Add a placeholder assistant message that will be filled during streaming
    const assistantIdx = chatMessages.length;
    chatMessages = [...chatMessages, {
      role: 'assistant',
      content: '',
      results: [],
      query: currentQuery,
      mode: activeMode,
      confidence: null,
      translatedTerms: [],
      detectedDomain: '',
      detectedLanguage: '',
      isStreaming: true,
    }];

    isLoading.set(true);
    error.set('');
    showQuickActions = false;
    aiAnswer.set('');
    streamingAnswer = '';
    isStreaming = true;
    query = '';

    try {
      await sendQueryStream(currentQuery, sessionId, {
        onResults(data) {
          searchResults.set(data.results);
          detectedLanguage = data.detected_language || '';
          detectedDomain = data.detected_domain || '';
          translatedTerms = data.translated_terms || [];
          // Update the assistant message with results
          chatMessages = chatMessages.map((m, i) => i === assistantIdx
            ? { ...m, results: data.results, translatedTerms: data.translated_terms || [], detectedDomain: data.detected_domain || '', detectedLanguage: data.detected_language || '' }
            : m);
          isLoading.set(false);
          scrollToBottom();
        },
        onChunk(content) {
          streamingAnswer += content;
          // Live-update the assistant message content
          chatMessages = chatMessages.map((m, i) => i === assistantIdx
            ? { ...m, content: streamingAnswer }
            : m);
          scrollToBottom();
        },
        onDone(data) {
          const finalAnswer = data.ai_answer || streamingAnswer;
          aiAnswer.set(finalAnswer);
          sessionId = data.session_id;
          isStreaming = false;
          streamingAnswer = '';
          const conf = detectConfidenceTier(finalAnswer, chatMessages[assistantIdx]?.results || []);
          chatMessages = chatMessages.map((m, i) => i === assistantIdx
            ? { ...m, content: finalAnswer, confidence: conf, isStreaming: false, mode: activeMode }
            : m);
          currentSession.set(data);
          loadSessions();
          showFollowUps = true;
          scrollToBottom();
        }
      });
    } catch (e) {
      error.set(e.message);
      try {
        const data = await sendQuery(currentQuery, sessionId);
        searchResults.set(data.results);
        const finalAnswer = data.ai_answer;
        aiAnswer.set(finalAnswer);
        sessionId = data.session_id;
        detectedLanguage = data.detected_language || '';
        detectedDomain = data.detected_domain || '';
        translatedTerms = data.translated_terms || [];
        const conf = detectConfidenceTier(finalAnswer, data.results);
        chatMessages = chatMessages.map((m, i) => i === assistantIdx
          ? { ...m, content: finalAnswer, results: data.results, confidence: conf, isStreaming: false, translatedTerms: data.translated_terms || [], detectedDomain: data.detected_domain || '', detectedLanguage: data.detected_language || '' }
          : m);
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

      // Rebuild chat thread from session messages
      const msgs = session.messages || [];
      const newChatMessages = [];
      
      for (const msg of msgs) {
        if (msg.role === 'user') {
          newChatMessages.push({ role: 'user', content: msg.content });
        } else if (msg.role === 'assistant') {
          newChatMessages.push({
            role: 'assistant',
            content: msg.content,
            results: [],
            query: '',
            mode: 'ringkas',
            confidence: null,
            translatedTerms: [],
            detectedDomain: '',
            detectedLanguage: '',
            isStreaming: false,
          });
        }
      }

      // Re-run the last user query to get search results for the latest answer
      const userMsgs = msgs.filter(m => m.role === 'user');
      const lastUserMsg = userMsgs.length > 0 ? userMsgs[userMsgs.length - 1] : null;

      if (lastUserMsg) {
        query = lastUserMsg.content;
        try {
          const data = await sendQuery(lastUserMsg.content, session.id);
          searchResults.set(data.results);
          detectedLanguage = data.detected_language || '';
          detectedDomain = data.detected_domain || '';
          translatedTerms = data.translated_terms || [];
          
          // Enrich the last assistant message with search results
          if (newChatMessages.length > 0) {
            const lastIdx = newChatMessages.length - 1;
            if (newChatMessages[lastIdx].role === 'assistant') {
              newChatMessages[lastIdx] = {
                ...newChatMessages[lastIdx],
                results: data.results,
                translatedTerms: data.translated_terms || [],
                detectedDomain: data.detected_domain || '',
                detectedLanguage: data.detected_language || '',
                confidence: detectConfidenceTier(newChatMessages[lastIdx].content, data.results),
              };
            }
          }
          aiAnswer.set(data.ai_answer || (newChatMessages.length > 0 ? newChatMessages[newChatMessages.length - 1].content : ''));
        } catch {
          // If re-running fails, still show cached messages
        }
      } else {
        searchResults.set([]);
      }

      chatMessages = newChatMessages;
      scrollToBottom();
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

  async function openBook(bookId, page, bookName = '', snippet = '', rowId = null) {
    readerLoading = true;
    readerBookId = bookId;
    readerBookName = bookName;
    highlightSnippet = snippet;
    showProdukHukumViewer = false;
    produkHukumData = null;
    try {
      readerData = await readBook(bookId, page, rowId);
      readerPage = readerData.current_page || page;
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

  function closeReader() {
    readerClosing = true;
    setTimeout(() => {
      showReader.set(false);
      readerClosing = false;
    }, 280);
  }

  async function openProdukHukum(docId, snippet = '') {
    produkHukumLoading = true;
    showProdukHukumViewer = true;
    showReader.set(false);
    try {
      produkHukumData = await getProdukHukumDetail(docId);
    } catch (e) {
      error.set('Gagal memuat dokumen: ' + e.message);
      showProdukHukumViewer = false;
    } finally {
      produkHukumLoading = false;
    }
  }

  function closeProdukHukumViewer() {
    readerClosing = true;
    setTimeout(() => {
      showProdukHukumViewer = false;
      produkHukumData = null;
      readerClosing = false;
    }, 280);
  }

  function openResult(result) {
    if (result.source_type === 'produk_hukum') {
      openProdukHukum(result.toc_id, result.content_snippet || result.title || '');
    } else {
      const rowId = result.toc_page ? parseInt(result.toc_page, 10) : null;
      openBook(result.book_id, result.page, result.book_name, result.content_snippet || result.title || '', rowId);
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

  async function navigatePage(page, rowId = null) {
    if (!readerBookId) return;
    readerLoading = true;
    highlightSnippet = '';
    try {
      readerData = await readBook(readerBookId, page, rowId);
      readerPage = readerData.current_page || page;
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
    chatMessages = [];
    showReader.set(false);
    showFollowUps = false;
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
    detectedMode = detectQueryMode(text);
    handleQuery();
  }

  function handleFollowUp(suggestion) {
    query = suggestion.query;
    showFollowUps = false;
    detectedMode = detectQueryMode(suggestion.query);
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

  // Event delegation handler for clickable inline references [1], [2], etc.
  function handleRefClick(e) {
    const refBtn = e.target.closest('.ai-inline-ref[data-ref-index]');
    if (!refBtn) return;
    const idx = parseInt(refBtn.dataset.refIndex, 10) - 1; // 1-based to 0-based
    // Find the most recent assistant message with results
    const lastAssistant = [...chatMessages].reverse().find(m => m.role === 'assistant' && m.results?.length > 0);
    if (lastAssistant && lastAssistant.results[idx]) {
      openResult(lastAssistant.results[idx]);
    }
  }

  onMount(() => {
    if (isAuth) { loadSessions(); loadProjects(); }
    window.addEventListener('keydown', handleGlobalKeydown);
    document.addEventListener('click', handleRefClick);
  });

  onDestroy(() => {
    if (typeof window !== 'undefined') {
      window.removeEventListener('keydown', handleGlobalKeydown);
      document.removeEventListener('click', handleRefClick);
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
<div class="layout" class:split-view={showReaderPanel || showProdukHukumViewer} bind:this={layoutEl}>
  <!-- Chat Panel (Task 5: controlled max-width) -->
  <div class="chat-panel" class:with-reader={showReaderPanel || showProdukHukumViewer} style={(showReaderPanel || showProdukHukumViewer) ? `flex: 0 0 ${panelRatio * 100}%` : ''}>
    <!-- Page Toolbar (page-specific actions only; auth/user in layout nav) -->
    {#if isAuth}
    <div class="page-toolbar">
      <button class="toolbar-btn" onclick={newChat} title="Obrolan baru (Ctrl+N)">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
        Baru
      </button>
      <button class="toolbar-btn" onclick={() => showShortcuts = true} title="Pintasan (Ctrl+/)">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="4" width="20" height="16" rx="2"/><path d="M6 8h.01M10 8h.01M14 8h.01M18 8h.01M8 12h.01M12 12h.01M16 12h.01M7 16h10"/></svg>
      </button>
      <button class="toolbar-btn" onclick={() => { if (!showHistory) loadSessions(); showHistory = !showHistory; }} title="Riwayat (Ctrl+.)">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
      </button>
    </div>
    {/if}

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

    <!-- Welcome / Chat Thread -->
    {#if chatMessages.length === 0 && !loading}
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

        <!-- Response Mode Selector -->
        <div class="mode-selector">
          {#each responseModes as mode}
            <button
              class="mode-chip"
              class:active={responseMode === mode.id}
              onclick={() => responseMode = mode.id}
              title={mode.desc}
            >
              <span class="mode-icon">{mode.icon}</span>
              <span class="mode-label">{mode.label}</span>
            </button>
          {/each}
        </div>

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

    <!-- Chat Thread -->
    {#if chatMessages.length > 0}
      <div class="chat-thread" bind:this={chatContainerEl}>
        {#each chatMessages as msg, msgIdx}
          <!-- User Message -->
          {#if msg.role === 'user'}
            <div class="chat-bubble chat-user fade-in">
              <div class="chat-bubble-content">{msg.content}</div>
            </div>
          {/if}

          <!-- Assistant Message -->
          {#if msg.role === 'assistant'}
            <div class="chat-bubble chat-assistant fade-in">
              <!-- Translated Terms Chip -->
              {#if msg.translatedTerms && msg.translatedTerms.length > 0}
                <div class="query-understood">
                  <span class="query-understood-label">🔍</span>
                  <span class="query-understood-text">{msg.translatedTerms.slice(0, 5).join(' · ')}</span>
                </div>
              {/if}

              <!-- Mode + Confidence bar -->
              {#if msg.content || msg.results?.length > 0}
                <div class="response-meta-bar">
                  <div class="response-meta-left">
                    <span class="active-mode-badge">
                      {responseModes.find(m => m.id === (msg.mode || 'ringkas'))?.icon || '✨'} Mode: {responseModes.find(m => m.id === (msg.mode || 'ringkas'))?.label || 'Ringkas'}
                    </span>
                    <div class="inline-mode-switch">
                      {#each responseModes.filter(m => m.id !== 'auto') as mode}
                        <button class="mode-pill" class:active={(msg.mode || activeMode) === mode.id} onclick={() => responseMode = mode.id} title={mode.desc}>{mode.icon}</button>
                      {/each}
                    </div>
                  </div>
                  {#if !msg.isStreaming && msg.confidence}
                    <span class="confidence-badge confidence-{msg.confidence.tier}" title={msg.confidence.desc}>
                      {msg.confidence.icon} {msg.confidence.label}
                    </span>
                  {/if}
                </div>
              {/if}

              <!-- AI Answer Content -->
              {#if msg.content || msg.isStreaming}
                <div class="ai-answer card mode-{msg.mode || activeMode}" class:streaming={msg.isStreaming}>
                  <div class="ai-header">
                    <div class="ai-header-left">
                      <span class="ai-icon">
                        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 2a4 4 0 0 1 4 4v2a4 4 0 0 1-8 0V6a4 4 0 0 1 4-4z"/><path d="M16 14a4 4 0 0 0-8 0v4h8v-4z"/><circle cx="9" cy="9" r="1" fill="currentColor"/><circle cx="15" cy="9" r="1" fill="currentColor"/></svg>
                      </span>
                      <h3>
                        {#if msg.isStreaming}
                          Menyintesis jawaban...
                        {:else if (msg.mode || activeMode) === 'ibaroh'}
                          Ibaroh & Kutipan
                        {:else if (msg.mode || activeMode) === 'lengkap'}
                          Analisis Lengkap
                        {:else if (msg.mode || activeMode) === 'bahtsul-masail'}
                          Rumusan Bahtsul Masail
                        {:else}
                          Jawaban
                        {/if}
                      </h3>
                      {#if msg.isStreaming}
                        <span class="streaming-indicator" role="status" aria-live="polite" aria-label="Sedang menyintesis jawaban">
                          <span class="dot"></span><span class="dot"></span><span class="dot"></span>
                        </span>
                      {/if}
                    </div>
                    <div class="ai-header-right">
                      {#if msg.results?.length > 0}
                        <span class="ai-source-count">Dari {msg.results.length} referensi</span>
                      {/if}
                      {#if !msg.isStreaming && msg.content}
                        <button class="copy-answer-btn" onclick={() => copyToClipboard(msg.content, 'Jawaban')} title="Salin jawaban">
                          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
                        </button>
                      {/if}
                    </div>
                  </div>

                  <div class="ai-content markdown-body">
                    {#if msg.isStreaming && msg.content}
                      {@html renderMarkdown(msg.content)}
                      <span class="typing-cursor">▊</span>
                    {:else if msg.content}
                      {@const sections = parseAnswerSections(msg.content)}
                      {#if sections.length > 1}
                        {#each sections as section}
                          {#if section.type === 'jawaban'}
                            <div class="answer-section section-jawaban">
                              <div class="section-header"><h4 class="section-title">{section.title}</h4></div>
                              <div class="section-content">{@html renderMarkdown(section.content)}</div>
                            </div>
                          {:else if section.type === 'ibaroh'}
                            <div class="answer-section section-ibaroh">
                              <div class="section-header">
                                <h4 class="section-title">{section.title}</h4>
                                <button class="copy-section-btn" onclick={() => copyToClipboard(section.content, 'Ibaroh')} title="Salin ibaroh">
                                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
                                </button>
                              </div>
                              <div class="section-content ibaroh-content">{@html renderMarkdown(section.content)}</div>
                            </div>
                          {:else if section.type === 'khilaf'}
                            <div class="answer-section section-khilaf">
                              <div class="section-header"><h4 class="section-title">{section.title}</h4></div>
                              <div class="section-content">{@html renderMarkdown(section.content)}</div>
                            </div>
                          {:else if section.type === 'kesimpulan'}
                            <div class="answer-section section-kesimpulan">
                              <div class="section-header"><h4 class="section-title">{section.title}</h4></div>
                              <div class="section-content">{@html renderMarkdown(section.content)}</div>
                            </div>
                          {:else if section.type === 'intro'}
                            <div class="section-content">{@html renderMarkdown(section.content)}</div>
                          {:else}
                            <div class="answer-section">
                              {#if section.title}<div class="section-header"><h4 class="section-title">{section.title}</h4></div>{/if}
                              <div class="section-content">{@html renderMarkdown(section.content)}</div>
                            </div>
                          {/if}
                        {/each}
                      {:else}
                        {@html renderMarkdown(msg.content)}
                      {/if}
                    {:else if msg.isStreaming}
                      <div class="ai-thinking">
                        <span class="thinking-text">Menganalisis ibaroh dari kitab-kitab...</span>
                      </div>
                    {/if}
                  </div>

                  <!-- Source chips -->
                  {#if !msg.isStreaming && msg.results?.length > 0}
                    <div class="ai-refs">
                      <span class="ai-refs-label">Sumber:</span>
                      {#each msg.results.slice(0, 5) as result, i}
                        <button class="ai-ref-chip" onclick={() => openResult(result)} title="Buka kitab">
                          [{i+1}] {result.book_name || `Kitab ${result.book_id}`}, hal. {result.page}
                        </button>
                      {/each}
                    </div>
                  {/if}
                </div>
              {/if}

              <!-- Search Results for this message -->
              {#if !msg.isStreaming && msg.results?.length > 0}
                <div class="msg-results">
                  <button class="toggle-results-btn" onclick={() => { msg.showResults = !msg.showResults; chatMessages = chatMessages; }}>
                    {msg.showResults ? '▼' : '▶'} {msg.results.length} referensi ditemukan
                  </button>
                  {#if msg.showResults}
                    <!-- Filter bar -->
                    <div class="results-filter-bar">
                      <div class="filter-group">
                        <button class="filter-chip" class:active={filterSource === 'all'} onclick={() => filterSource = 'all'}>Semua</button>
                        <button class="filter-chip" class:active={filterSource === 'kitab'} onclick={() => filterSource = 'kitab'}>📚 Kitab</button>
                        <button class="filter-chip" class:active={filterSource === 'produk_hukum'} onclick={() => filterSource = 'produk_hukum'}>📋 Produk Hukum</button>
                      </div>
                      <div class="filter-group">
                        <button class="filter-chip" class:active={filterMinScore === 0} onclick={() => filterMinScore = 0}>Semua skor</button>
                        <button class="filter-chip" class:active={filterMinScore === 40} onclick={() => filterMinScore = 40}>●●○ 40+</button>
                        <button class="filter-chip" class:active={filterMinScore === 70} onclick={() => filterMinScore = 70}>●●● 70+</button>
                      </div>
                    </div>
                    {@const filtered = filterResults(msg.results)}
                    {#if filtered.length === 0}
                      <p class="filter-empty">Tidak ada hasil yang cocok dengan filter.</p>
                    {/if}
                    <div class="results-list results-collapsible fade-in">
                      {#each filtered as result, i}
                        <div
                          class="result-card card"
                          class:produk-hukum={result.source_type === 'produk_hukum'}
                          onclick={() => openResult(result)}
                          onkeydown={(e) => { if (e.key === 'Enter') openResult(result); }}
                          role="button"
                          tabindex="0"
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
                            <h4 class="result-title arabic-text" lang="ar">{result.title}</h4>
                          </div>
                          {#if result.hierarchy && result.hierarchy.length > 0}
                            <div class="result-hierarchy">
                              {#each result.hierarchy as h, j}
                                <span class="hierarchy-item">{h}</span>
                                {#if j < result.hierarchy.length - 1}<span class="hierarchy-sep">›</span>{/if}
                              {/each}
                            </div>
                          {/if}
                          {#if result.content_snippet}
                            <p class="result-snippet arabic-text" lang="ar">{result.content_snippet}</p>
                          {/if}
                          <div class="result-meta">
                            <span>📖 {result.book_name || `Kitab ${result.book_id}`}</span>
                            {#if result.author_name}<span>👤 {result.author_name}</span>{/if}
                            {#if result.category}<span>🏷️ {result.category}</span>{/if}
                            <span>📄 Hal. {result.page}</span>
                            <button class="cite-copy-btn" title="Salin referensi" onclick={(e) => { e.stopPropagation(); copyToClipboard(`${result.book_name || 'Kitab ' + result.book_id}, hal. ${result.page}${result.author_name ? ', ' + result.author_name : ''}`, 'Referensi'); }}>
                              📋
                            </button>
                          </div>
                        </div>
                      {/each}
                    </div>
                  {/if}
                </div>
              {/if}
            </div>

            <!-- Follow-up Suggestions (only for last assistant message) -->
            {#if msgIdx === chatMessages.length - 1 && showFollowUps && !msg.isStreaming && followUpSuggestions.length > 0}
              <div class="follow-ups fade-in">
                <span class="follow-ups-label">Lanjutkan pencarian:</span>
                <div class="follow-ups-grid">
                  {#each followUpSuggestions as suggestion}
                    <button class="follow-up-chip" onclick={() => handleFollowUp(suggestion)}>
                      <span class="follow-up-icon">{suggestion.icon}</span>
                      {suggestion.label}
                    </button>
                  {/each}
                </div>
              </div>
            {/if}
          {/if}
        {/each}
      </div>
    {/if}

    <!-- Loading Skeleton (only shown when no messages yet) -->
    {#if loading && chatMessages.length === 0}
      <div class="loading-skeleton fade-in">
        <div class="skeleton-bubble skeleton-user">
          <div class="skeleton-line skeleton-short"></div>
        </div>
        <div class="skeleton-bubble skeleton-assistant">
          <div class="skeleton-line skeleton-full"></div>
          <div class="skeleton-line skeleton-medium"></div>
          <div class="skeleton-line skeleton-long"></div>
          <div class="skeleton-line skeleton-short"></div>
        </div>
        <p class="skeleton-label">Sedang mencari di kitab-kitab...</p>
      </div>
    {/if}

    <!-- Error -->
    {#if errorMsg}
      <div class="alert alert-error fade-in">
        {errorMsg}
        <button class="btn btn-ghost btn-sm" onclick={() => error.set('')} style="margin-left: 8px;">✕</button>
      </div>
    {/if}

    <!-- Search Input -->
    <div class="search-bar">
      <!-- Compact mode selector when chat is active -->
      {#if chatMessages.length > 0}
        <div class="search-mode-bar">
          {#each responseModes as mode}
            <button
              class="search-mode-chip"
              class:active={responseMode === mode.id}
              onclick={() => responseMode = mode.id}
              title={mode.desc}
            >
              {mode.icon} {mode.label}
            </button>
          {/each}
        </div>
      {/if}
      <div class="search-input-wrapper">
        <textarea
          class="search-input"
          bind:value={query}
          placeholder={chatMessages.length > 0 ? 'Lanjutkan pertanyaan...' : 'Tanyakan masalah fikih... (Ctrl+K)'}
          onkeydown={handleKeydown}
          rows="1"
          aria-label="Pertanyaan fikih"
          onfocus={() => { if (!query && chatMessages.length === 0) showQuickActions = true; }}
          onblur={() => setTimeout(() => showQuickActions = false, 200)}
        ></textarea>
        <button
          class="btn btn-primary search-btn"
          onclick={handleQuery}
          disabled={loading || isStreaming || !query.trim()}
          aria-label="Kirim pertanyaan"
        >
          {#if loading || isStreaming}
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

    <!-- Export Floating Button (when chat has content) -->
    {#if chatMessages.length > 0 && !isStreaming}
      <div class="export-float">
        <button class="export-float-btn" onclick={() => showExportMenu = !showExportMenu} title="Ekspor hasil" aria-label="Ekspor hasil" aria-expanded={showExportMenu}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
        </button>
        {#if showExportMenu}
          <div class="export-dropdown fade-in">
            <button class="export-option" onclick={exportMarkdown}>📝 Markdown (.md)</button>
            <button class="export-option" onclick={exportDocx}>📄 Word (.doc)</button>
            <button class="export-option" onclick={exportPdf}>📋 PDF</button>
            <button class="export-option" onclick={exportPlainText}>📃 Plain Text</button>
          </div>
        {/if}
      </div>
    {/if}

    <!-- Copy Toast -->
    {#if copyToast}
      <div class="copy-toast fade-in">{copyToast}</div>
    {/if}
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
    <div class="reader-panel fade-in" class:closing={readerClosing}>
      <div class="reader-header">
        <button class="btn btn-ghost close-btn" onclick={closeReader} aria-label="Tutup panel pembaca">✕ Tutup</button>
        <button class="btn btn-ghost toc-toggle-btn" onclick={() => showMobileToc = !showMobileToc} aria-label="Toggle daftar isi" aria-expanded={showMobileToc}>📑 Daftar Isi</button>
        <h3 class="reader-title arabic-text" lang="ar">
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
                <button class="toc-item" class:active={readerPage === node.page} onclick={() => navigatePage(node.page, parseInt(node.page, 10) || null)}>
                  {node.content}
                </button>
                {#if node.children && node.children.length > 0}
                  <div class="toc-children">
                    {#each node.children as child}
                      <button class="toc-item toc-child" class:active={readerPage === child.page} onclick={() => navigatePage(child.page, parseInt(child.page, 10) || null)}>
                        {child.content}
                      </button>
                      {#if child.children && child.children.length > 0}
                        <div class="toc-children">
                          {#each child.children as grandchild}
                            <button class="toc-item toc-grandchild" class:active={readerPage === grandchild.page} onclick={() => navigatePage(grandchild.page, parseInt(grandchild.page, 10) || null)}>
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
              <div class="page-content arabic-text" lang="ar">
                {@html highlightContent(page.content.replace(/\^M/g, '<br/>'))}
              </div>
            {/each}
            
            <div class="page-nav">
              <button class="btn btn-secondary" onclick={() => navigatePage(String(Math.max(1, parseInt(readerPage || '1') - 1)))} disabled={readerPage === '1'}>
                ← Sebelumnya
              </button>
              <span class="page-info">
                Hal.
                <input
                  type="number"
                  class="page-jump-input"
                  value={readerPage || '1'}
                  min="1"
                  max={readerData.total_pages}
                  onkeydown={(e) => { if (e.key === 'Enter') { const v = parseInt(e.target.value); if (v >= 1 && v <= readerData.total_pages) navigatePage(String(v)); } }}
                  onchange={(e) => { const v = parseInt(e.target.value); if (v >= 1 && v <= readerData.total_pages) navigatePage(String(v)); }}
                />
                / {readerData.total_pages}
              </span>
              <button class="btn btn-secondary" onclick={() => navigatePage(String(parseInt(readerPage || '1') + 1))}>
                Berikutnya →
              </button>
            </div>
          {/if}
        </div>
      </div>
    </div>
  {/if}

  <!-- Produk Hukum Document Viewer Panel -->
  {#if showProdukHukumViewer}
    <div class="reader-panel fade-in" class:closing={readerClosing}>
      <div class="reader-header">
        <button class="btn btn-ghost close-btn" onclick={closeProdukHukumViewer} aria-label="Tutup panel dokumen">✕ Tutup</button>
        <h3 class="reader-title">
          {#if produkHukumData}
            📋 {produkHukumData.title}
          {:else}
            📋 Memuat dokumen...
          {/if}
        </h3>
      </div>
      
      <div class="reader-layout">
        {#if produkHukumLoading}
          <div class="reader-content">
            <div class="loading-container">
              <div class="loading-spinner"></div>
            </div>
          </div>
        {:else if produkHukumData}
          <div class="produk-hukum-sidebar">
            <h4>Info Dokumen</h4>
            <div class="ph-info-list">
              <div class="ph-info-item">
                <span class="ph-info-label">Kategori</span>
                <span class="ph-info-value">{produkHukumData.category}</span>
              </div>
              {#if produkHukumData.subcategory}
                <div class="ph-info-item">
                  <span class="ph-info-label">Sub-kategori</span>
                  <span class="ph-info-value">{produkHukumData.subcategory}</span>
                </div>
              {/if}
              <div class="ph-info-item">
                <span class="ph-info-label">Tipe File</span>
                <span class="ph-info-value">{produkHukumData.file_type.toUpperCase()}</span>
              </div>
              <div class="ph-info-item">
                <span class="ph-info-label">Halaman</span>
                <span class="ph-info-value">{produkHukumData.page_count}</span>
              </div>
              {#if produkHukumData.source_file}
                <div class="ph-info-item">
                  <span class="ph-info-label">File Sumber</span>
                  <span class="ph-info-value ph-source-file">{produkHukumData.source_file}</span>
                </div>
              {/if}
            </div>
          </div>
          <div class="reader-content produk-hukum-content">
            <div class="ph-document-body">
              {@html DOMPurify.sanitize(produkHukumData.content.replace(/\n/g, '<br/>'), {
                ALLOWED_TAGS: ['br', 'p', 'div', 'span', 'b', 'i', 'em', 'strong', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'ul', 'ol', 'li', 'table', 'tr', 'td', 'th', 'thead', 'tbody', 'mark', 'sup', 'sub', 'blockquote'],
                ALLOWED_ATTR: ['class', 'dir', 'data-type', 'style']
              })}
            </div>
          </div>
        {/if}
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

  /* ─── Page Toolbar ─── */
  .page-toolbar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 0;
    border-bottom: 1px solid var(--color-border);
  }

  .toolbar-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 5px 10px;
    border: none;
    background: none;
    cursor: pointer;
    border-radius: 8px;
    color: var(--color-text-light);
    font-size: 0.8rem;
    transition: all 0.15s;
  }

  .toolbar-btn:hover {
    background: var(--color-bg-alt);
    color: var(--color-primary);
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

  /* ─── Response Mode Selector ─── */
  .mode-selector {
    display: flex;
    gap: 6px;
    margin-top: 20px;
    flex-wrap: wrap;
    justify-content: center;
  }

  .mode-chip {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 14px;
    background: var(--color-surface);
    border: 1.5px solid var(--color-border);
    border-radius: 20px;
    cursor: pointer;
    font-size: 0.82rem;
    color: var(--color-text-light);
    font-family: var(--font-ui);
    transition: all 0.2s;
  }

  .mode-chip:hover {
    border-color: var(--color-primary);
    color: var(--color-primary);
    background: #f0f9f4;
  }

  .mode-chip.active {
    border-color: var(--color-primary);
    background: var(--color-primary);
    color: white;
  }

  .mode-icon { font-size: 1rem; }
  .mode-label { font-weight: 500; }

  /* ─── Response Meta Bar ─── */
  .response-meta-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 12px;
    flex-wrap: wrap;
  }

  .response-meta-left {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .active-mode-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 12px;
    background: var(--color-bg-alt);
    border-radius: 16px;
    font-size: 0.78rem;
    color: var(--color-text-light);
    font-weight: 500;
  }

  .auto-tag {
    font-size: 0.65rem;
    padding: 1px 5px;
    background: var(--color-primary);
    color: white;
    border-radius: 8px;
    margin-left: 2px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .inline-mode-switch {
    display: flex;
    gap: 2px;
    background: var(--color-bg-alt);
    border-radius: 12px;
    padding: 2px;
  }

  .mode-pill {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border: none;
    background: none;
    border-radius: 10px;
    cursor: pointer;
    font-size: 0.85rem;
    transition: all 0.15s;
  }

  .mode-pill:hover {
    background: var(--color-surface);
  }

  .mode-pill.active {
    background: var(--color-primary);
    box-shadow: 0 1px 4px rgba(0,0,0,0.15);
  }

  /* ─── Confidence Badge ─── */
  .confidence-badge {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 12px;
    border-radius: 16px;
    font-size: 0.75rem;
    font-weight: 600;
    flex-shrink: 0;
  }

  .confidence-qathi {
    background: #e8f5e9;
    color: #2e7d32;
  }

  .confidence-zhanni {
    background: #fff8e1;
    color: #f57f17;
  }

  .confidence-ghaib {
    background: #ffebee;
    color: #c62828;
  }

  /* ─── Search Mode Bar (compact, in search bar area) ─── */
  .search-mode-bar {
    display: flex;
    gap: 4px;
    margin-bottom: 8px;
    flex-wrap: wrap;
  }

  .search-mode-chip {
    padding: 4px 10px;
    background: none;
    border: 1px solid var(--color-border);
    border-radius: 14px;
    cursor: pointer;
    font-size: 0.72rem;
    color: var(--color-text-muted);
    font-family: var(--font-ui);
    transition: all 0.15s;
  }

  .search-mode-chip:hover {
    border-color: var(--color-primary);
    color: var(--color-primary);
  }

  .search-mode-chip.active {
    border-color: var(--color-primary);
    background: var(--color-primary);
    color: white;
  }

  /* ─── Enhanced AI Answer Sections ─── */
  .ai-header-right {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .copy-answer-btn, .copy-section-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    border-radius: 6px;
    cursor: pointer;
    color: var(--color-text-muted);
    transition: all 0.15s;
  }

  .copy-answer-btn:hover, .copy-section-btn:hover {
    border-color: var(--color-primary);
    color: var(--color-primary);
    background: #f0f9f4;
  }

  .copy-section-btn {
    width: 24px;
    height: 24px;
  }

  .answer-section {
    margin-bottom: 16px;
    padding: 14px 16px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
  }

  .section-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }

  .section-title {
    font-size: 1rem;
    font-weight: 700;
    color: var(--color-primary);
    margin: 0;
    font-family: var(--font-ui);
  }

  /* Jawaban section — green accent */
  .section-jawaban {
    background: linear-gradient(135deg, #f0f9f4, #fff);
    border-color: #c8e6c9;
    border-left: 3px solid #4caf50;
  }

  .section-jawaban .section-title { color: #2e7d32; }

  /* Ibaroh section — gold accent */
  .section-ibaroh {
    background: linear-gradient(135deg, #fffdf5, #fff);
    border-color: #f5e6c8;
    border-left: 3px solid var(--color-secondary);
  }

  .section-ibaroh .section-title { color: #8d6e00; }

  .ibaroh-content {
    font-family: var(--font-arabic);
    line-height: 2.4;
  }

  .ibaroh-content :global(blockquote) {
    margin: 10px 0;
    padding: 14px 18px;
    border-left: 4px solid var(--color-secondary);
    background: rgba(201, 168, 76, 0.08);
    border-radius: 0 8px 8px 0;
    font-size: 1.1rem;
    line-height: 2.4;
    direction: rtl;
    text-align: right;
  }

  /* Khilaf section — blue accent */
  .section-khilaf {
    background: linear-gradient(135deg, #f5f9ff, #fff);
    border-color: #c8d6f5;
    border-left: 3px solid #1976d2;
  }

  .section-khilaf .section-title { color: #1565c0; }

  /* Kesimpulan section — primary accent */
  .section-kesimpulan {
    background: linear-gradient(135deg, #f5faf7, #fff);
    border-color: #b8dcc8;
    border-left: 3px solid var(--color-primary);
  }

  .section-kesimpulan .section-title { color: var(--color-primary); }

  /* Mode-specific AI answer card variations */
  .ai-answer.mode-ibaroh {
    border-left-color: var(--color-secondary);
  }

  .ai-answer.mode-lengkap {
    border-left-color: #1976d2;
  }

  .ai-answer.mode-bahtsul-masail {
    border-left-color: #7b1fa2;
    background: linear-gradient(135deg, #faf5ff, #fff);
  }

  /* ─── Follow-up Suggestions ─── */
  .follow-ups {
    margin-bottom: 16px;
    padding: 12px;
    background: var(--color-bg-alt);
    border-radius: var(--radius-sm);
  }

  .follow-ups-label {
    font-size: 0.78rem;
    color: var(--color-text-muted);
    font-weight: 500;
    display: block;
    margin-bottom: 8px;
  }

  .follow-ups-grid {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .follow-up-chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 6px 12px;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: 16px;
    cursor: pointer;
    font-size: 0.78rem;
    color: var(--color-text);
    font-family: var(--font-ui);
    transition: all 0.2s;
  }

  .follow-up-chip:hover {
    border-color: var(--color-primary);
    background: #f0f9f4;
    transform: translateY(-1px);
    box-shadow: 0 2px 6px rgba(0,0,0,0.06);
  }

  .follow-up-icon { font-size: 0.9rem; }

  /* ─── Copy Toast ─── */
  .copy-toast {
    position: fixed;
    bottom: 80px;
    left: 50%;
    transform: translateX(-50%);
    padding: 8px 20px;
    background: var(--color-primary);
    color: white;
    border-radius: 20px;
    font-size: 0.82rem;
    font-weight: 500;
    box-shadow: 0 4px 16px rgba(0,0,0,0.2);
    z-index: 1000;
    animation: toastIn 0.3s ease-out;
  }

  @keyframes toastIn {
    from { opacity: 0; transform: translateX(-50%) translateY(10px); }
    to { opacity: 1; transform: translateX(-50%) translateY(0); }
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

  /* ─── Chat Thread ─── */
  .chat-thread {
    flex: 1;
    overflow-y: auto;
    padding: 20px 0;
    display: flex;
    flex-direction: column;
    gap: 16px;
    scroll-behavior: smooth;
  }

  .chat-bubble {
    max-width: 95%;
    animation: chatBubbleIn 0.4s cubic-bezier(0.34, 1.56, 0.64, 1) both;
  }

  @keyframes chatBubbleIn {
    from {
      opacity: 0;
      transform: translateY(16px) scale(0.97);
    }
    to {
      opacity: 1;
      transform: translateY(0) scale(1);
    }
  }

  .chat-user {
    align-self: flex-end;
  }

  .chat-user .chat-bubble-content {
    background: var(--color-primary);
    color: white;
    padding: 10px 16px;
    border-radius: 16px 16px 4px 16px;
    font-size: 0.95rem;
    line-height: 1.6;
    white-space: pre-wrap;
    word-break: break-word;
  }

  .chat-assistant {
    align-self: flex-start;
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 8px;
    animation: chatBubbleIn 0.5s cubic-bezier(0.34, 1.56, 0.64, 1) both;
    animation-delay: 0.1s;
  }

  /* Inline reference buttons in AI response */
  :global(.ai-inline-ref) {
    display: inline;
    background: var(--color-primary-bg, #e8f5e9);
    color: var(--color-primary, #2e7d32);
    border: 1px solid var(--color-primary, #2e7d32);
    border-radius: 4px;
    padding: 0 4px;
    font-size: 0.85em;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.2s ease;
    vertical-align: baseline;
    line-height: 1;
  }
  :global(.ai-inline-ref:hover) {
    background: var(--color-primary, #2e7d32);
    color: white;
    transform: scale(1.1);
  }

  /* ─── Collapsible Results in Chat ─── */
  .msg-results {
    margin-top: 4px;
  }

  .toggle-results-btn {
    background: var(--color-bg-alt);
    border: 1px solid var(--color-border);
    padding: 6px 14px;
    border-radius: var(--radius-sm);
    font-size: 0.85rem;
    color: var(--color-text-light);
    cursor: pointer;
    transition: all 0.15s;
    font-family: var(--font-ui);
    width: 100%;
    text-align: left;
  }

  .toggle-results-btn:hover {
    background: var(--color-border);
    color: var(--color-text);
  }

  .results-filter-bar {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    padding: 8px 0;
    margin-bottom: 4px;
  }

  .filter-group {
    display: flex;
    gap: 4px;
  }

  .filter-chip {
    padding: 4px 10px;
    border-radius: 14px;
    border: 1px solid var(--color-border);
    background: var(--color-surface);
    color: var(--color-text-light);
    font-size: 0.75rem;
    cursor: pointer;
    transition: var(--transition);
  }

  .filter-chip.active {
    background: var(--color-primary);
    color: white;
    border-color: var(--color-primary);
  }

  .filter-chip:hover:not(.active) {
    border-color: var(--color-primary-light);
    color: var(--color-primary);
  }

  .filter-empty {
    text-align: center;
    color: var(--color-text-muted);
    font-size: 0.85rem;
    padding: 16px;
  }

  .results-collapsible {
    margin-top: 8px;
  }

  /* ─── Skeleton Loader ─── */
  .loading-skeleton {
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 32px 0;
    max-width: 720px;
    margin: 0 auto;
  }

  .skeleton-bubble {
    border-radius: 16px;
    padding: 16px 20px;
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .skeleton-user {
    align-self: flex-end;
    background: var(--color-primary);
    opacity: 0.15;
    max-width: 40%;
  }

  .skeleton-assistant {
    align-self: flex-start;
    background: var(--color-bg-alt);
    width: 85%;
  }

  .skeleton-line {
    height: 14px;
    border-radius: 7px;
    background: linear-gradient(90deg, var(--color-border) 25%, var(--color-bg) 50%, var(--color-border) 75%);
    background-size: 200% 100%;
    animation: shimmer 1.5s infinite;
  }

  .skeleton-short { width: 45%; }
  .skeleton-medium { width: 65%; }
  .skeleton-long { width: 85%; }
  .skeleton-full { width: 100%; }

  .skeleton-label {
    text-align: center;
    color: var(--color-text-muted);
    font-size: 0.85rem;
    margin-top: 8px;
  }

  @keyframes shimmer {
    0% { background-position: 200% 0; }
    100% { background-position: -200% 0; }
  }

  /* ─── Export Float ─── */
  .export-float {
    position: fixed;
    bottom: 90px;
    right: 24px;
    z-index: 50;
  }

  .export-float-btn {
    width: 44px;
    height: 44px;
    border-radius: 50%;
    background: var(--color-primary);
    color: white;
    border: none;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: var(--shadow-lg);
    transition: all 0.2s;
  }

  .export-float-btn:hover {
    transform: scale(1.1);
    background: var(--color-primary-light);
  }

  .export-dropdown {
    position: absolute;
    bottom: 52px;
    right: 0;
    background: var(--color-surface);
    border: 1px solid var(--color-border);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow-lg);
    min-width: 160px;
    overflow: hidden;
  }

  .export-option {
    display: block;
    width: 100%;
    padding: 10px 14px;
    border: none;
    background: none;
    text-align: left;
    cursor: pointer;
    font-size: 0.85rem;
    font-family: var(--font-ui);
    transition: background 0.15s;
  }

  .export-option:hover {
    background: var(--color-bg-alt);
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
    align-items: center;
  }

  .cite-copy-btn {
    margin-left: auto;
    background: none;
    border: none;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 0.8rem;
    opacity: 0.5;
    transition: opacity 0.15s;
  }

  .cite-copy-btn:hover {
    opacity: 1;
    background: var(--color-bg-alt);
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
    flex-wrap: wrap;
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
    animation: slideInPanel 0.3s ease-out;
  }

  .reader-panel.closing {
    animation: slideOutPanel 0.28s ease-in forwards;
  }

  @keyframes slideInPanel {
    from { opacity: 0; transform: translateX(40px); }
    to { opacity: 1; transform: translateX(0); }
  }

  @keyframes slideOutPanel {
    from { opacity: 1; transform: translateX(0); }
    to { opacity: 0; transform: translateX(40px); }
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
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .page-jump-input {
    width: 52px;
    padding: 3px 6px;
    border: 1px solid var(--color-border);
    border-radius: 6px;
    text-align: center;
    font-size: 0.85rem;
    background: var(--color-surface);
    color: var(--color-text);
    -moz-appearance: textfield;
  }

  .page-jump-input::-webkit-outer-spin-button,
  .page-jump-input::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }

  .toc-toggle-btn { display: none; }

  /* ─── Produk Hukum Document Viewer ─── */
  .produk-hukum-sidebar {
    width: 240px;
    border-right: 1px solid var(--color-border);
    overflow-y: auto;
    padding: 14px;
    background: var(--color-bg);
    flex-shrink: 0;
  }

  .produk-hukum-sidebar h4 {
    margin-bottom: 12px;
    color: var(--color-primary);
    font-family: var(--font-ui);
    font-size: 0.9rem;
  }

  .ph-info-list {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .ph-info-item {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .ph-info-label {
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    color: var(--color-text-muted);
    font-weight: 600;
  }

  .ph-info-value {
    font-size: 0.85rem;
    color: var(--color-text);
  }

  .ph-source-file {
    word-break: break-all;
    font-size: 0.78rem;
    color: var(--color-text-light);
  }

  .produk-hukum-content {
    direction: ltr;
  }

  .ph-document-body {
    line-height: 1.9;
    font-size: 1.05rem;
    white-space: pre-wrap;
    word-wrap: break-word;
  }

  /* ─── Responsive ─── */
  @media (max-width: 768px) {
    .chat-panel {
      padding: 0 12px;
    }

    .chat-panel.with-reader {
      min-width: unset;
    }

    .page-toolbar {
      padding: 4px 0;
    }

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

    .mode-selector { gap: 4px; }
    .mode-chip { padding: 6px 10px; font-size: 0.75rem; }
    .mode-label { display: none; }
    
    .response-meta-bar { flex-direction: column; align-items: flex-start; gap: 6px; }
    .inline-mode-switch { display: none; }
    
    .follow-ups-grid { gap: 4px; }
    .follow-up-chip { font-size: 0.72rem; padding: 5px 10px; }
    
    .search-mode-bar { display: none; }

    .answer-section { padding: 10px 12px; }
  }

  @media (max-width: 360px) {
    .chat-panel { padding: 0 8px; }
    .welcome h2 { font-size: 1.1rem; }
    .result-title { font-size: 0.9rem; }
  }
</style>
