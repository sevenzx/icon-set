<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import {
    getShareAccessSession,
    getShareEditSet,
    logoutShareAccess,
    renameShareEditIcon,
    uploadShareEditIcon,
    uploadShareEditIconsBatch
  } from '$lib/api';
  import { copyText } from '$lib/clipboard';
  import { toast } from '$lib/toast';
  import type { IconEntry, IconManifest } from '$lib/types';

  type IconSortMode = 'name-asc' | 'name-desc' | 'path-asc';

  const batchUploadMaxBytes = 10 * 1024 * 1024;

  let manifest: IconManifest | null = null;
  let loading = true;
  let uploading = false;
  let batchUploading = false;
  let renaming = false;
  let error = '';
  let uploadName = '';
  let selectedFile: File | null = null;
  let singleFileInput: HTMLInputElement | null = null;
  let batchFiles: File[] = [];
  let archiveFile: File | null = null;
  let batchFilesInput: HTMLInputElement | null = null;
  let archiveInput: HTMLInputElement | null = null;
  let query = '';
  let sortMode: IconSortMode = 'name-asc';
  let previewIcon: IconEntry | null = null;
  let renameIconTarget: IconEntry | null = null;
  let renameValue = '';

  $: batchTotalBytes =
    batchFiles.reduce((total, file) => total + file.size, 0) +
    (archiveFile?.size ?? 0);
  $: batchTooLarge = batchTotalBytes > batchUploadMaxBytes;
  $: visibleIcons = sortIcons(filterIcons(manifest?.icons ?? [], query), sortMode);

  /// 检查协作者会话，未进入时回到协作入口页。
  async function guardSession() {
    const session = await getShareAccessSession();
    if (!session.active) {
      await goto('/share/edit');
      return false;
    }

    return true;
  }

  /// 读取协作者当前可编辑的集合。
  async function refreshManifest() {
    loading = true;
    error = '';

    try {
      manifest = await getShareEditSet();
    } catch (err) {
      error = err instanceof Error ? err.message : '协作集合加载失败';
      manifest = null;
    } finally {
      loading = false;
    }
  }

  /// 记录用户选择的上传图片。
  function handleFileChange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    selectedFile = input.files?.[0] ?? null;
  }

  /// 记录用户批量选择的图片。
  function handleBatchFilesChange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    batchFiles = Array.from(input.files ?? []);
  }

  /// 记录用户选择的 zip 压缩包。
  function handleArchiveChange(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    archiveFile = input.files?.[0] ?? null;
  }

  /// 上传单张图片到共享编辑集合。
  async function submitUpload() {
    if (!selectedFile) return;
    uploading = true;
    error = '';

    try {
      manifest = await uploadShareEditIcon(uploadName, selectedFile);
      uploadName = '';
      selectedFile = null;
      if (singleFileInput) singleFileInput.value = '';
      toast.info('图片已上传');
    } catch (err) {
      error = err instanceof Error ? err.message : '上传失败';
      toast.error(error);
    } finally {
      uploading = false;
    }
  }

  /// 批量上传图片或 zip 压缩包到共享编辑集合。
  async function submitBatchUpload() {
    if (batchFiles.length === 0 && !archiveFile) {
      toast.error('请选择图片或 zip 压缩包');
      return;
    }
    if (batchTooLarge) {
      toast.error('批量上传总体积不能超过 10MB');
      return;
    }

    batchUploading = true;
    error = '';

    try {
      manifest = await uploadShareEditIconsBatch(batchFiles, archiveFile);
      batchFiles = [];
      archiveFile = null;
      if (batchFilesInput) batchFilesInput.value = '';
      if (archiveInput) archiveInput.value = '';
      toast.info('批量图片已上传');
    } catch (err) {
      error = err instanceof Error ? err.message : '批量上传失败';
      toast.error(error);
    } finally {
      batchUploading = false;
    }
  }

  function openPreview(icon: IconEntry) {
    previewIcon = icon;
  }

  function closePreview() {
    previewIcon = null;
  }

  function openRenameModal(icon: IconEntry) {
    previewIcon = null;
    renameIconTarget = icon;
    renameValue = icon.name;
  }

  function closeRenameModal() {
    if (renaming) return;
    renameIconTarget = null;
    renameValue = '';
  }

  /// 保存图标新名称。
  async function submitRename() {
    if (!renameIconTarget) return;
    renaming = true;

    try {
      manifest = await renameShareEditIcon(renameIconTarget.id, renameValue);
      renameIconTarget = null;
      renameValue = '';
      toast.info('图标名称已更新');
    } catch (err) {
      error = err instanceof Error ? err.message : '重命名失败';
      toast.error(error);
    } finally {
      renaming = false;
    }
  }

  async function copyPreviewUrl() {
    if (!previewIcon) return;

    try {
      await copyText(previewIcon.url);
      toast.info('Raw URL 已复制');
    } catch {
      toast.error('复制 Raw URL 失败');
    }
  }

  async function leaveShareSession() {
    try {
      await logoutShareAccess();
    } catch {
      toast.error('退出协作失败，请稍后重试');
      return;
    }
    await goto('/share/edit');
  }

  function filterIcons(icons: IconEntry[], keyword: string) {
    const normalized = keyword.trim().toLowerCase();
    if (!normalized) return icons;

    return icons.filter((icon) =>
      [icon.name, icon.path, icon.url]
        .filter(Boolean)
        .some((value) => value.toLowerCase().includes(normalized))
    );
  }

  function sortIcons(icons: IconEntry[], mode: IconSortMode) {
    const sorted = [...icons];
    sorted.sort((left, right) => {
      if (mode === 'path-asc') {
        return left.path.localeCompare(right.path);
      }

      const result = left.name.localeCompare(right.name);
      return mode === 'name-desc' ? -result : result;
    });

    return sorted;
  }

  function formatBytes(bytes: number) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  function handlePreviewBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      closePreview();
    }
  }

  function handleRenameBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      closeRenameModal();
    }
  }

  function handleWindowKeydown(event: KeyboardEvent) {
    if (event.key !== 'Escape') return;
    if (renameIconTarget) {
      closeRenameModal();
      return;
    }
    if (previewIcon) {
      closePreview();
    }
  }

  onMount(async () => {
    if (await guardSession()) {
      await refreshManifest();
    }
  });
</script>

<svelte:window on:keydown={handleWindowKeydown} />

<nav class="breadcrumb" aria-label="面包屑">
  <a href="/">图标集合</a>
  <span>/</span>
  <strong>共享编辑</strong>
</nav>

{#if loading}
  <div class="notice">正在读取共享编辑集合...</div>
{:else if error && !manifest}
  <div class="notice error">{error}</div>
{:else if manifest}
  <section class="set-admin-hero panel panel-pad">
    <div class="hero-copy">
      <span class="eyebrow">Shared Editor / {manifest.id}</span>
      <h1>{manifest.name}</h1>
      <p>{manifest.icons.length} icons · 仅开放上传与重命名</p>

      <div class="hero-actions">
        <button class="action secondary" type="button" on:click={refreshManifest}>
          刷新
        </button>
        <button class="action secondary" type="button" on:click={leaveShareSession}>
          退出协作
        </button>
      </div>
    </div>

    <div class="hero-note panel">
      <span class="eyebrow">Access Scope</span>
      <strong>只允许当前集合内的 icon 维护</strong>
      <p>你可以上传、批量上传和重命名图片；不能修改集合元信息，也不能删除整个集合。</p>
    </div>
  </section>

  <section class="upload-workbench">
    <div class="workbench-head">
      <div>
        <span class="eyebrow">Upload</span>
        <h2>上传工作台</h2>
        <p>先上传，再在下方图片列表里集中管理名称和预览。</p>
      </div>
    </div>

    <div class="manage-grid">
      <form class="panel panel-pad upload-card" on:submit|preventDefault={submitUpload}>
        <span class="eyebrow">Upload</span>
        <h2>单张上传</h2>
        <label class="field">
          <span>图标名称</span>
          <input class="input" bind:value={uploadName} maxlength="120" placeholder="留空则从文件名提取" />
          <small>只允许英文字母、数字、空格和 .、-、_</small>
        </label>
        <label class="file-drop">
          <input
            type="file"
            accept="image/png,image/jpeg,image/webp,image/svg+xml"
            bind:this={singleFileInput}
            on:change={handleFileChange}
          />
          <strong>{selectedFile ? selectedFile.name : '选择图片文件'}</strong>
          <span>支持 png / jpg / webp / svg</span>
        </label>
        <button class="action" type="submit" disabled={uploading || !selectedFile}>
          {uploading ? '上传中...' : '上传到集合'}
        </button>
      </form>

      <form class="panel panel-pad batch-card" on:submit|preventDefault={submitBatchUpload}>
        <span class="eyebrow">Batch Upload</span>
        <h2>批量上传</h2>
        <label class="file-drop">
          <input
            type="file"
            accept="image/png,image/jpeg,image/webp,image/svg+xml"
            multiple
            bind:this={batchFilesInput}
            on:change={handleBatchFilesChange}
          />
          <strong>{batchFiles.length > 0 ? `${batchFiles.length} 个图片文件` : '多选图片文件'}</strong>
          <span>支持 png / jpg / webp / svg</span>
        </label>
        <label class="file-drop">
          <input
            type="file"
            accept=".zip,application/zip"
            bind:this={archiveInput}
            on:change={handleArchiveChange}
          />
          <strong>{archiveFile ? archiveFile.name : '选择 zip 压缩包'}</strong>
          <span>从压缩包内图片文件名生成 name</span>
        </label>
        <div class:over-limit={batchTooLarge} class="batch-total">
          总体积 {formatBytes(batchTotalBytes)} / 10 MB
        </div>
        <button
          class="action"
          type="submit"
          disabled={batchUploading || batchTooLarge || (batchFiles.length === 0 && !archiveFile)}
        >
          {batchUploading ? '批量上传中...' : '批量上传到集合'}
        </button>
      </form>
    </div>
  </section>

  <section class="panel panel-pad icon-manager">
    <div class="manager-head">
      <div>
        <span class="eyebrow">Icons</span>
        <h2>图片管理</h2>
        <p>{visibleIcons.length} / {manifest.icons.length} 张图片</p>
      </div>
    </div>

    <div class="manager-toolbar">
      <label class="field manager-search">
        <span>搜索图片</span>
        <input class="input" bind:value={query} placeholder="输入 name / path / URL" />
      </label>

      <label class="field manager-sort">
        <span>排序</span>
        <select class="input" bind:value={sortMode}>
          <option value="name-asc">名称升序</option>
          <option value="name-desc">名称降序</option>
          <option value="path-asc">路径升序</option>
        </select>
      </label>
    </div>

    {#if manifest.icons.length === 0}
      <div class="notice">这个集合还没有图片。</div>
    {:else if visibleIcons.length === 0}
      <div class="notice">没有匹配的图片。</div>
    {:else}
      <div class="admin-icon-grid">
        {#each visibleIcons as icon}
          <article class="admin-icon-card">
            <button class="thumb" type="button" aria-label={`查看 ${icon.name} 大图`} on:click={() => openPreview(icon)}>
              <img src={icon.url} alt={icon.name} loading="lazy" />
            </button>

            <div class="icon-card-body">
              <button class="icon-name-button" type="button" title="打开重命名弹窗" on:click={() => openRenameModal(icon)}>
                {icon.name}
              </button>
              <code title={icon.path}>{icon.path}</code>
            </div>
          </article>
        {/each}
      </div>
    {/if}
  </section>
{/if}

{#if previewIcon}
  <div class="image-preview-backdrop" role="presentation" on:click={handlePreviewBackdropClick}>
    <div class="image-preview" role="dialog" aria-modal="true" aria-labelledby="image-preview-title">
      <header>
        <div>
          <span class="eyebrow">Preview</span>
          <h2 id="image-preview-title">{previewIcon.name}</h2>
        </div>
        <button class="preview-close" type="button" aria-label="关闭预览" on:click={closePreview}>
          ×
        </button>
      </header>

      <div class="preview-stage">
        <img src={previewIcon.url} alt={previewIcon.name} />
      </div>

      <footer>
        <code>{previewIcon.path || previewIcon.url}</code>
        <div class="preview-actions">
          <button class="action secondary" type="button" on:click={copyPreviewUrl}>
            复制 Raw URL
          </button>
          <button
            class="action"
            type="button"
            on:click={() => previewIcon && openRenameModal(previewIcon)}
          >
            重命名
          </button>
        </div>
      </footer>
    </div>
  </div>
{/if}

{#if renameIconTarget}
  <div class="rename-backdrop" role="presentation" on:click={handleRenameBackdropClick}>
    <div class="rename-modal" role="dialog" aria-modal="true" aria-labelledby="rename-modal-title">
      <form class="rename-form" on:submit|preventDefault={submitRename}>
        <header>
          <div>
            <span class="eyebrow">Rename Icon</span>
            <h2 id="rename-modal-title">重命名图片</h2>
          </div>
          <button
            class="preview-close"
            type="button"
            aria-label="关闭重命名弹窗"
            disabled={renaming}
            on:click={closeRenameModal}
          >
            ×
          </button>
        </header>

        <div class="rename-target">
          <div class="rename-thumb">
            <img src={renameIconTarget.url} alt={renameIconTarget.name} />
          </div>
          <div>
            <strong>{renameIconTarget.name}</strong>
            <code>{renameIconTarget.path}</code>
          </div>
        </div>

        <label class="field">
          <span>新的 name</span>
          <input
            class="input"
            bind:value={renameValue}
            maxlength="120"
            pattern="[A-Za-z0-9 ._-]+"
            required
            autocomplete="off"
            spellcheck="false"
            disabled={renaming}
          />
          <small>保存后 path 和 Raw URL 会跟随新的 name 更新。</small>
        </label>

        <div class="rename-modal-actions">
          <button class="action secondary" type="button" disabled={renaming} on:click={closeRenameModal}>
            取消
          </button>
          <button class="action" type="submit" disabled={renaming || !renameValue.trim()}>
            {renaming ? '保存中...' : '保存重命名'}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .set-admin-hero {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(320px, 0.56fr);
    gap: 24px;
    align-items: stretch;
  }

  .hero-copy,
  .hero-note,
  .upload-card,
  .batch-card,
  .icon-manager,
  .rename-form {
    display: grid;
    gap: 14px;
  }

  .hero-copy {
    min-width: 0;
  }

  .hero-note {
    align-content: start;
    padding: 18px;
    border: 1px solid rgba(246, 239, 217, 0.14);
    border-radius: 10px;
    background: rgba(246, 239, 217, 0.04);
  }

  .hero-note strong {
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: 24px;
    letter-spacing: 0;
  }

  .hero-actions,
  .manager-head,
  .workbench-head,
  .preview-actions,
  .rename-modal-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .hero-actions {
    justify-content: flex-start;
    margin-top: auto;
    padding-top: 18px;
  }

  .upload-workbench {
    display: grid;
    gap: 14px;
  }

  .manage-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px;
  }

  .field small {
    color: rgba(246, 239, 217, 0.5);
    font-size: 12px;
  }

  .file-drop {
    display: grid;
    min-height: 150px;
    place-items: center;
    padding: 20px;
    border: 1px dashed rgba(198, 255, 72, 0.42);
    border-radius: 8px;
    background: rgba(198, 255, 72, 0.06);
    text-align: center;
  }

  .file-drop input {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }

  .file-drop strong {
    color: #c6ff48;
    font-size: 18px;
  }

  .file-drop span,
  .batch-total,
  .manager-head p,
  .workbench-head p,
  p {
    color: rgba(246, 239, 217, 0.6);
  }

  .batch-total.over-limit {
    color: #ff6b4a;
  }

  .manager-toolbar {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(180px, 260px);
    gap: 12px;
    align-items: end;
    padding: 12px;
    border: 1px solid rgba(246, 239, 217, 0.12);
    border-radius: 8px;
    background: rgba(246, 239, 217, 0.035);
  }

  .admin-icon-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 14px;
  }

  .admin-icon-card {
    display: grid;
    gap: 12px;
    padding: 12px;
    border: 1px solid rgba(246, 239, 217, 0.16);
    border-radius: 8px;
    background:
      linear-gradient(180deg, rgba(246, 239, 217, 0.05), transparent 48%),
      rgba(12, 13, 11, 0.62);
  }

  .thumb {
    display: grid;
    width: 100%;
    min-height: 150px;
    place-items: center;
    padding: 16px;
    border: 1px solid rgba(246, 239, 217, 0.12);
    border-radius: 8px;
    background:
      linear-gradient(135deg, rgba(246, 239, 217, 0.035), transparent 45%),
      rgba(0, 0, 0, 0.24);
  }

  .thumb img,
  .rename-thumb img {
    max-width: 112px;
    max-height: 112px;
    object-fit: contain;
  }

  .thumb:hover,
  .thumb:focus-visible {
    border-color: rgba(198, 255, 72, 0.36);
    outline: none;
  }

  .icon-card-body {
    display: grid;
    gap: 8px;
    min-width: 0;
  }

  .icon-name-button {
    width: 100%;
    min-width: 0;
    padding: 0;
    overflow: hidden;
    border: 0;
    color: #f6efd9;
    background: transparent;
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: 22px;
    letter-spacing: 0;
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .icon-name-button:hover,
  .icon-name-button:focus-visible {
    color: #c6ff48;
    outline: none;
  }

  .icon-card-body code,
  .image-preview footer code,
  .rename-target code {
    overflow: hidden;
    color: rgba(246, 239, 217, 0.5);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .image-preview-backdrop,
  .rename-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: grid;
    place-items: center;
    padding: 20px;
    background: rgba(0, 0, 0, 0.74);
    backdrop-filter: blur(16px);
  }

  .image-preview,
  .rename-modal {
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
  .image-preview footer,
  .rename-form header,
  .rename-target {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .preview-close {
    display: grid;
    width: 40px;
    height: 40px;
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
  }

  .rename-thumb {
    display: grid;
    width: 96px;
    height: 96px;
    place-items: center;
    border: 1px solid rgba(246, 239, 217, 0.12);
    border-radius: 8px;
    background: rgba(246, 239, 217, 0.04);
  }

  h1,
  h2 {
    margin: 0;
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    letter-spacing: 0;
  }

  h1 {
    margin-top: 12px;
    font-size: 68px;
    line-height: 0.94;
  }

  h2 {
    font-size: 38px;
  }

  p {
    margin: 14px 0 0;
    line-height: 1.8;
  }

  @media (max-width: 980px) {
    .set-admin-hero,
    .manage-grid,
    .manager-toolbar {
      grid-template-columns: 1fr;
    }

    .hero-actions,
    .manager-head,
    .workbench-head,
    .preview-actions,
    .rename-modal-actions,
    .image-preview footer,
    .rename-target {
      align-items: stretch;
      flex-direction: column;
    }

    h1 {
      font-size: 46px;
    }
  }
</style>
