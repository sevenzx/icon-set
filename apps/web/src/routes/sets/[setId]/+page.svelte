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
  let previewIcon: IconEntry | null = null;
  let pageSize = 24;
  let currentPage = 1;
  let previousQuery = '';
  let previousPageSize = pageSize;

  const pageSizeOptions = [24, 48, 96];

  $: filteredIcons = filterIcons(manifest?.icons ?? [], query);
  $: manifestUrl = manifest ? manifestRawUrl(manifest.id) : '';
  $: showPaginationControls =
    (manifest?.icons.length ?? 0) > pageSizeOptions[0];
  $: totalPages = Math.max(1, Math.ceil(filteredIcons.length / pageSize));
  $: if (query !== previousQuery || pageSize !== previousPageSize) {
    currentPage = 1;
    previousQuery = query;
    previousPageSize = pageSize;
  }
  $: if (currentPage > totalPages) {
    currentPage = totalPages;
  }
  $: pageStart = (currentPage - 1) * pageSize;
  $: pageEnd = Math.min(pageStart + pageSize, filteredIcons.length);
  $: pagedIcons = filteredIcons.slice(pageStart, pageEnd);

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

  function openPreview(icon: IconEntry) {
    previewIcon = icon;
  }

  function closePreview() {
    previewIcon = null;
  }

  function copyPreviewUrl() {
    if (!previewIcon) return;
    void copyUrl(previewIcon.url);
  }

  function goToPage(page: number) {
    currentPage = Math.min(Math.max(page, 1), totalPages);
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && previewIcon) {
      closePreview();
    }
  }

  function handlePreviewBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      closePreview();
    }
  }

  onMount(() => {
    void refreshManifest();

    const handleAuthChanged = () => {
      void refreshManifest();
    };
    window.addEventListener('icon-set:auth-changed', handleAuthChanged);

    return () => {
      window.removeEventListener('icon-set:auth-changed', handleAuthChanged);
    };
  });
</script>

<svelte:window on:keydown={handleKeydown} />

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
        <span>icons</span>
      </div>
      <button
        class="action secondary manifest-copy"
        type="button"
        on:click={() => copyUrl(manifestUrl)}
      >
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
      <input class="input" bind:value={query} placeholder="输入名称" />
    </label>
  </section>

  {#if filteredIcons.length === 0}
    <div class="notice">没有匹配的图标。</div>
  {:else}
    <section class="icon-grid">
      {#each pagedIcons as icon}
        <article class="icon-card panel">
          <button
            class:copied={copiedUrl === icon.url}
            class:failed={failedUrl === icon.url}
            class="copy-url"
            type="button"
            aria-label={`复制 ${icon.name} 的 Raw URL`}
            title="复制 Raw URL"
            on:click={() => copyUrl(icon.url)}
          >
            {#if copiedUrl === icon.url}
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="m5 12 4 4L19 6" />
              </svg>
            {:else if failedUrl === icon.url}
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M12 7v6" />
                <path d="M12 17h.01" />
              </svg>
            {:else}
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <rect x="8" y="8" width="10" height="10" rx="2" />
                <path
                  d="M6 14H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h7a2 2 0 0 1 2 2v1"
                />
              </svg>
            {/if}
          </button>
          <div class="icon-stage">
            <button
              class="image-preview-button"
              type="button"
              aria-label={`查看 ${icon.name} 大图`}
              title="查看大图"
              on:click={() => openPreview(icon)}
            >
              <img src={icon.url} alt={icon.name} loading="lazy" />
            </button>
          </div>
          <div class="icon-meta">
            <h2>{icon.name}</h2>
            <code>{icon.path || icon.url}</code>
          </div>
        </article>
      {/each}
    </section>

    {#if showPaginationControls}
      <nav class="pagination" aria-label="图标分页">
        <span class="pagination-summary">
          <strong>{pageStart + 1}-{pageEnd}</strong>
          / {filteredIcons.length} icons
        </span>
        <label class="page-size-control">
          <span>每页</span>
          <select class="input page-size-select" bind:value={pageSize}>
            {#each pageSizeOptions as option}
              <option value={option}>{option} 个</option>
            {/each}
          </select>
        </label>
        {#if totalPages > 1}
          <div class="pagination-actions">
            <button
              class="page-button"
              type="button"
              aria-label="上一页"
              disabled={currentPage === 1}
              on:click={() => goToPage(currentPage - 1)}
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="m15 18-6-6 6-6" />
              </svg>
            </button>
            <strong class="page-indicator">{currentPage} / {totalPages}</strong>
            <button
              class="page-button"
              type="button"
              aria-label="下一页"
              disabled={currentPage === totalPages}
              on:click={() => goToPage(currentPage + 1)}
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="m9 18 6-6-6-6" />
              </svg>
            </button>
          </div>
        {/if}
      </nav>
    {/if}
  {/if}
{/if}

{#if previewIcon}
  <div
    class="image-preview-backdrop"
    role="presentation"
    on:click={handlePreviewBackdropClick}
  >
    <div
      class="image-preview"
      role="dialog"
      aria-modal="true"
      aria-labelledby="image-preview-title"
    >
      <header>
        <div>
          <span class="eyebrow">Preview</span>
          <h2 id="image-preview-title">{previewIcon.name}</h2>
        </div>
        <button
          class="preview-close"
          type="button"
          aria-label="关闭大图预览"
          on:click={closePreview}
        >
          ×
        </button>
      </header>

      <div class="preview-stage">
        <img src={previewIcon.url} alt={previewIcon.name} />
      </div>

      <footer>
        <code>{previewIcon.path || previewIcon.url}</code>
        <button
          class="action secondary"
          type="button"
          on:click={copyPreviewUrl}
        >
          {copiedUrl === previewIcon.url ? '已复制' : '复制 Raw URL'}
        </button>
      </footer>
    </div>
  </div>
{/if}

<style>
  .set-hero {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(220px, 0.36fr);
    gap: 18px;
    overflow: hidden;
  }

  h1 {
    max-width: 900px;
    margin: 12px 0 0;
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: 64px;
    line-height: 0.94;
    letter-spacing: 0;
  }

  .set-hero p {
    max-width: 780px;
    margin: 14px 0 0;
    color: rgba(246, 239, 217, 0.68);
    line-height: 1.7;
  }

  .hero-side {
    display: grid;
    gap: 10px;
    min-width: 0;
    align-content: end;
  }

  .stat-stack {
    position: relative;
    display: grid;
    grid-template-rows: auto 1fr auto;
    min-height: 116px;
    padding: 16px 18px;
    overflow: hidden;
    align-self: stretch;
    border: 1px solid rgba(198, 255, 72, 0.28);
    border-radius: 14px;
    background:
      linear-gradient(135deg, rgba(255, 85, 36, 0.055), transparent 42%),
      rgba(198, 255, 72, 0.055);
  }

  .stat-stack::after {
    position: absolute;
    top: 16px;
    right: 16px;
    width: 44px;
    border-top: 1px solid rgba(198, 255, 72, 0.24);
    content: '';
  }

  .stat-stack strong {
    z-index: 1;
    justify-self: start;
    align-self: start;
    color: #ff5524;
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: 60px;
    line-height: 0.82;
  }

  .stat-stack span {
    z-index: 1;
    justify-self: end;
    align-self: end;
    max-width: 150px;
    color: rgba(246, 239, 217, 0.62);
    font-size: 14px;
    text-align: right;
  }

  .manifest-copy {
    width: 100%;
    min-height: 40px;
  }

  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
  }

  .search-field {
    flex: 1;
  }

  .icon-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(168px, 1fr));
    gap: 14px;
  }

  .icon-card {
    position: relative;
    display: grid;
    gap: 12px;
    padding: 12px;
    transition:
      border-color 160ms ease,
      background 160ms ease;
  }

  .icon-card:hover {
    border-color: rgba(198, 255, 72, 0.5);
    background: rgba(24, 26, 20, 0.82);
  }

  .copy-url {
    position: absolute;
    top: 20px;
    right: 20px;
    z-index: 2;
    display: grid;
    width: 34px;
    height: 34px;
    place-items: center;
    border: 1px solid rgba(246, 239, 217, 0.18);
    border-radius: 10px;
    color: rgba(246, 239, 217, 0.72);
    background: rgba(12, 13, 11, 0.78);
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.22);
    transition:
      border-color 160ms ease,
      background 160ms ease,
      color 160ms ease;
  }

  .copy-url:hover,
  .copy-url:focus-visible {
    border-color: rgba(198, 255, 72, 0.55);
    color: #c6ff48;
    background: rgba(24, 26, 20, 0.92);
    outline: none;
  }

  .copy-url.copied {
    border-color: rgba(198, 255, 72, 0.62);
    color: #c6ff48;
  }

  .copy-url.failed {
    border-color: rgba(255, 85, 36, 0.62);
    color: #ff5524;
  }

  .copy-url svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 2;
  }

  .icon-stage {
    display: grid;
    min-height: 132px;
    place-items: center;
    border: 1px solid rgba(246, 239, 217, 0.15);
    border-radius: 14px;
    background:
      linear-gradient(45deg, rgba(246, 239, 217, 0.06) 25%, transparent 25%),
      linear-gradient(-45deg, rgba(246, 239, 217, 0.06) 25%, transparent 25%),
      rgba(246, 239, 217, 0.04);
    background-position:
      0 0,
      0 8px;
    background-size: 16px 16px;
  }

  .image-preview-button {
    display: grid;
    width: 100%;
    min-height: 132px;
    place-items: center;
    border: 0;
    color: inherit;
    background: transparent;
    cursor: zoom-in;
  }

  .image-preview-button:focus-visible {
    outline: 2px solid rgba(198, 255, 72, 0.72);
    outline-offset: -6px;
  }

  .image-preview-button img {
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
    font-size: 21px;
    letter-spacing: 0;
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

  .image-preview-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: grid;
    place-items: center;
    padding: 20px;
    background: rgba(0, 0, 0, 0.74);
    backdrop-filter: blur(16px);
  }

  .image-preview {
    display: grid;
    gap: 14px;
    width: min(860px, 100%);
    max-height: calc(100vh - 40px);
    padding: clamp(14px, 2.6vw, 22px);
    border: 1px solid rgba(246, 239, 217, 0.2);
    border-radius: 12px;
    color: #f6efd9;
    background:
      linear-gradient(135deg, rgba(198, 255, 72, 0.08), transparent 34%),
      rgba(16, 17, 14, 0.96);
    box-shadow: 0 30px 90px rgba(0, 0, 0, 0.62);
  }

  .image-preview header,
  .image-preview footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .image-preview h2 {
    margin: 8px 0 0;
    overflow: hidden;
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: 36px;
    line-height: 1;
    letter-spacing: 0;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .preview-close {
    display: grid;
    width: 40px;
    height: 40px;
    flex: 0 0 auto;
    place-items: center;
    border: 1px solid rgba(246, 239, 217, 0.2);
    border-radius: 10px;
    color: #f6efd9;
    background: rgba(246, 239, 217, 0.06);
    font-size: 24px;
    line-height: 1;
  }

  .preview-close:hover,
  .preview-close:focus-visible {
    border-color: rgba(198, 255, 72, 0.5);
    background: rgba(198, 255, 72, 0.12);
    outline: none;
  }

  .preview-stage {
    display: grid;
    min-height: min(560px, 62vh);
    place-items: center;
    overflow: hidden;
    border: 1px solid rgba(246, 239, 217, 0.16);
    border-radius: 10px;
    background:
      linear-gradient(45deg, rgba(246, 239, 217, 0.055) 25%, transparent 25%),
      linear-gradient(-45deg, rgba(246, 239, 217, 0.055) 25%, transparent 25%),
      rgba(246, 239, 217, 0.035);
    background-position:
      0 0,
      0 10px;
    background-size: 20px 20px;
  }

  .preview-stage img {
    max-width: min(100%, 720px);
    max-height: min(62vh, 620px);
    object-fit: contain;
    filter: drop-shadow(0 22px 42px rgba(0, 0, 0, 0.42));
  }

  .image-preview footer code {
    min-width: 0;
    overflow: hidden;
    color: rgba(246, 239, 217, 0.58);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pagination {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    flex-wrap: wrap;
    padding: 12px 14px;
    border: 1px solid rgba(246, 239, 217, 0.16);
    border-radius: 8px;
    background: rgba(16, 17, 14, 0.64);
  }

  .pagination-summary {
    color: rgba(246, 239, 217, 0.58);
    font-size: 12px;
  }

  .pagination-summary strong {
    color: #c6ff48;
  }

  .page-size-control {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
    color: rgba(246, 239, 217, 0.58);
    font-size: 12px;
    font-weight: 800;
  }

  .page-size-select {
    width: 108px;
    min-height: 38px;
    color: #f6efd9;
  }

  .pagination-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .page-button {
    display: grid;
    width: 38px;
    height: 38px;
    place-items: center;
    border: 1px solid rgba(246, 239, 217, 0.2);
    border-radius: 10px;
    color: rgba(246, 239, 217, 0.76);
    background: rgba(246, 239, 217, 0.06);
    transition:
      border-color 160ms ease,
      background 160ms ease,
      color 160ms ease;
  }

  .page-button:hover:not(:disabled),
  .page-button:focus-visible {
    border-color: rgba(198, 255, 72, 0.55);
    color: #c6ff48;
    background: rgba(198, 255, 72, 0.1);
    outline: none;
  }

  .page-button:disabled {
    cursor: not-allowed;
    opacity: 0.38;
  }

  .page-button svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-linecap: round;
    stroke-linejoin: round;
    stroke-width: 2;
  }

  .page-indicator {
    min-width: 74px;
    color: rgba(246, 239, 217, 0.72);
    font-size: 12px;
    text-align: center;
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

    .pagination {
      align-items: stretch;
      flex-direction: column;
    }

    .page-size-control {
      justify-content: space-between;
      margin-left: 0;
    }

    .page-size-select {
      width: 130px;
    }

    .pagination-actions {
      justify-content: space-between;
    }

    .hero-side {
      min-width: 0;
    }

    .stat-stack {
      display: flex;
      min-height: 58px;
      align-items: baseline;
      justify-content: center;
      gap: 10px;
      padding: 10px 14px;
    }

    .stat-stack::after {
      display: none;
    }

    .stat-stack strong {
      font-size: 44px;
      line-height: 0.86;
    }

    .stat-stack strong,
    .stat-stack span {
      justify-self: center;
      align-self: baseline;
      text-align: center;
    }

    .image-preview {
      max-height: calc(100vh - 24px);
    }

    .image-preview header,
    .image-preview footer {
      align-items: stretch;
      flex-direction: column;
    }

    .image-preview h2 {
      font-size: 28px;
    }

    .preview-close {
      position: absolute;
      top: 14px;
      right: 14px;
    }

    .preview-stage {
      min-height: min(420px, 58vh);
    }
  }

  @media (max-width: 520px) {
    h1 {
      font-size: 42px;
    }

    .icon-grid {
      grid-template-columns: repeat(auto-fill, minmax(148px, 1fr));
    }
  }
</style>
