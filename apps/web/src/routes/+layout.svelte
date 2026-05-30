<script lang="ts">
  import ToastHost from '$lib/ToastHost.svelte';
</script>

<svelte:head>
  <title>Icon Set Vault</title>
  <meta
    name="description"
    content="使用 GitHub raw.githubusercontent.com 管理和发布图标集合。"
  />
</svelte:head>

<div class="site-shell">
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
      radial-gradient(circle at 16% 12%, rgba(255, 85, 36, 0.22), transparent 28rem),
      radial-gradient(circle at 88% 0%, rgba(198, 255, 72, 0.16), transparent 24rem),
      repeating-linear-gradient(90deg, rgba(246, 239, 217, 0.04) 0 1px, transparent 1px 72px),
      repeating-linear-gradient(0deg, rgba(246, 239, 217, 0.035) 0 1px, transparent 1px 72px),
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
    gap: 28px;
  }

  :global(.eyebrow) {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    width: fit-content;
    padding: 8px 12px;
    border: 1px solid rgba(246, 239, 217, 0.24);
    border-radius: 999px;
    color: #c6ff48;
    background: rgba(12, 13, 11, 0.58);
    box-shadow: 0 0 0 1px rgba(198, 255, 72, 0.06) inset;
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  :global(.breadcrumb) {
    display: flex;
    align-items: center;
    gap: 10px;
    width: fit-content;
    padding: 10px 12px;
    border: 1px solid rgba(246, 239, 217, 0.16);
    border-radius: 999px;
    color: rgba(246, 239, 217, 0.62);
    background: rgba(12, 13, 11, 0.58);
    font-size: 12px;
    font-weight: 800;
  }

  :global(.breadcrumb a) {
    color: #c6ff48;
  }

  :global(.breadcrumb strong) {
    max-width: min(52vw, 520px);
    overflow: hidden;
    color: #f6efd9;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.hero) {
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1.3fr) minmax(260px, 0.7fr);
    gap: 28px;
    min-height: 420px;
    padding: clamp(24px, 5vw, 60px);
    overflow: hidden;
    border: 1px solid rgba(246, 239, 217, 0.22);
    border-radius: 32px;
    background:
      linear-gradient(135deg, rgba(246, 239, 217, 0.08), transparent 48%),
      linear-gradient(162deg, rgba(255, 85, 36, 0.14), rgba(12, 13, 11, 0.72) 42%),
      #10110e;
    box-shadow: 0 32px 80px rgba(0, 0, 0, 0.38);
  }

  :global(.hero::after) {
    position: absolute;
    right: -80px;
    bottom: -110px;
    width: 360px;
    height: 360px;
    border: 1px solid rgba(198, 255, 72, 0.4);
    border-radius: 44% 56% 55% 45%;
    content: '';
    transform: rotate(-18deg);
  }

  :global(.hero-title) {
    max-width: 920px;
    margin: 0;
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: clamp(54px, 11vw, 136px);
    font-weight: 800;
    line-height: 0.86;
    letter-spacing: -0.08em;
    text-transform: uppercase;
  }

  :global(.hero-title span) {
    color: #ff5524;
  }

  :global(.lead) {
    max-width: 760px;
    margin: 20px 0 0;
    color: rgba(246, 239, 217, 0.76);
    font-size: clamp(15px, 1.8vw, 18px);
    line-height: 1.85;
  }

  :global(.panel) {
    border: 1px solid rgba(246, 239, 217, 0.2);
    border-radius: 26px;
    background: rgba(16, 17, 14, 0.78);
    box-shadow: 0 24px 70px rgba(0, 0, 0, 0.28);
    backdrop-filter: blur(18px);
  }

  :global(.panel-pad) {
    padding: clamp(18px, 3vw, 30px);
  }

  :global(.grid) {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(230px, 1fr));
    gap: 18px;
  }

  :global(.action) {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 10px;
    min-height: 44px;
    padding: 11px 16px;
    border: 1px solid rgba(246, 239, 217, 0.28);
    border-radius: 999px;
    color: #0c0d0b;
    background: #c6ff48;
    box-shadow: 5px 5px 0 #ff5524;
    font-weight: 800;
    letter-spacing: -0.03em;
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
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  :global(.input),
  :global(.textarea) {
    width: 100%;
    border: 1px solid rgba(246, 239, 217, 0.22);
    border-radius: 18px;
    color: #f6efd9;
    background: rgba(12, 13, 11, 0.72);
    outline: none;
  }

  :global(.input) {
    min-height: 48px;
    padding: 0 16px;
  }

  :global(.textarea) {
    min-height: 116px;
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
    border-radius: 18px;
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
    width: min(1440px, 100%);
    margin: 0 auto;
    padding: 18px clamp(16px, 3vw, 36px) 60px;
  }

  main {
    display: grid;
    gap: 24px;
  }

  @media (max-width: 760px) {
    :global(.hero) {
      grid-template-columns: 1fr;
      min-height: auto;
    }
  }
</style>
