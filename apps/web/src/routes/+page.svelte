<script lang="ts">
  import { onMount } from 'svelte';
  import { manifestRawUrl } from '$lib/asset-url';
  import { copyText } from '$lib/clipboard';
  import { listSets } from '$lib/api';
  import { toast } from '$lib/toast';
  import type { IconSetSummary } from '$lib/types';

  let sets: IconSetSummary[] = [];
  let loading = true;
  let error = '';
  let copiedSetId = '';
  let failedSetId = '';

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
    const url = manifestRawUrl(setId);
    copiedSetId = '';
    failedSetId = '';

    try {
      await copyText(url);
      copiedSetId = setId;
      toast.info('集合地址已复制');
      window.setTimeout(() => {
        if (copiedSetId === setId) copiedSetId = '';
      }, 1600);
    } catch {
      failedSetId = setId;
      toast.error('复制集合地址失败');
      window.setTimeout(() => {
        if (failedSetId === setId) failedSetId = '';
      }, 1600);
    }
  }

  onMount(() => {
    void refreshSets();
  });
</script>

<section class="hero">
  <div>
    <span class="eyebrow">Raw GitHub Icon Library</span>
    <h1 class="hero-title">ICON<br /><span>SETS</span></h1>
    <p class="lead">
      为常用服务整理干净、可搜索、可复制链接的图标集合，让每一次替换封面和配置图标都更快一点。
    </p>
  </div>

  <aside class="hero-card panel panel-pad">
    <p>DATA SHAPE</p>
    <code>{`sets/{id}/manifest.json`}</code>
    <strong>{sets.length}</strong>
    <span>active sets</span>
  </aside>
</section>

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
                class="copy-set"
                type="button"
                title="复制 manifest.json 地址"
                on:click={() => copySetUrl(set.id)}
              >
                {#if copiedSetId === set.id}
                  已复制
                {:else if failedSetId === set.id}
                  复制失败
                {:else}
                  复制地址
                {/if}
              </button>
              <a class="open-set" href={`/sets/${set.id}`} title="打开集合">OPEN →</a>
            </div>
          </footer>
        </article>
      {/each}
    </div>
  {/if}
</section>

<style>
  .hero-card {
    align-self: end;
    display: grid;
    gap: 12px;
    min-height: 260px;
    border-color: rgba(198, 255, 72, 0.28);
  }

  .hero-card p {
    margin: 0;
    color: rgba(246, 239, 217, 0.58);
    font-size: 12px;
    font-weight: 800;
    letter-spacing: 0.16em;
  }

  .hero-card code {
    width: fit-content;
    padding: 10px 12px;
    border: 1px solid rgba(246, 239, 217, 0.16);
    border-radius: 14px;
    color: #c6ff48;
    background: rgba(0, 0, 0, 0.3);
  }

  .hero-card strong {
    align-self: end;
    color: #ff5524;
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: 112px;
    line-height: 0.8;
  }

  .hero-card span:not(.eyebrow) {
    color: rgba(246, 239, 217, 0.62);
  }

  .section-head {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: 18px;
  }

  h2,
  h3 {
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    letter-spacing: -0.05em;
  }

  h2 {
    margin: 12px 0 0;
    font-size: clamp(36px, 5vw, 68px);
    line-height: 0.94;
  }

  .set-card {
    position: relative;
    display: grid;
    min-height: 280px;
    padding: 22px;
    overflow: hidden;
    transition: border-color 180ms ease, background 180ms ease;
  }

  .set-card::before {
    position: absolute;
    inset: auto -40px -80px auto;
    width: 180px;
    height: 180px;
    border: 1px solid rgba(255, 85, 36, 0.36);
    border-radius: 36% 64% 43% 57%;
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
    width: 72px;
    height: 72px;
    place-items: center;
    border-radius: 22px;
    color: #0c0d0b;
    background: #f6efd9;
    box-shadow: 7px 7px 0 #ff5524;
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: 28px;
    font-weight: 800;
  }

  .set-id {
    position: absolute;
    top: 24px;
    right: 22px;
    color: rgba(198, 255, 72, 0.78);
    font-size: 12px;
  }

  .set-card h3 {
    align-self: end;
    margin: 44px 0 8px;
    font-size: 32px;
    line-height: 1;
  }

  .set-card p {
    display: -webkit-box;
    min-height: 72px;
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
    margin-top: 28px;
    color: rgba(246, 239, 217, 0.6);
    font-size: 13px;
  }

  .card-actions {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .copy-set {
    min-height: 34px;
    padding: 0 12px;
    border: 1px solid rgba(246, 239, 217, 0.2);
    border-radius: 999px;
    color: #f6efd9;
    background: rgba(246, 239, 217, 0.08);
    font-size: 12px;
    font-weight: 800;
    transition: border-color 160ms ease, background 160ms ease, color 160ms ease;
  }

  .copy-set:hover {
    border-color: rgba(198, 255, 72, 0.5);
    color: #0c0d0b;
    background: #c6ff48;
  }

  .open-set {
    color: #c6ff48;
    font-weight: 800;
  }

  .empty {
    display: grid;
    gap: 16px;
    max-width: 720px;
  }

  .empty h3 {
    margin: 0;
    font-size: 42px;
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
  }
</style>
