import { writable, derived } from 'svelte/store';

// Auth store
function createAuthStore() {
  const { subscribe, set, update } = writable({
    token: null,
    user: null,
    isAuthenticated: false
  });

  return {
    subscribe,
    login(token, user) {
      if (typeof window !== 'undefined') {
        localStorage.setItem('kizana_token', token);
        localStorage.setItem('kizana_user', JSON.stringify(user));
      }
      set({ token, user, isAuthenticated: true });
    },
    logout() {
      if (typeof window !== 'undefined') {
        localStorage.removeItem('kizana_token');
        localStorage.removeItem('kizana_user');
      }
      set({ token: null, user: null, isAuthenticated: false });
    },
    restore() {
      if (typeof window !== 'undefined') {
        const token = localStorage.getItem('kizana_token');
        const userStr = localStorage.getItem('kizana_user');
        if (token && userStr) {
          try {
            const user = JSON.parse(userStr);
            set({ token, user, isAuthenticated: true });
          } catch {
            set({ token: null, user: null, isAuthenticated: false });
          }
        }
      }
    }
  };
}

export const auth = createAuthStore();

// Chat store
export const currentSession = writable(null);
export const sessions = writable([]);
export const searchResults = writable([]);
export const aiAnswer = writable('');
export const isLoading = writable(false);
export const error = writable('');

// Book reader store
export const selectedBook = writable(null);
export const bookData = writable(null);
export const showReader = writable(false);
