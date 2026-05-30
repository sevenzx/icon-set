<script lang="ts">
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import { manifestRawUrl } from '$lib/asset-url';
  import { getSet } from '$lib/api';
  import { copyText } from '$lib/clipboard';
  import { toast } from '$lib/toast';
  import type { IconEntry, IconManifest } from '$lib/types';

  let manifest: IconManifest | null = null;
  let loading = true;
  let error = '';
  let query = '';
  let copiedUrl = '';
  let failedUrl = '';

  $: filteredIcons = filterIcons(manifest?.icons ?? [], query);
  $: manifestUrl = manifest ? manifestRawUrl(manifest.id) : '';

  /// 读取当前路由对应的图标集合。
  async function refreshManifest() {
    loading = true;
    error = '';
    const setId = $page.params.setId;

    if (!setId) {
      error = '缺少集合 ID';
      loading = false;
      return;
    }

    try {
      manifest = await getSet(setId);
    } catch (err) {
      error = err instanceof Error ? err.message : '图标集合加载失败';
    } finally {
      loading = false;
    }
  }

  /// 根据搜索词过滤图标名称。
  function filterIcons(icons: IconEntry[], keyword: string) {
    const normalized = keyword.trim().toLowerCase();
    if (!normalized) return icons;
    return icons.filter((icon) => icon.name.toLowerCase().includes(normalized));
  }

  /// 复制图标 raw 地址到剪贴板。
  async function copyUrl(url: string) {
    copiedUrl = '';
    failedUrl = '';

    try {
      await copyText(url);
      copiedUrl = url;
      toast.info('地址已复制');
      window.setTimeout(() => {
        if (copiedUrl === url) copiedUrl = '';
      }, 1600);
    } catch {
      failedUrl = url;
      toast.error('复制地址失败');
      window.setTimeout(() => {
        if (failedUrl === url) failedUrl = '';
      }, 1600);
    }
  }

  onMount(() => {
    void refreshManifest();
  });
</script>

{#if loading}
  <div class="notice">正在读取 manifest.json...</div>
{:else if error}
  <div class="notice error">{error}</div>
{:else if manifest}
  <nav class="breadcrumb" aria-label="面包屑">
    <a href="/">图标集合</a>
    <span>/</span>
    <strong>{manifest.name}</strong>
  </nav>

  <section class="set-hero panel panel-pad">
    <div>
      <span class="eyebrow">/{manifest.id}</span>
      <h1>{manifest.name}</h1>
      <p>{manifest.description || '这个集合还没有描述。'}</p>
    </div>
    <div class="hero-side">
      <div class="stat-stack">
        <strong>{manifest.icons.length}</strong>
        <span>icons in manifest</span>
      </div>
      <button class="action secondary manifest-copy" type="button" on:click={() => copyUrl(manifestUrl)}>
        {#if copiedUrl === manifestUrl}
          已复制 Manifest
        {:else if failedUrl === manifestUrl}
          复制失败
        {:else}
          复制 Manifest URL
        {/if}
      </button>
    </div>
  </section>

  <section class="toolbar panel panel-pad">
    <label class="field search-field">
      <span>搜索图标</span>
      <input class="input" bind:value={query} placeholder="输入名称，例如 Emby" />
    </label>
    <div class="result-count" aria-live="polite">
      <strong>{filteredIcons.length}</strong>
      <span>/ {manifest.icons.length} icons</span>
    </div>
  </section>

  {#if filteredIcons.length === 0}
    <div class="notice">没有匹配的图标。</div>
  {:else}
    <section class="icon-grid">
      {#each filteredIcons as icon}
        <article class="icon-card panel">
          <div class="icon-stage">
            <img src={icon.url} alt={icon.name} loading="lazy" />
          </div>
          <div class="icon-meta">
            <h2>{icon.name}</h2>
            <code>{icon.path || icon.url}</code>
          </div>
          <button class="action secondary" type="button" on:click={() => copyUrl(icon.url)}>
            {#if copiedUrl === icon.url}
              已复制
            {:else if failedUrl === icon.url}
              复制失败
            {:else}
              复制 Raw URL
            {/if}
          </button>
        </article>
      {/each}
    </section>
  {/if}
{/if}

<style>
  .set-hero {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 28px;
    overflow: hidden;
  }

  h1 {
    max-width: 900px;
    margin: 14px 0 0;
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: clamp(44px, 8vw, 104px);
    line-height: 0.9;
    letter-spacing: -0.07em;
  }

  .set-hero p {
    max-width: 780px;
    margin: 18px 0 0;
    color: rgba(246, 239, 217, 0.72);
    line-height: 1.8;
  }

  .hero-side {
    display: grid;
    gap: 14px;
    min-width: 220px;
    align-self: stretch;
  }

  .stat-stack {
    display: grid;
    place-items: center;
    align-self: stretch;
    border: 1px solid rgba(198, 255, 72, 0.28);
    border-radius: 24px;
    background: rgba(198, 255, 72, 0.08);
  }

  .stat-stack strong {
    color: #ff5524;
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: 74px;
    line-height: 0.9;
  }

  .stat-stack span {
    max-width: 130px;
    color: rgba(246, 239, 217, 0.62);
    text-align: center;
  }

  .manifest-copy {
    width: 100%;
    min-height: 46px;
  }

  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }

  .search-field {
    flex: 1;
  }

  .result-count {
    display: grid;
    min-width: 120px;
    justify-items: end;
    color: rgba(246, 239, 217, 0.58);
    font-size: 12px;
  }

  .result-count strong {
    color: #c6ff48;
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: 34px;
    line-height: 0.9;
  }

  .icon-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
    gap: 16px;
  }

  .icon-card {
    display: grid;
    gap: 14px;
    padding: 14px;
    transition: border-color 160ms ease, background 160ms ease;
  }

  .icon-card:hover {
    border-color: rgba(198, 255, 72, 0.5);
    background: rgba(24, 26, 20, 0.82);
  }

  .icon-stage {
    display: grid;
    min-height: 170px;
    place-items: center;
    border: 1px solid rgba(246, 239, 217, 0.15);
    border-radius: 20px;
    background:
      linear-gradient(45deg, rgba(246, 239, 217, 0.06) 25%, transparent 25%),
      linear-gradient(-45deg, rgba(246, 239, 217, 0.06) 25%, transparent 25%),
      rgba(246, 239, 217, 0.04);
    background-position: 0 0, 0 8px;
    background-size: 16px 16px;
  }

  .icon-stage img {
    max-width: 96px;
    max-height: 96px;
    object-fit: contain;
    filter: drop-shadow(0 12px 22px rgba(0, 0, 0, 0.32));
  }

  .icon-meta {
    display: grid;
    gap: 8px;
    min-width: 0;
  }

  .icon-meta h2 {
    margin: 0;
    overflow: hidden;
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: 24px;
    letter-spacing: -0.04em;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .icon-meta code {
    overflow: hidden;
    color: rgba(246, 239, 217, 0.5);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  @media (max-width: 720px) {
    .set-hero,
    .toolbar {
      grid-template-columns: 1fr;
    }

    .toolbar {
      align-items: stretch;
      flex-direction: column;
    }

    .hero-side {
      min-width: 0;
    }

    .stat-stack {
      min-height: 160px;
    }

    .result-count {
      justify-items: start;
    }
  }
</style>
