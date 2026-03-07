import { auth } from './stores.js';
import { get } from 'svelte/store';

const API_BASE = '/api';

async function request(path, options = {}) {
  const authState = get(auth);
  const headers = {
    'Content-Type': 'application/json',
    ...(authState.token ? { Authorization: `Bearer ${authState.token}` } : {}),
    ...options.headers
  };

  const res = await fetch(`${API_BASE}${path}`, {
    ...options,
    headers
  });

  if (res.status === 401) {
    auth.logout();
    throw new Error('Session expired. Please login again.');
  }

  const data = await res.json();

  if (!res.ok) {
    throw new Error(data.error || 'Request failed');
  }

  return data;
}

// Auth
export async function register(email, password, displayName = '') {
  return request('/auth/register', {
    method: 'POST',
    body: JSON.stringify({ email, password, display_name: displayName || undefined })
  });
}

export async function login(email, password) {
  return request('/auth/login', {
    method: 'POST',
    body: JSON.stringify({ email, password })
  });
}

// Profile
export async function getProfile() {
  return request('/auth/profile');
}

export async function updateProfile(displayName = null, email = null) {
  return request('/auth/profile', {
    method: 'PUT',
    body: JSON.stringify({
      display_name: displayName,
      email: email
    })
  });
}

export async function changePassword(currentPassword, newPassword) {
  return request('/auth/change-password', {
    method: 'POST',
    body: JSON.stringify({
      current_password: currentPassword,
      new_password: newPassword
    })
  });
}

// Search / Query
export async function sendQuery(query, sessionId = null) {
  return request('/query', {
    method: 'POST',
    body: JSON.stringify({ query, session_id: sessionId })
  });
}

/**
 * Stream AI-synthesized answer via SSE.
 * @param {string} query
 * @param {string|null} sessionId
 * @param {{ onResults?: Function, onChunk?: Function, onDone?: Function, onError?: Function }} callbacks
 * @returns {Promise<void>}
 */
export async function sendQueryStream(query, sessionId = null, callbacks = {}) {
  const authState = get(auth);
  const headers = {
    'Content-Type': 'application/json',
    ...(authState.token ? { Authorization: `Bearer ${authState.token}` } : {})
  };

  const res = await fetch(`${API_BASE}/query/stream`, {
    method: 'POST',
    headers,
    body: JSON.stringify({ query, session_id: sessionId })
  });

  if (res.status === 401) {
    auth.logout();
    throw new Error('Session expired. Please login again.');
  }

  if (!res.ok) {
    const errData = await res.json().catch(() => ({ error: 'Stream request failed' }));
    throw new Error(errData.error || 'Stream request failed');
  }

  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;

    buffer += decoder.decode(value, { stream: true });

    // Parse SSE events (separated by double newline)
    const parts = buffer.split('\n\n');
    buffer = parts.pop(); // Keep incomplete chunk in buffer

    for (const part of parts) {
      if (!part.trim()) continue;

      let eventType = 'message';
      let data = '';

      for (const line of part.split('\n')) {
        if (line.startsWith('event: ')) {
          eventType = line.slice(7).trim();
        } else if (line.startsWith('data: ')) {
          data += line.slice(6);
        }
      }

      if (!data) continue;

      try {
        const parsed = JSON.parse(data);
        if (eventType === 'search_results' && callbacks.onResults) {
          callbacks.onResults(parsed);
        } else if (eventType === 'ai_chunk' && callbacks.onChunk) {
          callbacks.onChunk(parsed.content);
        } else if (eventType === 'ai_done' && callbacks.onDone) {
          callbacks.onDone(parsed);
        }
      } catch {
        // Non-JSON data, skip
      }
    }
  }
}

// Book
export async function readBook(bookId, page = null) {
  return request('/book', {
    method: 'POST',
    body: JSON.stringify({ book_id: bookId, page })
  });
}

// Sessions
export async function getSessions() {
  return request('/sessions');
}

export async function getSession(sessionId) {
  return request(`/sessions/${sessionId}`);
}

export async function deleteSession(sessionId) {
  return request(`/sessions/${sessionId}`, { method: 'DELETE' });
}

export async function deleteSessionsBatch(sessionIds) {
  return request('/sessions/delete', {
    method: 'POST',
    body: JSON.stringify({ session_ids: sessionIds })
  });
}

export async function renameSession(sessionId, title) {
  return request(`/sessions/${sessionId}`, {
    method: 'PUT',
    body: JSON.stringify({ title })
  });
}

// Status
export async function getStatus() {
  return request('/status');
}

// ─── Produk Hukum (public — no auth needed) ───

export async function getProdukHukumList(page = 1, perPage = 20, category = null) {
  let url = `/produk-hukum?page=${page}&per_page=${perPage}`;
  if (category) url += `&category=${encodeURIComponent(category)}`;
  const res = await fetch(`${API_BASE}${url}`);
  const data = await res.json();
  if (!res.ok) throw new Error(data.error || 'Request failed');
  return data;
}

export async function getProdukHukumDetail(id) {
  const res = await fetch(`${API_BASE}/produk-hukum/${id}`);
  const data = await res.json();
  if (!res.ok) throw new Error(data.error || 'Request failed');
  return data;
}

export async function searchProdukHukum(query, limit = 20) {
  const res = await fetch(`${API_BASE}/produk-hukum/search?q=${encodeURIComponent(query)}&limit=${limit}`);
  const data = await res.json();
  if (!res.ok) throw new Error(data.error || 'Request failed');
  return data;
}

export async function getProdukHukumStats() {
  const res = await fetch(`${API_BASE}/produk-hukum/stats`);
  const data = await res.json();
  if (!res.ok) throw new Error(data.error || 'Request failed');
  return data;
}

// ─── API Keys (Task 11) ───

export async function getApiKeys() {
  return request('/api-keys');
}

export async function createApiKey(name, permissions = null, rateLimitPerMin = 30, expiresInDays = null) {
  return request('/api-keys', {
    method: 'POST',
    body: JSON.stringify({
      name,
      permissions,
      rate_limit: rateLimitPerMin,
      expires_in_days: expiresInDays
    })
  });
}

export async function revokeApiKey(keyId) {
  return request(`/api-keys/${keyId}`, { method: 'DELETE' });
}
