export function renderMarkdown(markdown: string) {
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
    output.push(`<pre><code>${escapeHtml(codeLines.join('\n'))}</code></pre>`);
  }

  return output.join('\n');
}

function inlineMarkdown(value: string) {
  return escapeHtml(value)
    .replace(
      /\[([^\]]+)\]\(([^)\s]+)\)/g,
      (_match, label: string, href: string) => {
        const safeHref = safeLinkHref(href);
        if (!safeHref) return label;
        const isExternal = /^https?:\/\//.test(safeHref);
        const externalAttrs = isExternal
          ? ' target="_blank" rel="noreferrer"'
          : '';
        return `<a href="${safeHref}"${externalAttrs}>${label}</a>`;
      }
    )
    .replace(/`([^`]+)`/g, '<code>$1</code>');
}

function safeLinkHref(href: string) {
  const normalized = href.trim();
  if (/^(https?:\/\/|\/|#)/.test(normalized)) return normalized;
  return '';
}

function escapeHtml(value: string) {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}
