<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import '../app.css';
  import { getSession, logout } from '$lib/api';
  import { authenticated } from '$lib/auth-state';
  import ToastHost from '$lib/ToastHost.svelte';
  import type { UserProfile } from '$lib/types';

  const repositoryUrl = 'https://github.com/sevenzx/icon-set';

  let sessionUser: UserProfile | null = null;
  let accountMenuOpen = false;

  $: accountName = sessionUser?.name || sessionUser?.login || '已登录用户';
  $: accountLogin = sessionUser?.login
    ? `@${sessionUser.login}`
    : 'GitHub 账号';
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
              <img
                src={sessionUser.avatar_url}
                alt=""
                referrerpolicy="no-referrer"
              />
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
              <a href="/console" role="menuitem" on:click={closeAccountMenu}
                >进入控制台</a
              >
              <button type="button" role="menuitem" on:click={submitLogout}
                >退出登录</button
              >
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
  .site-shell {
    width: min(var(--shell-max), 100%);
    margin: 0 auto;
    padding: calc(var(--topbar-top) + var(--topbar-height) + 28px)
      var(--shell-pad) 56px;
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
    .site-shell.has-account {
      --topbar-actions-space: 96px;
    }

    .login-link {
      min-width: 54px;
      padding: 0 10px;
    }
  }

  @media (max-width: 460px) {
    .site-shell.has-account {
      --topbar-actions-space: 96px;
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
  }
</style>
