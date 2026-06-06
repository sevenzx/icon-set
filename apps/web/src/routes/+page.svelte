<script lang="ts">
  import { CircleAlert, Link2, Share2 } from '@lucide/svelte';
  import { onMount } from 'svelte';
  import { manifestRawUrl, sharePageUrl } from '$lib/asset-url';
  import { copyText } from '$lib/clipboard';
  import { getSession, listSets } from '$lib/api';
  import { renderMarkdown } from '$lib/markdown';
  import { toast } from '$lib/toast';
  import type { IconSetSummary, RepoConfig } from '$lib/types';

  let sets: IconSetSummary[] = [];
  let repoConfig: RepoConfig | null = null;
  let introHtml = '';
  let introError = '';
  let sessionLoaded = false;
  let isAuthenticated = false;
  let loading = true;
  let error = '';
  let failedSetId = '';
  let failedShareSetId = '';

  $: showGuestIntro = sessionLoaded && !isAuthenticated;

  /// 刷新当前登录用户的仓库配置，用于复制 GitHub Raw manifest 地址。
  async function refreshRepoConfig() {
    try {
      const session = await getSession();
      isAuthenticated = session.authenticated;
      repoConfig = isAuthenticated ? (session.repo_config ?? null) : null;
    } catch {
      isAuthenticated = false;
      repoConfig = null;
    } finally {
      sessionLoaded = true;
    }
  }

  /// 加载未登录用户看到的网站功能介绍。
  async function refreshIntro() {
    introError = '';

    try {
      const response = await fetch('/docs/site-intro.md');
      if (!response.ok) {
        throw new Error(`网站介绍加载失败：${response.status}`);
      }
      introHtml = renderMarkdown(await response.text());
    } catch (err) {
      introHtml = '';
      introError = err instanceof Error ? err.message : '网站介绍加载失败';
    }
  }

  /// 加载图标集合列表。
  async function refreshSets() {
    loading = true;
    error = '';

    try {
      sets = await listSets();
    } catch (err) {
      error = err instanceof Error ? err.message : '图标集合加载失败';
    } finally {
      loading = false;
    }
  }

  /// 生成集合卡片的两位视觉标记。
  function setMark(name: string) {
    return name
      .replace(/[^a-zA-Z0-9\u4e00-\u9fa5]/g, '')
      .slice(0, 2)
      .toUpperCase();
  }

  /// 复制集合 manifest.json 的 raw 地址。
  async function copySetUrl(setId: string) {
    const url = manifestRawUrl(setId, repoConfig);
    failedSetId = '';

    try {
      await copyText(url);
      toast.info('集合地址已复制');
    } catch {
      failedSetId = setId;
      toast.error('复制集合地址失败');
      window.setTimeout(() => {
        if (failedSetId === setId) failedSetId = '';
      }, 1600);
    }
  }

  /// 复制集合分享页地址。
  async function copyShareSetUrl(setId: string) {
    const url = sharePageUrl(manifestRawUrl(setId, repoConfig));
    failedShareSetId = '';

    try {
      await copyText(url);
      toast.info('分享链接已复制');
    } catch {
      failedShareSetId = setId;
      toast.error('复制分享链接失败');
      window.setTimeout(() => {
        if (failedShareSetId === setId) failedShareSetId = '';
      }, 1600);
    }
  }

  onMount(() => {
    void (async () => {
      await Promise.all([refreshRepoConfig(), refreshSets(), refreshIntro()]);
    })();

    const handleAuthChanged = () => {
      void (async () => {
        await refreshRepoConfig();
        await refreshSets();
      })();
    };
    window.addEventListener('icon-set:auth-changed', handleAuthChanged);

    return () => {
      window.removeEventListener('icon-set:auth-changed', handleAuthChanged);
    };
  });
</script>

<section class="hero">
  <div>
    <span class="eyebrow">Raw GitHub Icon Library</span>
    <h1 class="hero-title">ICON<br /><span>SETS</span></h1>
    <p class="lead">为常用服务整理干净、可搜索、可复制链接的图标集合。</p>
  </div>

  <aside class="hero-card panel panel-pad">
    <strong>{sets.length}</strong>
    <span>active sets</span>
  </aside>
</section>

{#if showGuestIntro}
  <section class="guest-intro panel panel-pad" aria-label="网站功能介绍">
    {#if introError}
      <div class="notice error">{introError}</div>
    {:else if !introHtml}
      <div class="notice">正在加载网站功能介绍...</div>
    {:else}
      <div class="markdown-body guest-markdown">
        {@html introHtml}
      </div>
    {/if}
  </section>
{/if}

<section class="page-stack">
  <div class="section-head">
    <div>
      <span class="eyebrow">Collections</span>
      <h2>图标集合</h2>
    </div>
  </div>

  {#if loading}
    <div class="notice">正在从 GitHub 读取 sets.json...</div>
  {:else if error}
    <div class="notice error">{error}</div>
  {:else if sets.length === 0}
    <div class="empty panel panel-pad">
      <span class="eyebrow">No sets yet</span>
      <h3>还没有任何图标集合</h3>
      <p>这里会展示已经发布的图标集合。</p>
    </div>
  {:else}
    <div class="grid">
      {#each sets as set}
        <article class="set-card panel">
          <a class="set-link" href={`/sets/${set.id}`} title="打开集合">
            <span class="set-mark">{setMark(set.name)}</span>
            <span class="set-id">/{set.id}</span>
            <h3>{set.name}</h3>
            <p>{set.description || '这个集合还没有描述。'}</p>
          </a>
          <footer>
            <span>{set.icon_count} icons</span>
            <div class="card-actions">
              <button
                class="copy-set compact-action"
                type="button"
                title="复制 manifest.json 地址"
                on:click={() => copySetUrl(set.id)}
              >
                {#if failedSetId === set.id}
                  <CircleAlert size={15} strokeWidth={2.2} />
                {:else}
                  <Link2 size={15} strokeWidth={2.2} />
                {/if}
                {#if failedSetId === set.id}
                  复制失败
                {:else}
                  Manifest
                {/if}
              </button>
              <button
                class="copy-set compact-action"
                type="button"
                title="复制分享链接"
                on:click={() => copyShareSetUrl(set.id)}
              >
                {#if failedShareSetId === set.id}
                  <CircleAlert size={15} strokeWidth={2.2} />
                {:else}
                  <Share2 size={15} strokeWidth={2.2} />
                {/if}
                {#if failedShareSetId === set.id}
                  复制失败
                {:else}
                  分享
                {/if}
              </button>
            </div>
          </footer>
        </article>
      {/each}
    </div>
  {/if}
</section>

<style>
  .page-stack {
    display: grid;
    gap: 20px;
  }

  .guest-intro {
    margin: 20px 0;
    overflow: hidden;
    border-color: rgba(198, 255, 72, 0.24);
    background:
      linear-gradient(135deg, rgba(198, 255, 72, 0.07), transparent 42%),
      rgba(16, 17, 14, 0.76);
  }

  .guest-markdown {
    max-width: 900px;
  }

  .hero-card {
    align-self: end;
    display: grid;
    align-content: end;
    gap: 14px;
    min-height: 168px;
    border-color: rgba(198, 255, 72, 0.28);
  }

  .hero-card strong {
    color: #ff5524;
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: 82px;
    line-height: 0.8;
  }

  .hero-card span:not(.eyebrow) {
    color: rgba(246, 239, 217, 0.62);
  }

  .section-head {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: 14px;
  }

  h2,
  h3 {
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    letter-spacing: 0;
  }

  h2 {
    margin: 10px 0 0;
    font-size: 54px;
    line-height: 0.94;
  }

  .set-card {
    position: relative;
    display: grid;
    min-height: 230px;
    padding: 18px;
    overflow: hidden;
    transition:
      border-color 180ms ease,
      background 180ms ease;
  }

  .set-card::before {
    position: absolute;
    inset: auto -46px -88px auto;
    width: 150px;
    height: 150px;
    border: 1px solid rgba(255, 85, 36, 0.24);
    border-radius: 8px;
    content: '';
    transform: rotate(20deg);
  }

  .set-card:hover {
    border-color: rgba(198, 255, 72, 0.5);
    background: rgba(24, 26, 20, 0.88);
  }

  .set-link {
    position: relative;
    z-index: 1;
    display: grid;
  }

  .set-mark {
    display: grid;
    width: 54px;
    height: 54px;
    place-items: center;
    border-radius: 14px;
    color: #0c0d0b;
    background: #f6efd9;
    box-shadow: 4px 4px 0 rgba(255, 85, 36, 0.86);
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: 22px;
    font-weight: 800;
  }

  .set-id {
    position: absolute;
    top: 18px;
    right: 18px;
    color: rgba(198, 255, 72, 0.78);
    font-size: 12px;
  }

  .set-card h3 {
    align-self: end;
    margin: 34px 0 8px;
    font-size: 26px;
    line-height: 1;
  }

  .set-card p {
    display: -webkit-box;
    min-height: 54px;
    margin: 0;
    overflow: hidden;
    color: rgba(246, 239, 217, 0.66);
    line-height: 1.6;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 3;
    line-clamp: 3;
  }

  .set-card footer {
    position: relative;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-top: 20px;
    color: rgba(246, 239, 217, 0.6);
    font-size: 13px;
  }

  .card-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .copy-set {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 7px;
    min-height: 34px;
    padding: 0 12px;
    border: 1px solid rgba(246, 239, 217, 0.2);
    border-radius: 10px;
    color: #f6efd9;
    background: rgba(246, 239, 217, 0.08);
    font-size: 12px;
    font-weight: 800;
    transition:
      border-color 160ms ease,
      background 160ms ease,
      color 160ms ease;
  }

  .copy-set:hover {
    border-color: rgba(198, 255, 72, 0.5);
    color: #0c0d0b;
    background: #c6ff48;
  }

  .compact-action {
    letter-spacing: 0.04em;
  }

  .compact-action :global(svg) {
    flex: 0 0 auto;
  }

  .empty {
    display: grid;
    gap: 16px;
    max-width: 720px;
  }

  .empty h3 {
    margin: 0;
    font-size: 38px;
  }

  .empty p {
    margin: 0;
    color: rgba(246, 239, 217, 0.7);
    line-height: 1.8;
  }

  @media (max-width: 700px) {
    .section-head {
      align-items: stretch;
      flex-direction: column;
    }

    h2 {
      font-size: 40px;
    }
  }
</style>
