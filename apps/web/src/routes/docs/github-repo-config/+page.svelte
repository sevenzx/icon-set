<script lang="ts">
  import { onMount } from 'svelte';

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

  function renderMarkdown(markdown: string) {
    const output: string[] = [];
    const paragraph: string[] = [];
    let inList = false;
    let inCode = false;
    let codeLines: string[] = [];

    const closeParagraph = () => {
      if (paragraph.length === 0) return;
      output.push(`<p>${paragraph.map(inlineMarkdown).join(' ')}</p>`);
      paragraph.length = 0;
    };

    const closeList = () => {
      if (!inList) return;
      output.push('</ul>');
      inList = false;
    };

    for (const line of markdown.split(/\r?\n/)) {
      if (line.startsWith('```')) {
        if (inCode) {
          output.push(
            `<pre><code>${escapeHtml(codeLines.join('\n'))}</code></pre>`
          );
          codeLines = [];
          inCode = false;
        } else {
          closeParagraph();
          closeList();
          inCode = true;
        }
        continue;
      }

      if (inCode) {
        codeLines.push(line);
        continue;
      }

      if (!line.trim()) {
        closeParagraph();
        closeList();
        continue;
      }

      const heading = /^(#{1,3})\s+(.+)$/.exec(line);
      if (heading) {
        closeParagraph();
        closeList();
        const level = heading[1].length;
        output.push(`<h${level}>${inlineMarkdown(heading[2])}</h${level}>`);
        continue;
      }

      const listItem = /^-\s+(.+)$/.exec(line);
      if (listItem) {
        closeParagraph();
        if (!inList) {
          output.push('<ul>');
          inList = true;
        }
        output.push(`<li>${inlineMarkdown(listItem[1])}</li>`);
        continue;
      }

      paragraph.push(line.trim());
    }

    closeParagraph();
    closeList();
    if (inCode) {
      output.push(
        `<pre><code>${escapeHtml(codeLines.join('\n'))}</code></pre>`
      );
    }

    return output.join('\n');
  }

  function inlineMarkdown(value: string) {
    return escapeHtml(value).replace(/`([^`]+)`/g, '<code>$1</code>');
  }

  function escapeHtml(value: string) {
    return value
      .replaceAll('&', '&amp;')
      .replaceAll('<', '&lt;')
      .replaceAll('>', '&gt;')
      .replaceAll('"', '&quot;')
      .replaceAll("'", '&#39;');
  }
</script>

<svelte:head>
  <title>GitHub 仓库配置指引 - Icon Set</title>
</svelte:head>

<nav class="breadcrumb" aria-label="面包屑">
  <a href="/">图标集合</a>
  <span>/</span>
  <a href="/console">控制台</a>
  <span>/</span>
  <strong>配置指引</strong>
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
