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
          output.push(`<pre><code>${escapeHtml(codeLines.join('\n'))}</code></pre>`);
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
      output.push(`<pre><code>${escapeHtml(codeLines.join('\n'))}</code></pre>`);
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

  .markdown-body :global(h1),
  .markdown-body :global(h2),
  .markdown-body :global(h3) {
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    letter-spacing: 0;
  }

  .markdown-body :global(h1) {
    margin: 0 0 22px;
    font-size: clamp(44px, 7vw, 72px);
    line-height: 0.94;
    overflow-wrap: anywhere;
  }

  .markdown-body :global(h2) {
    margin: 34px 0 12px;
    color: #c6ff48;
    font-size: 30px;
  }

  .markdown-body :global(h3) {
    margin: 24px 0 10px;
    font-size: 22px;
  }

  .markdown-body :global(p),
  .markdown-body :global(li) {
    color: rgba(246, 239, 217, 0.74);
    font-size: 15px;
    line-height: 1.85;
    overflow-wrap: break-word;
  }

  .markdown-body :global(p) {
    margin: 12px 0 0;
  }

  .markdown-body :global(ul) {
    display: grid;
    gap: 8px;
    margin: 12px 0 0;
    padding-left: 22px;
  }

  .markdown-body :global(li::marker) {
    color: #ff5524;
  }

  .markdown-body :global(code) {
    border: 1px solid rgba(246, 239, 217, 0.14);
    border-radius: 6px;
    padding: 2px 6px;
    color: #c6ff48;
    background: rgba(12, 13, 11, 0.58);
    font-size: 0.92em;
    overflow-wrap: anywhere;
  }

  .markdown-body :global(pre) {
    margin: 14px 0 0;
    max-width: 100%;
    overflow-x: auto;
    border: 1px solid rgba(246, 239, 217, 0.16);
    border-radius: 10px;
    background:
      linear-gradient(90deg, rgba(198, 255, 72, 0.06), transparent 42%),
      rgba(12, 13, 11, 0.72);
    -webkit-overflow-scrolling: touch;
  }

  .markdown-body :global(pre code) {
    display: block;
    min-width: max-content;
    border: 0;
    padding: 16px;
    color: #f6efd9;
    background: transparent;
    line-height: 1.7;
    white-space: pre;
    overflow-wrap: normal;
  }

  @media (max-width: 640px) {
    .docs-shell {
      padding: 18px 14px;
    }

    .markdown-body :global(h1) {
      margin-bottom: 18px;
      font-size: clamp(30px, 9vw, 40px);
      line-height: 1.08;
    }

    .markdown-body :global(h2) {
      margin: 28px 0 10px;
      font-size: 24px;
      line-height: 1.16;
    }

    .markdown-body :global(h3) {
      margin: 22px 0 8px;
      font-size: 19px;
      line-height: 1.22;
    }

    .markdown-body :global(p),
    .markdown-body :global(li) {
      font-size: 14px;
      line-height: 1.78;
    }

    .markdown-body :global(ul) {
      gap: 6px;
      padding-left: 18px;
    }

    .markdown-body :global(code) {
      padding: 1px 5px;
      font-size: 0.88em;
    }

    .markdown-body :global(pre) {
      margin-top: 12px;
      border-radius: 8px;
    }

    .markdown-body :global(pre code) {
      padding: 12px;
      font-size: 12px;
      line-height: 1.65;
    }
  }
</style>
