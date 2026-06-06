<script lang="ts">
  import { onMount } from 'svelte';
  import { renderMarkdown } from '$lib/markdown';

  let html = '';
  let error = '';

  onMount(async () => {
    try {
      const response = await fetch('/docs/github-repo-config.md');
      if (!response.ok) {
        throw new Error(`文档加载失败：${response.status}`);
      }
      html = renderMarkdown(await response.text());
    } catch (err) {
      error = err instanceof Error ? err.message : '文档加载失败';
    }
  });
</script>

<svelte:head>
  <title>GitHub 仓库配置指引 - Icon Set</title>
</svelte:head>

<nav class="breadcrumb" aria-label="面包屑">
  <a href="/">图标集合</a>
  <span>/</span>
  <strong>GitHub 仓库配置</strong>
</nav>

<article class="docs-shell panel panel-pad">
  {#if error}
    <div class="notice error">{error}</div>
  {:else if !html}
    <div class="notice">正在加载配置指引...</div>
  {:else}
    <div class="markdown-body">
      {@html html}
    </div>
  {/if}
</article>

<style>
  .docs-shell {
    width: 100%;
    max-width: 920px;
    margin: 0 auto;
    overflow: hidden;
  }

  .markdown-body {
    min-width: 0;
    overflow-wrap: break-word;
  }

  @media (max-width: 640px) {
    .docs-shell {
      padding: 18px 14px;
    }
  }
</style>
