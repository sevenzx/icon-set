<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { getSession, logout } from '$lib/api';
  import { authenticated } from '$lib/auth-state';
  import ToastHost from '$lib/ToastHost.svelte';
  import type { UserProfile } from '$lib/types';

  const repositoryUrl = 'https://github.com/sevenzx/icon-set';

  let sessionUser: UserProfile | null = null;
  let accountMenuOpen = false;

  $: accountName = sessionUser?.name || sessionUser?.login || '已登录用户';
  $: accountLogin = sessionUser?.login ? `@${sessionUser.login}` : 'GitHub 账号';
  $: avatarFallback = (sessionUser?.login || sessionUser?.name || 'U')
    .trim()
    .slice(0, 2)
    .toUpperCase();

  onMount(async () => {
    try {
      const session = await getSession();
      sessionUser = session.authenticated ? (session.user ?? null) : null;
    } catch {
      sessionUser = null;
      authenticated.set(false);
    }
  });

  function toggleAccountMenu() {
    accountMenuOpen = !accountMenuOpen;
  }

  function closeAccountMenu() {
    accountMenuOpen = false;
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      closeAccountMenu();
    }
  }

  async function submitLogout() {
    closeAccountMenu();
    await logout();
    sessionUser = null;
    await goto('/');
  }
</script>

<svelte:window on:click={closeAccountMenu} on:keydown={handleWindowKeydown} />

<svelte:head>
  <title>Icon Set Vault</title>
  <meta
    name="description"
    content="使用 GitHub raw.githubusercontent.com 管理和发布图标集合。"
  />
</svelte:head>

<div class="site-shell" class:has-account={$authenticated}>
  <header class="shell-topbar">
    <a class="brand-lockup" href="/" aria-label="返回图标集合首页">
      <span>IS</span>
    </a>
    <nav class="top-actions" aria-label="站点操作">
      {#if $authenticated}
        <div class="account-menu">
          <button
            class="account-button"
            type="button"
            aria-haspopup="menu"
            aria-expanded={accountMenuOpen}
            aria-label="打开账户菜单"
            title={accountName}
            on:click|stopPropagation={toggleAccountMenu}
          >
            {#if sessionUser?.avatar_url}
              <img src={sessionUser.avatar_url} alt="" referrerpolicy="no-referrer" />
            {:else}
              <span>{avatarFallback}</span>
            {/if}
          </button>

          {#if accountMenuOpen}
            <div class="account-popover" role="menu">
              <div class="account-meta">
                <strong>{accountName}</strong>
                <span>{accountLogin}</span>
              </div>
              <a href="/console" role="menuitem" on:click={closeAccountMenu}>进入控制台</a>
              <button type="button" role="menuitem" on:click={submitLogout}>退出登录</button>
            </div>
          {/if}
        </div>
      {:else}
        <a class="login-link" href="/console/login">登录</a>
      {/if}
      <a
        class="repo-link"
        href={repositoryUrl}
        target="_blank"
        rel="noreferrer"
        aria-label="打开 GitHub 仓库 sevenzx/icon-set"
        title="sevenzx/icon-set"
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="M12 .5a12 12 0 0 0-3.79 23.39c.6.11.82-.26.82-.58v-2.23c-3.34.73-4.04-1.42-4.04-1.42-.55-1.39-1.34-1.76-1.34-1.76-1.09-.75.08-.74.08-.74 1.2.09 1.84 1.24 1.84 1.24 1.07 1.83 2.81 1.3 3.49.99.11-.78.42-1.3.76-1.6-2.66-.3-5.47-1.33-5.47-5.93 0-1.31.47-2.38 1.24-3.22-.12-.3-.54-1.52.12-3.18 0 0 1.01-.32 3.3 1.23a11.5 11.5 0 0 1 6.01 0c2.29-1.55 3.3-1.23 3.3-1.23.66 1.66.24 2.88.12 3.18.77.84 1.23 1.91 1.23 3.22 0 4.61-2.81 5.62-5.49 5.92.43.37.82 1.1.82 2.22v3.3c0 .32.22.7.83.58A12 12 0 0 0 12 .5Z"
            fill="currentColor"
          />
        </svg>
      </a>
    </nav>
  </header>

  <main>
    <slot />
  </main>
</div>

<ToastHost />

<style>
  @import url('https://fonts.googleapis.com/css2?family=Bricolage+Grotesque:wght@600;700;800&family=IBM+Plex+Mono:wght@400;500;600;700&display=swap');

  :global(*) {
    box-sizing: border-box;
  }

  :global(html) {
    --shell-max: 1180px;
    --shell-pad: clamp(14px, 3vw, 30px);
    --topbar-top: 14px;
    --topbar-height: 44px;
    --topbar-gap: 10px;
    --topbar-edge: max(
      var(--shell-pad),
      calc((100vw - var(--shell-max)) / 2 + var(--shell-pad))
    );
    --topbar-logo-space: calc(var(--topbar-height) + var(--topbar-gap));
    --topbar-actions-space: 144px;
    color-scheme: dark;
    background: #0c0d0b;
  }

  :global(body) {
    margin: 0;
    min-width: 320px;
    min-height: 100vh;
    color: #f6efd9;
    font-family: 'IBM Plex Mono', ui-monospace, SFMono-Regular, Menlo, monospace;
    background:
      radial-gradient(circle at 16% 8%, rgba(255, 85, 36, 0.12), transparent 25rem),
      radial-gradient(circle at 86% 0%, rgba(198, 255, 72, 0.1), transparent 22rem),
      repeating-linear-gradient(90deg, rgba(246, 239, 217, 0.035) 0 1px, transparent 1px 72px),
      repeating-linear-gradient(0deg, rgba(246, 239, 217, 0.028) 0 1px, transparent 1px 72px),
      #0c0d0b;
  }

  :global(body::before) {
    position: fixed;
    inset: 0;
    z-index: -1;
    pointer-events: none;
    content: '';
    background-image: linear-gradient(rgba(246, 239, 217, 0.06) 1px, transparent 1px),
      linear-gradient(90deg, rgba(246, 239, 217, 0.06) 1px, transparent 1px);
    background-size: 16px 16px;
    mask-image: radial-gradient(circle at center, black, transparent 72%);
  }

  :global(a) {
    color: inherit;
    text-decoration: none;
  }

  :global(button),
  :global(input),
  :global(textarea),
  :global(select) {
    font: inherit;
  }

  :global(button) {
    cursor: pointer;
  }

  :global(.page-stack) {
    display: grid;
    gap: 20px;
  }

  :global(.eyebrow) {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    width: fit-content;
    padding: 8px 12px;
    border: 1px solid rgba(246, 239, 217, 0.24);
    border-radius: 10px;
    color: #c6ff48;
    background: rgba(12, 13, 11, 0.58);
    box-shadow: 0 0 0 1px rgba(198, 255, 72, 0.06) inset;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0;
    text-transform: uppercase;
  }

  :global(.breadcrumb) {
    position: fixed;
    top: var(--topbar-top);
    right: calc(var(--topbar-edge) + var(--topbar-actions-space));
    left: calc(var(--topbar-edge) + var(--topbar-logo-space));
    z-index: 110;
    display: flex;
    align-items: center;
    gap: 10px;
    height: var(--topbar-height);
    min-width: 0;
    padding: 0 14px;
    overflow: hidden;
    border: 1px solid rgba(246, 239, 217, 0.16);
    border-radius: 10px;
    color: rgba(246, 239, 217, 0.62);
    background:
      linear-gradient(90deg, rgba(198, 255, 72, 0.07), transparent 44%),
      rgba(12, 13, 11, 0.7);
    box-shadow: 0 14px 34px rgba(0, 0, 0, 0.2);
    backdrop-filter: blur(18px);
    font-size: 12px;
    font-weight: 800;
  }

  :global(.breadcrumb a) {
    flex: 0 0 auto;
    color: #c6ff48;
  }

  :global(.breadcrumb span) {
    flex: 0 0 auto;
  }

  :global(.breadcrumb strong) {
    min-width: 0;
    max-width: min(52vw, 520px);
    overflow: hidden;
    color: #f6efd9;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.hero) {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1.25fr) minmax(240px, 0.75fr);
    gap: clamp(18px, 4vw, 36px);
    min-height: 300px;
    padding: clamp(22px, 4vw, 42px);
    overflow: hidden;
    border: 1px solid rgba(246, 239, 217, 0.22);
    border-radius: 12px;
    background:
      linear-gradient(135deg, rgba(246, 239, 217, 0.06), transparent 48%),
      linear-gradient(162deg, rgba(255, 85, 36, 0.08), rgba(12, 13, 11, 0.78) 42%),
      #10110e;
    box-shadow: 0 18px 48px rgba(0, 0, 0, 0.28);
  }

  :global(.hero::after) {
    position: absolute;
    right: -70px;
    bottom: -100px;
    width: 270px;
    height: 270px;
    border: 1px solid rgba(198, 255, 72, 0.26);
    border-radius: 44% 56% 55% 45%;
    content: '';
    transform: rotate(-18deg);
  }

  :global(.hero-title) {
    max-width: 920px;
    margin: 0;
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: 104px;
    font-weight: 800;
    line-height: 0.86;
    letter-spacing: 0;
    text-transform: uppercase;
  }

  :global(.hero-title span) {
    color: #ff5524;
  }

  :global(.lead) {
    max-width: 760px;
    margin: 18px 0 0;
    color: rgba(246, 239, 217, 0.72);
    font-size: 15px;
    line-height: 1.75;
  }

  :global(.panel) {
    border: 1px solid rgba(246, 239, 217, 0.2);
    border-radius: 8px;
    background: rgba(16, 17, 14, 0.76);
    box-shadow: 0 16px 44px rgba(0, 0, 0, 0.22);
    backdrop-filter: blur(18px);
  }

  :global(.panel-pad) {
    padding: clamp(16px, 2.4vw, 26px);
  }

  :global(.grid) {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
    gap: 14px;
  }

  :global(.action) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    min-height: 40px;
    padding: 9px 14px;
    border: 1px solid rgba(246, 239, 217, 0.28);
    border-radius: 10px;
    color: #0c0d0b;
    background: #c6ff48;
    box-shadow: 3px 3px 0 rgba(255, 85, 36, 0.86);
    font-weight: 800;
    letter-spacing: 0;
    transition: border-color 160ms ease, background 160ms ease, color 160ms ease;
  }

  :global(.action:hover) {
    border-color: rgba(198, 255, 72, 0.58);
    color: #0c0d0b;
    background: #f6efd9;
  }

  :global(.action.secondary) {
    color: #f6efd9;
    background: rgba(246, 239, 217, 0.08);
    box-shadow: none;
  }

  :global(.action.danger) {
    color: #f6efd9;
    background: rgba(255, 85, 36, 0.26);
    box-shadow: none;
  }

  :global(.field) {
    display: grid;
    gap: 8px;
  }

  :global(.field span) {
    color: rgba(246, 239, 217, 0.7);
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0;
    text-transform: uppercase;
  }

  :global(.input),
  :global(.textarea) {
    width: 100%;
    border: 1px solid rgba(246, 239, 217, 0.22);
    border-radius: 8px;
    color: #f6efd9;
    background: rgba(12, 13, 11, 0.72);
    outline: none;
  }

  :global(.input) {
    min-height: 44px;
    padding: 0 16px;
  }

  :global(.textarea) {
    min-height: 106px;
    padding: 14px 16px;
    resize: vertical;
  }

  :global(.input:focus),
  :global(.textarea:focus) {
    border-color: rgba(198, 255, 72, 0.74);
    box-shadow: 0 0 0 4px rgba(198, 255, 72, 0.1);
  }

  :global(.notice) {
    padding: 14px 16px;
    border: 1px solid rgba(198, 255, 72, 0.22);
    border-radius: 8px;
    color: rgba(246, 239, 217, 0.82);
    background: rgba(198, 255, 72, 0.08);
    line-height: 1.6;
  }

  :global(.error) {
    border-color: rgba(255, 85, 36, 0.5);
    color: #ffd8c9;
    background: rgba(255, 85, 36, 0.12);
  }

  .site-shell {
    width: min(var(--shell-max), 100%);
    margin: 0 auto;
    padding: calc(var(--topbar-top) + var(--topbar-height) + 28px) var(--shell-pad) 56px;
  }

  .site-shell.has-account {
    --topbar-actions-space: 104px;
  }

  .shell-topbar {
    position: fixed;
    top: var(--topbar-top);
    right: var(--topbar-edge);
    left: var(--topbar-edge);
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--topbar-gap);
    height: var(--topbar-height);
    pointer-events: none;
  }

  .shell-topbar::before {
    position: absolute;
    inset: 0;
    z-index: -1;
    border: 1px solid rgba(246, 239, 217, 0.08);
    border-radius: 14px;
    content: '';
    background:
      linear-gradient(90deg, rgba(255, 85, 36, 0.09), transparent 32%),
      linear-gradient(90deg, transparent 56%, rgba(198, 255, 72, 0.08)),
      rgba(12, 13, 11, 0.58);
    box-shadow: 0 18px 46px rgba(0, 0, 0, 0.24);
    backdrop-filter: blur(20px);
  }

  .brand-lockup,
  .login-link,
  .account-button,
  .repo-link {
    display: inline-flex;
    align-items: center;
    min-height: var(--topbar-height);
    border: 1px solid rgba(246, 239, 217, 0.16);
    border-radius: 10px;
    background: rgba(12, 13, 11, 0.58);
    font-size: 12px;
    font-weight: 800;
    pointer-events: auto;
  }

  .brand-lockup {
    justify-content: center;
    width: var(--topbar-height);
    padding: 4px;
  }

  .brand-lockup span {
    display: grid;
    width: 32px;
    height: 32px;
    place-items: center;
    border-radius: 10px;
    color: #0c0d0b;
    background: #f6efd9;
    box-shadow: 3px 3px 0 rgba(255, 85, 36, 0.9);
  }

  .top-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    pointer-events: auto;
  }

  .login-link {
    justify-content: center;
    min-width: 82px;
    padding: 0 14px;
    color: #f6efd9;
  }

  .account-menu {
    position: relative;
    pointer-events: auto;
  }

  .account-button {
    justify-content: center;
    width: var(--topbar-height);
    padding: 4px;
    color: #0c0d0b;
  }

  .account-button img,
  .account-button span {
    display: grid;
    width: 32px;
    height: 32px;
    place-items: center;
    border-radius: 10px;
  }

  .account-button img {
    object-fit: cover;
  }

  .account-button span {
    background: #f6efd9;
    font-size: 12px;
    font-weight: 900;
  }

  .account-popover {
    position: absolute;
    top: calc(100% + 8px);
    right: 0;
    z-index: 130;
    display: grid;
    gap: 6px;
    width: 196px;
    padding: 8px;
    border: 1px solid rgba(246, 239, 217, 0.18);
    border-radius: 12px;
    color: #f6efd9;
    background:
      linear-gradient(135deg, rgba(198, 255, 72, 0.08), transparent 58%),
      rgba(12, 13, 11, 0.92);
    box-shadow: 0 18px 44px rgba(0, 0, 0, 0.36);
    backdrop-filter: blur(18px);
  }

  .account-popover::before {
    position: absolute;
    top: -5px;
    right: 17px;
    width: 9px;
    height: 9px;
    border-top: 1px solid rgba(246, 239, 217, 0.18);
    border-left: 1px solid rgba(246, 239, 217, 0.18);
    content: '';
    background: rgba(12, 13, 11, 0.92);
    transform: rotate(45deg);
  }

  .account-meta {
    display: grid;
    gap: 2px;
    padding: 8px 9px 10px;
    border-bottom: 1px solid rgba(246, 239, 217, 0.12);
  }

  .account-meta strong,
  .account-meta span {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .account-meta strong {
    color: #f6efd9;
    font-size: 13px;
  }

  .account-meta span {
    color: rgba(246, 239, 217, 0.55);
    font-size: 11px;
  }

  .account-popover a,
  .account-popover button {
    display: flex;
    align-items: center;
    min-height: 36px;
    width: 100%;
    padding: 0 10px;
    border: 0;
    border-radius: 8px;
    color: rgba(246, 239, 217, 0.88);
    background: transparent;
    font-size: 12px;
    font-weight: 800;
    text-align: left;
  }

  .account-popover a:hover,
  .account-popover button:hover {
    color: #0c0d0b;
    background: #c6ff48;
  }

  .repo-link {
    justify-content: center;
    width: 40px;
    padding: 0;
    color: #c6ff48;
  }

  .repo-link svg {
    width: 19px;
    height: 19px;
  }

  .repo-link:hover,
  .login-link:hover,
  .account-button:hover,
  .account-button[aria-expanded='true'],
  .brand-lockup:hover {
    border-color: rgba(198, 255, 72, 0.42);
    background: rgba(24, 26, 20, 0.78);
  }

  main {
    display: grid;
    gap: 18px;
  }

  @media (max-width: 760px) {
    :global(html) {
      --topbar-actions-space: 116px;
    }

    .site-shell.has-account {
      --topbar-actions-space: 96px;
    }

    :global(.breadcrumb) {
      gap: 8px;
      padding: 0 12px;
    }

    :global(.breadcrumb a:first-of-type),
    :global(.breadcrumb a:first-of-type + span) {
      display: none;
    }

    :global(.hero) {
      grid-template-columns: 1fr;
      min-height: auto;
    }

    :global(.hero-title) {
      font-size: 68px;
    }

    .login-link {
      min-width: 54px;
      padding: 0 10px;
    }

  }

  @media (max-width: 460px) {
    :global(html) {
      --topbar-top: 10px;
      --shell-pad: 12px;
      --topbar-gap: 8px;
      --topbar-actions-space: 104px;
    }

    .site-shell.has-account {
      --topbar-actions-space: 96px;
    }

    :global(.breadcrumb) {
      padding: 0 10px;
      font-size: 11px;
    }

    :global(.breadcrumb strong) {
      max-width: none;
    }

    .shell-topbar {
      gap: 10px;
    }

    .top-actions {
      gap: 6px;
    }

    .login-link {
      min-width: 44px;
      padding: 0 9px;
    }

    .repo-link {
      width: 40px;
      flex: 0 0 40px;
    }

    :global(.hero-title) {
      font-size: 54px;
    }
  }
</style>
