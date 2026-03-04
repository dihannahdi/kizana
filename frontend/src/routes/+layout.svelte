<script>
  import '../app.css';
  import { auth } from '$lib/stores.js';
  import { onMount } from 'svelte';
  import { page } from '$app/state';

  let { children } = $props();
  let menuOpen = $state(false);

  onMount(() => {
    auth.restore();
  });

  let currentPath = $derived(page.url?.pathname || '/');

  function closeMenu() {
    menuOpen = false;
  }
</script>

<div class="app-shell">
  <nav class="site-nav">
    <div class="nav-inner">
      <a href="/" class="nav-brand">بحث المسائل</a>
      
      <!-- Hamburger toggle for mobile -->
      <button class="menu-toggle" onclick={() => menuOpen = !menuOpen} aria-label="Menu navigasi">
        {#if menuOpen}
          <svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
        {:else}
          <svg xmlns="http://www.w3.org/2000/svg" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="18" x2="21" y2="18"/></svg>
        {/if}
      </button>

      <div class="nav-links" class:open={menuOpen}>
        <a href="/" class="nav-link" class:active={currentPath === '/'} onclick={closeMenu}>
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/><polyline points="9 22 9 12 15 12 15 22"/></svg>
          Beranda
        </a>
        <a href="/tentang" class="nav-link" class:active={currentPath === '/tentang'} onclick={closeMenu}>
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/></svg>
          Tentang
        </a>
        <a href="/statistik" class="nav-link" class:active={currentPath === '/statistik'} onclick={closeMenu}>
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="20" x2="18" y2="10"/><line x1="12" y1="20" x2="12" y2="4"/><line x1="6" y1="20" x2="6" y2="14"/></svg>
          Statistik
        </a>
        <a href="/api-docs" class="nav-link" class:active={currentPath === '/api-docs'} onclick={closeMenu}>
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 18l6-6-6-6"/><path d="M8 6l-6 6 6 6"/></svg>
          API
        </a>
        <a href="/bantuan" class="nav-link" class:active={currentPath === '/bantuan'} onclick={closeMenu}>
          <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><path d="M9.09 9a3 3 0 0 1 5.83 1c0 2-3 3-3 3"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>
          Bantuan
        </a>
      </div>
    </div>
  </nav>

  <main class="app-main">
    {@render children()}
  </main>

  <footer class="site-footer">
    <div class="footer-inner">
      <div class="footer-brand">
        <span class="footer-logo">بحث المسائل</span>
        <p class="footer-tagline">bahtsulmasail.tech — Mesin pencari khazanah turats Islam klasik — 7.872 kitab dalam genggaman Anda.</p>
      </div>
      <div class="footer-links">
        <a href="/">Beranda</a>
        <a href="/tentang">Tentang</a>
        <a href="/statistik">Statistik</a>
        <a href="/api-docs">API</a>
        <a href="/bantuan">Bantuan</a>
        <a href="/pelajari">Pelajari</a>
        <a href="/pengaturan">Pengaturan</a>
      </div>
      <div class="footer-copy">
        &copy; {new Date().getFullYear()} Bahtsul Masail &mdash; bahtsulmasail.tech
      </div>
    </div>
  </footer>
</div>

<style>
  .app-shell {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--color-bg);
  }

  .site-nav {
    background: var(--color-surface);
    border-bottom: 1px solid var(--color-border);
    position: sticky;
    top: 0;
    z-index: 100;
    box-shadow: 0 1px 3px rgba(0,0,0,0.06);
  }

  .nav-inner {
    max-width: 1400px;
    margin: 0 auto;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 24px;
    height: 52px;
    direction: ltr;
    position: relative;
  }

  .nav-brand {
    font-family: 'Amiri', 'Noto Naskh Arabic', serif;
    font-size: 1.5rem;
    font-weight: 700;
    color: var(--color-primary);
    text-decoration: none;
    letter-spacing: 1px;
  }

  /* Hamburger toggle — hidden on desktop */
  .menu-toggle {
    display: none;
    background: none;
    border: none;
    padding: 6px;
    color: var(--color-text);
    cursor: pointer;
    border-radius: 6px;
    align-items: center;
    justify-content: center;
    transition: background 0.15s;
  }

  .menu-toggle:hover {
    background: var(--color-bg-alt);
  }

  .nav-links {
    display: flex;
    gap: 8px;
    direction: ltr;
  }

  .nav-link {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    border-radius: 8px;
    text-decoration: none;
    color: var(--color-text-light);
    font-size: 0.9rem;
    font-weight: 500;
    transition: all 0.15s ease;
    white-space: nowrap;
  }

  .nav-link:hover {
    background: var(--color-bg);
    color: var(--color-primary);
  }

  .nav-link.active {
    background: var(--color-primary);
    color: white;
  }

  .nav-link svg {
    flex-shrink: 0;
  }

  .app-main {
    flex: 1;
  }

  /* Footer */
  .site-footer {
    background: var(--color-surface);
    border-top: 1px solid var(--color-border);
    padding: 32px 24px 20px;
    direction: ltr;
  }

  .footer-inner {
    max-width: 1400px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 16px;
    text-align: center;
  }

  .footer-logo {
    font-family: 'Amiri', 'Noto Naskh Arabic', serif;
    font-size: 1.3rem;
    font-weight: 700;
    color: var(--color-primary);
  }

  .footer-tagline {
    color: var(--color-text-light);
    font-size: 0.85rem;
    margin-top: 4px;
    max-width: 500px;
  }

  .footer-links {
    display: flex;
    gap: 20px;
    flex-wrap: wrap;
    justify-content: center;
  }

  .footer-links a {
    color: var(--color-text-light);
    text-decoration: none;
    font-size: 0.85rem;
    transition: color 0.15s;
  }

  .footer-links a:hover {
    color: var(--color-primary);
  }

  .footer-copy {
    color: var(--color-text-light);
    font-size: 0.75rem;
    opacity: 0.7;
  }

  /* ─── Mobile Responsive ─── */
  @media (max-width: 768px) {
    .nav-inner {
      padding: 0 16px;
      height: 48px;
    }

    .nav-brand {
      font-size: 1.3rem;
    }

    /* Show hamburger on mobile */
    .menu-toggle {
      display: flex;
    }

    /* Nav links as dropdown on mobile */
    .nav-links {
      display: none;
      position: absolute;
      top: 100%;
      left: 0;
      right: 0;
      background: var(--color-surface);
      border-bottom: 1px solid var(--color-border);
      box-shadow: 0 4px 16px rgba(0,0,0,0.1);
      flex-direction: column;
      padding: 8px 16px 12px;
      gap: 4px;
      z-index: 99;
    }

    .nav-links.open {
      display: flex;
    }

    .nav-link {
      padding: 10px 14px;
      font-size: 0.9rem;
      border-radius: 8px;
    }

    .nav-link svg {
      width: 18px;
      height: 18px;
    }

    .nav-link.active {
      background: var(--color-primary);
      color: white;
    }

    /* Footer */
    .site-footer {
      padding: 20px 16px 16px;
    }

    .footer-tagline {
      font-size: 0.8rem;
    }

    .footer-links {
      gap: 12px;
    }

    .footer-links a {
      font-size: 0.8rem;
    }
  }
</style>
