<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { onMount } from 'svelte';
  import DeleteConfirmModal from '$lib/DeleteConfirmModal.svelte';
  import { manifestRawUrl } from '$lib/asset-url';
  import {
    getAdminSet,
    getSession,
    removeIcon,
    renameIcon,
    updateSet,
    uploadIcon,
    uploadIconsBatch
  } from '$lib/api';
  import { copyText } from '$lib/clipboard';
  import { toast } from '$lib/toast';
  import type { IconEntry, IconManifest, RepoConfig } from '$lib/types';

  type IconSortMode = 'name-asc' | 'name-desc' | 'path-asc';

  const batchUploadMaxBytes = 10 * 1024 * 1024;

  let manifest: IconManifest | null = null;
  let repoConfig: RepoConfig | null = null;
  let loading = true;
  let savingMeta = false;
  let uploading = false;
  let batchUploading = false;
  let deletingIcon = false;
  let error = '';
  let metaForm = { name: '', description: '' };
  let uploadName = '';
  let selectedFile: File | null = null;
  let batchFiles: File[] = [];
  let archiveFile: File | null = null;
  let batchFilesInput: HTMLInputElement | null = null;
  let archiveInput: HTMLInputElement | null = null;
  let deleteIconTarget: IconEntry | null = null;
  let bulkDeleteOpen = false;
  let renameIconTarget: IconEntry | null = null;
  let previewIcon: IconEntry | null = null;
  let managerQuery = '';
  let managerSort: IconSortMode = 'name-asc';
  let managerPageSize = 24;
  let managerPage = 1;
  let previousManagerQuery = '';
  let previousManagerSort: IconSortMode = managerSort;
  let previousManagerPageSize = managerPageSize;
  let selectedIconIds: string[] = [];
  let activeMenuId = '';
  let renameValue = '';
  let renamingIcon = false;
  let manifestUrlCopied = false;
  const iconNamePattern = /^[A-Za-z0-9 ._-]+$/;
  const managerPageSizeOptions = [24, 48, 96];

  $: batchTotalBytes =
    batchFiles.reduce((total, file) => total + file.size, 0) +
    (archiveFile?.size ?? 0);
  $: batchTooLarge = batchTotalBytes > batchUploadMaxBytes;
  $: visibleIcons = sortIcons(
    filterManagedIcons(manifest?.icons ?? [], managerQuery),
    managerSort
  );
  $: managerTotalPages = Math.max(
    1,
    Math.ceil(visibleIcons.length / managerPageSize)
  );
  $: if (
    managerQuery !== previousManagerQuery ||
    managerSort !== previousManagerSort ||
    managerPageSize !== previousManagerPageSize
  ) {
    managerPage = 1;
    previousManagerQuery = managerQuery;
    previousManagerSort = managerSort;
    previousManagerPageSize = managerPageSize;
  }
  $: if (managerPage > managerTotalPages) {
    managerPage = managerTotalPages;
  }
  $: managerPageStart = (managerPage - 1) * managerPageSize;
  $: managerPageEnd = Math.min(
    managerPageStart + managerPageSize,
    visibleIcons.length
  );
  $: pagedIcons = visibleIcons.slice(managerPageStart, managerPageEnd);
  $: selectedIcons = (manifest?.icons ?? []).filter((icon) =>
    selectedIconIds.includes(icon.id)
  );
  $: selectedCount = selectedIcons.length;
  $: allPageSelected =
    pagedIcons.length > 0 &&
    pagedIcons.every((icon) => selectedIconIds.includes(icon.id));
  $: bulkDeleteConfirmLabel = `删除 ${selectedCount} 个`;
  $: manifestUrl = manifest ? manifestRawUrl(manifest.id, repoConfig) : '';

  /// 同步 manifest 和依赖它的 UI 状态。
  function applyManifest(nextManifest: IconManifest) {
    const nextIds = new Set(nextManifest.icons.map((icon) => icon.id));

    manifest = nextManifest;
    selectedIconIds = selectedIconIds.filter((id) => nextIds.has(id));

    if (activeMenuId && !nextIds.has(activeMenuId)) activeMenuId = '';
    if (renameIconTarget && !nextIds.has(renameIconTarget.id)) {
      closeRenameModal();
    }
    if (previewIcon && !nextIds.has(previewIcon.id)) previewIcon = null;
  }

  /// 校验控制台会话，未登录时跳转登录页。
  async function guardSession() {
    const session = await getSession();
    if (!session.authenticated) {
      await goto('/console/login');
      return false;
    }
    repoConfig = session.repo_config ?? null;
    return true;
  }

  /// 读取当前集合 manifest 并初始化表单。
  async function refreshManifest() {
    loading = true;
    error = '';
    const setId = page.params.setId;

    if (!setId) {
      error = '缺少集合 ID';
      loading = false;
      return;
    }

    try {
      const nextManifest = await getAdminSet(setId);
      applyManifest(nextManifest);
      metaForm = {
        name: nextManifest.name,
        description: nextManifest.description
      };
    } catch (err) {
      error = err instanceof Error ? err.message : '集合加载失败';
      if (manifest) toast.error(error);
    } finally {
      loading = false;
    }
  }

  /// 保存集合名称和描述。
  async function submitMeta() {
    if (!manifest) return;
    savingMeta = true;
    error = '';

    try {
      await updateSet(manifest.id, metaForm);
      await refreshManifest();
      toast.info('集合信息已更新');
    } catch (err) {
      error = err instanceof Error ? err.message : '保存集合失败';
      toast.error(error);
    } finally {
      savingMeta = false;
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

  /// 上传图片并刷新 manifest。
  async function submitUpload() {
    if (!manifest || !selectedFile) return;
    error = '';

    if (uploadName.trim() && !isValidIconName(uploadName)) {
      error = iconNameError(uploadName);
      toast.error(error);
      return;
    }

    uploading = true;

    try {
      applyManifest(await uploadIcon(manifest.id, uploadName, selectedFile));
      uploadName = '';
      selectedFile = null;
      toast.info('图片已上传到 GitHub');
    } catch (err) {
      error = err instanceof Error ? err.message : '上传失败';
      toast.error(error);
    } finally {
      uploading = false;
    }
  }

  /// 批量上传图片或 zip 压缩包并刷新 manifest。
  async function submitBatchUpload() {
    if (!manifest) return;
    error = '';

    if (batchFiles.length === 0 && !archiveFile) {
      error = '请选择图片或 zip 压缩包';
      toast.error(error);
      return;
    }
    if (batchTooLarge) {
      error = '批量上传总体积不能超过 10MB';
      toast.error(error);
      return;
    }

    batchUploading = true;

    try {
      applyManifest(
        await uploadIconsBatch(manifest.id, batchFiles, archiveFile)
      );
      batchFiles = [];
      archiveFile = null;
      if (batchFilesInput) batchFilesInput.value = '';
      if (archiveInput) archiveInput.value = '';
      toast.info('批量图片已上传到 GitHub');
    } catch (err) {
      error = err instanceof Error ? err.message : '批量上传失败';
      toast.error(error);
    } finally {
      batchUploading = false;
    }
  }

  function openRenameModal(icon: IconEntry) {
    renameIconTarget = icon;
    renameValue = icon.name;
    activeMenuId = '';
    error = '';
  }

  function closeRenameModal() {
    if (renamingIcon) return;
    renameIconTarget = null;
    renameValue = '';
  }

  /// 保存单个图标名称。
  async function confirmRenameIcon() {
    if (!manifest || !renameIconTarget) return;
    error = '';
    const nextName = renameValue.trim();

    if (!isValidIconName(nextName)) {
      error = iconNameError(renameValue);
      toast.error(error);
      return;
    }

    if (nextName === renameIconTarget.name) {
      closeRenameModal();
      return;
    }

    renamingIcon = true;

    try {
      applyManifest(
        await renameIcon(manifest.id, renameIconTarget.id, nextName)
      );
      renameIconTarget = null;
      renameValue = '';
      toast.info('图标名称已更新');
    } catch (err) {
      error = err instanceof Error ? err.message : '改名失败';
      toast.error(error);
    } finally {
      renamingIcon = false;
    }
  }

  function startRename(icon: IconEntry) {
    openRenameModal(icon);
  }

  /// 打开删除图标确认弹窗。
  function openDeleteIconModal(icon: IconEntry) {
    deleteIconTarget = icon;
    activeMenuId = '';
    error = '';
  }

  /// 关闭删除图标确认弹窗。
  function closeDeleteIconModal() {
    if (deletingIcon) return;
    deleteIconTarget = null;
  }

  /// 删除单个图标及 GitHub 文件。
  async function confirmDeleteIcon() {
    if (!manifest) return;
    if (!deleteIconTarget) return;
    deletingIcon = true;
    error = '';

    try {
      const deletedName = deleteIconTarget.name;
      applyManifest(await removeIcon(manifest.id, deleteIconTarget.id));
      deleteIconTarget = null;
      toast.info(`图标 ${deletedName} 已删除`);
    } catch (err) {
      error = err instanceof Error ? err.message : '删除失败';
      toast.error(error);
    } finally {
      deletingIcon = false;
    }
  }

  function openBulkDeleteModal() {
    if (selectedCount === 0) return;
    bulkDeleteOpen = true;
    activeMenuId = '';
  }

  function closeBulkDeleteModal() {
    if (deletingIcon) return;
    bulkDeleteOpen = false;
  }

  async function confirmBulkDeleteIcons() {
    if (!manifest || selectedIcons.length === 0) return;
    deletingIcon = true;
    error = '';
    const targets = [...selectedIcons];

    try {
      let nextManifest = manifest;
      for (const icon of targets) {
        nextManifest = await removeIcon(nextManifest.id, icon.id);
      }
      applyManifest(nextManifest);
      selectedIconIds = [];
      bulkDeleteOpen = false;
      toast.info(`已删除 ${targets.length} 个图标`);
    } catch (err) {
      error = err instanceof Error ? err.message : '批量删除失败';
      toast.error(error);
      await refreshManifest();
    } finally {
      deletingIcon = false;
    }
  }

  function filterManagedIcons(icons: IconEntry[], keyword: string) {
    const normalized = keyword.trim().toLowerCase();
    if (!normalized) return icons;

    return icons.filter((icon) =>
      [icon.name, icon.path, icon.url]
        .filter(Boolean)
        .some((value) => value.toLowerCase().includes(normalized))
    );
  }

  function sortIcons(icons: IconEntry[], sortMode: IconSortMode) {
    const sorted = [...icons];
    sorted.sort((left, right) => {
      if (sortMode === 'path-asc') {
        return left.path.localeCompare(right.path);
      }

      const result = left.name.localeCompare(right.name);
      return sortMode === 'name-desc' ? -result : result;
    });
    return sorted;
  }

  function isIconSelected(iconId: string) {
    return selectedIconIds.includes(iconId);
  }

  function toggleIconSelection(iconId: string) {
    selectedIconIds = isIconSelected(iconId)
      ? selectedIconIds.filter((id) => id !== iconId)
      : [...selectedIconIds, iconId];
  }

  function togglePageSelection() {
    const pageIds = pagedIcons.map((icon) => icon.id);
    if (allPageSelected) {
      selectedIconIds = selectedIconIds.filter((id) => !pageIds.includes(id));
      return;
    }

    selectedIconIds = Array.from(new Set([...selectedIconIds, ...pageIds]));
  }

  function clearIconSelection() {
    selectedIconIds = [];
  }

  function toggleIconMenu(iconId: string) {
    activeMenuId = activeMenuId === iconId ? '' : iconId;
  }

  function closeIconMenu() {
    activeMenuId = '';
  }

  async function copyIconUrl(icon: IconEntry) {
    activeMenuId = '';

    try {
      await copyText(icon.url);
      toast.info('Raw URL 已复制');
    } catch {
      toast.error('复制 Raw URL 失败');
    }
  }

  async function copySelectedUrls() {
    if (selectedIcons.length === 0) return;

    try {
      await copyText(selectedIcons.map((icon) => icon.url).join('\n'));
      toast.info(`已复制 ${selectedIcons.length} 个 Raw URL`);
    } catch {
      toast.error('批量复制失败');
    }
  }

  async function copyManifestUrl() {
    if (!manifestUrl) return;

    try {
      await copyText(manifestUrl);
      manifestUrlCopied = true;
      toast.info('Manifest URL 已复制');
      window.setTimeout(() => {
        manifestUrlCopied = false;
      }, 1600);
    } catch {
      toast.error('复制 Manifest URL 失败');
    }
  }

  function openPreview(icon: IconEntry) {
    previewIcon = icon;
    activeMenuId = '';
  }

  function closePreview() {
    previewIcon = null;
  }

  function copyPreviewUrl() {
    if (!previewIcon) return;
    void copyIconUrl(previewIcon);
  }

  function startPreviewRename() {
    if (!previewIcon) return;
    startRename(previewIcon);
    closePreview();
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
      return;
    }
    closeIconMenu();
  }

  function goToManagerPage(pageNumber: number) {
    managerPage = Math.min(Math.max(pageNumber, 1), managerTotalPages);
  }

  /// 校验图标名称只能包含英文字母、空格和 .-_。
  function isValidIconName(value: string) {
    const name = value.trim();
    return (
      name.length > 0 &&
      name.length <= 120 &&
      !value.endsWith(' ') &&
      iconNamePattern.test(name)
    );
  }

  /// 返回图标名称校验失败时的展示文案。
  function iconNameError(value: string) {
    if (value.endsWith(' ')) return '图标名称最后不能是空格';
    return '图标名称只能包含英文字母、数字、空格和 .、-、_';
  }

  /// 格式化文件体积。
  function formatBytes(bytes: number) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  onMount(async () => {
    if (await guardSession()) {
      await refreshManifest();
    }
  });
</script>

<svelte:window on:click={closeIconMenu} on:keydown={handleWindowKeydown} />

{#if loading}
  <div class="notice">正在读取 manifest.json...</div>
{:else if error && !manifest}
  <div class="notice error">{error}</div>
{:else if manifest}
  <nav class="breadcrumb" aria-label="面包屑">
    <a href="/">图标集合</a>
    <span>/</span>
    <a href="/console">控制台</a>
    <span>/</span>
    <strong>{manifest.name}</strong>
  </nav>

  <section class="set-admin-hero panel panel-pad">
    <div class="hero-copy">
      <span class="eyebrow">Console / {manifest.id}</span>
      <h1>{manifest.name}</h1>
      <p>{manifest.icons.length} icons · manifest.json</p>

      <div class="hero-actions">
        <a class="action secondary" href="/console">返回控制台</a>
        <button
          class="action secondary"
          type="button"
          on:click={copyManifestUrl}
        >
          {manifestUrlCopied ? '已复制 Manifest' : '复制 Manifest URL'}
        </button>
      </div>
    </div>

    <form class="hero-meta-form" on:submit|preventDefault={submitMeta}>
      <div class="hero-meta-head">
        <span class="eyebrow">Manifest Meta</span>
        <button class="action" type="submit" disabled={savingMeta}>
          {savingMeta ? '保存中...' : '保存信息'}
        </button>
      </div>
      <label class="field">
        <span>名称</span>
        <input class="input" bind:value={metaForm.name} />
      </label>
      <label class="field">
        <span>描述</span>
        <textarea
          class="textarea hero-textarea"
          bind:value={metaForm.description}
        ></textarea>
      </label>
    </form>
  </section>

  <section class="upload-workbench">
    <div class="workbench-head">
      <div>
        <span class="eyebrow">Upload</span>
        <h2>上传工作台</h2>
        <p>上传和管理分开处理，避免在图片列表里混杂新增流程。</p>
      </div>
    </div>

    <div class="manage-grid">
      <form
        class="panel panel-pad upload-card"
        on:submit|preventDefault={submitUpload}
      >
        <span class="eyebrow">Upload</span>
        <h2>单张上传</h2>
        <label class="field">
          <span>图标名称</span>
          <input
            class="input"
            bind:value={uploadName}
            maxlength="120"
            pattern="[A-Za-z0-9 ._-]*"
            placeholder="留空则从文件名提取"
            title="只能包含英文字母、数字、空格和 .、-、_"
          />
          <small>只允许英文字母、数字、空格和 .、-、_</small>
        </label>
        <label class="file-drop">
          <input
            type="file"
            accept="image/png,image/jpeg,image/webp,image/svg+xml"
            on:change={handleFileChange}
          />
          <strong>{selectedFile ? selectedFile.name : '选择图片文件'}</strong>
          <span>支持 png / jpg / webp / svg</span>
        </label>
        <button
          class="action"
          type="submit"
          disabled={uploading || !selectedFile}
        >
          {uploading ? '上传中...' : '上传到 GitHub'}
        </button>
      </form>

      <form
        class="panel panel-pad batch-card"
        on:submit|preventDefault={submitBatchUpload}
      >
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
          <strong
            >{batchFiles.length > 0
              ? `${batchFiles.length} 个图片文件`
              : '多选图片文件'}</strong
          >
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
          disabled={batchUploading ||
            batchTooLarge ||
            (batchFiles.length === 0 && !archiveFile)}
        >
          {batchUploading ? '批量上传中...' : '批量上传到 GitHub'}
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
      <button class="action secondary" type="button" on:click={refreshManifest}
        >刷新</button
      >
    </div>

    <div class="manager-toolbar">
      <label class="field manager-search">
        <span>搜索图片</span>
        <input
          class="input"
          bind:value={managerQuery}
          placeholder="输入 name / path / URL"
        />
      </label>

      <label class="field manager-sort">
        <span>排序</span>
        <select class="input" bind:value={managerSort}>
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
        {#each pagedIcons as icon}
          <article
            class:selected={isIconSelected(icon.id)}
            class="admin-icon-card"
          >
            <div class="card-topline">
              <label class="select-control" aria-label={`选择 ${icon.name}`}>
                <input
                  type="checkbox"
                  checked={isIconSelected(icon.id)}
                  on:change={() => toggleIconSelection(icon.id)}
                />
                <span class:checked={isIconSelected(icon.id)} aria-hidden="true"
                ></span>
              </label>

              <button
                class="thumb"
                type="button"
                aria-label={`查看 ${icon.name} 大图`}
                on:click={() => openPreview(icon)}
              >
                <img src={icon.url} alt={icon.name} loading="lazy" />
              </button>

              <div class="icon-menu-wrap">
                <button
                  class="icon-menu-button"
                  type="button"
                  aria-haspopup="menu"
                  aria-expanded={activeMenuId === icon.id}
                  aria-label={`${icon.name} 操作菜单`}
                  on:click|stopPropagation={() => toggleIconMenu(icon.id)}
                >
                  <span></span>
                  <span></span>
                  <span></span>
                </button>

                {#if activeMenuId === icon.id}
                  <div class="icon-menu" role="menu">
                    <button
                      type="button"
                      role="menuitem"
                      on:click={() => openPreview(icon)}>查看大图</button
                    >
                    <button
                      type="button"
                      role="menuitem"
                      on:click={() => copyIconUrl(icon)}>复制 Raw URL</button
                    >
                    <button
                      type="button"
                      role="menuitem"
                      on:click={() => startRename(icon)}>重命名</button
                    >
                    <button
                      class="danger-item"
                      type="button"
                      role="menuitem"
                      on:click={() => openDeleteIconModal(icon)}
                    >
                      删除
                    </button>
                  </div>
                {/if}
              </div>
            </div>

            <div class="icon-card-body">
              <button
                class="icon-name-button"
                type="button"
                title="打开重命名弹窗"
                on:click={() => openRenameModal(icon)}
              >
                {icon.name}
              </button>
              <code title={icon.path}>{icon.path}</code>
            </div>
          </article>
        {/each}
      </div>
    {/if}

    {#if manifest.icons.length > 0}
      <div class="manager-footer">
        {#if selectedCount > 0}
          <div class="selection-bar">
            <div class="selection-count">
              <strong>{selectedCount}</strong>
              <span>已选择</span>
            </div>
            <div class="batch-actions" aria-label="批量操作">
              <button
                class="action secondary"
                type="button"
                on:click={copySelectedUrls}>复制 Raw URL</button
              >
              <button
                class="action danger"
                type="button"
                on:click={openBulkDeleteModal}>批量删除</button
              >
              <button
                class="selection-clear"
                type="button"
                on:click={clearIconSelection}>清空选择</button
              >
            </div>
          </div>
        {/if}

        <div class="pagination-bar">
          <button
            class="action secondary page-select-action"
            type="button"
            disabled={pagedIcons.length === 0}
            on:click={togglePageSelection}
          >
            {allPageSelected ? '取消本页' : '选择本页'}
          </button>

          <div class="pagination-controls">
            <label class="manager-page-size" aria-label="每页显示数量">
              <select class="input" bind:value={managerPageSize}>
                {#each managerPageSizeOptions as option}
                  <option value={option}>{option}项 / 页</option>
                {/each}
              </select>
            </label>

            <span class="page-range">
              {visibleIcons.length === 0
                ? '0 / 0'
                : `${managerPageStart + 1}-${managerPageEnd} / ${visibleIcons.length}`}
            </span>

            {#if managerTotalPages > 1}
              <div class="pager-strip">
                <button
                  class="mini-action"
                  type="button"
                  disabled={managerPage === 1}
                  on:click={() => goToManagerPage(managerPage - 1)}
                >
                  上一页
                </button>
                <strong>{managerPage} / {managerTotalPages}</strong>
                <button
                  class="mini-action"
                  type="button"
                  disabled={managerPage === managerTotalPages}
                  on:click={() => goToManagerPage(managerPage + 1)}
                >
                  下一页
                </button>
              </div>
            {/if}
          </div>
        </div>
      </div>
    {/if}
  </section>
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
          aria-label="关闭预览"
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
        <div class="preview-actions">
          <button
            class="action secondary"
            type="button"
            on:click={copyPreviewUrl}
          >
            复制 Raw URL
          </button>
          <button class="action" type="button" on:click={startPreviewRename}>
            重命名
          </button>
        </div>
      </footer>
    </div>
  </div>
{/if}

{#if renameIconTarget}
  <div
    class="rename-backdrop"
    role="presentation"
    on:click={handleRenameBackdropClick}
  >
    <div
      class="rename-modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="rename-modal-title"
    >
      <form class="rename-form" on:submit|preventDefault={confirmRenameIcon}>
        <header>
          <div>
            <span class="eyebrow">Rename Icon</span>
            <h2 id="rename-modal-title">重命名图片</h2>
          </div>
          <button
            class="preview-close"
            type="button"
            aria-label="关闭重命名弹窗"
            disabled={renamingIcon}
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
            autocomplete="off"
            spellcheck="false"
            disabled={renamingIcon}
            title="只能包含英文字母、数字、空格和 .、-、_"
          />
          <small>保存后 path 和 Raw URL 会跟随新的 name 更新。</small>
        </label>

        <div class="rename-modal-actions">
          <button
            class="action secondary"
            type="button"
            disabled={renamingIcon}
            on:click={closeRenameModal}>取消</button
          >
          <button class="action" type="submit" disabled={renamingIcon}>
            {renamingIcon ? '保存中...' : '保存重命名'}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<DeleteConfirmModal
  open={bulkDeleteOpen}
  eyebrow="Bulk Delete"
  title="批量删除图标"
  target={selectedIcons.map((icon) => icon.name).join(' / ')}
  description="这个操作会从当前集合移除已选择的图标记录，并删除 GitHub 仓库中的对应文件。"
  impact={manifest
    ? [
        `所属集合：${manifest.name}（/${manifest.id}）`,
        `删除数量：${selectedCount} 个`,
        '删除后需要从 GitHub 历史或备份中恢复。'
      ]
    : []}
  confirmLabel={bulkDeleteConfirmLabel}
  confirmHint={`输入 ${bulkDeleteConfirmLabel} 继续删除`}
  actionLabel="永久删除已选图标"
  submitting={deletingIcon}
  onCancel={closeBulkDeleteModal}
  onConfirm={confirmBulkDeleteIcons}
/>

<DeleteConfirmModal
  open={deleteIconTarget !== null}
  eyebrow="Delete Icon"
  title="删除图标"
  target={deleteIconTarget ? deleteIconTarget.name : ''}
  description="这个操作会从当前集合移除图标记录，并删除 GitHub 仓库中的对应文件。请先确认影响范围，再输入图标名称执行删除。"
  impact={deleteIconTarget && manifest
    ? [
        `所属集合：${manifest.name}（/${manifest.id}）`,
        `文件路径：${deleteIconTarget.path || '未记录路径'}`,
        '删除后需要从 GitHub 历史或备份中恢复。'
      ]
    : []}
  confirmLabel={deleteIconTarget?.name ?? ''}
  confirmHint={deleteIconTarget ? `输入 ${deleteIconTarget.name} 继续删除` : ''}
  actionLabel="永久删除图标"
  submitting={deletingIcon}
  onCancel={closeDeleteIconModal}
  onConfirm={confirmDeleteIcon}
/>

<style>
  .set-admin-hero {
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(380px, 0.64fr);
    gap: 32px;
    align-items: stretch;
  }

  .hero-copy {
    display: flex;
    min-width: 0;
    flex-direction: column;
  }

  .hero-meta-form {
    display: grid;
    gap: 14px;
    align-content: start;
    min-width: 0;
    padding-left: 28px;
    border-left: 1px solid rgba(246, 239, 217, 0.14);
  }

  .hero-meta-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .hero-meta-head .action {
    min-height: 40px;
    padding: 9px 14px;
    box-shadow: none;
  }

  .hero-textarea {
    min-height: 92px;
  }

  h1,
  h2 {
    margin: 0;
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    letter-spacing: 0;
  }

  h1 {
    margin-top: 12px;
    font-size: 72px;
    line-height: 0.94;
  }

  h2 {
    font-size: 42px;
  }

  p {
    margin: 14px 0 0;
    color: rgba(246, 239, 217, 0.68);
  }

  .hero-actions,
  .manager-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .hero-actions {
    justify-content: flex-start;
    margin-top: auto;
    padding-top: 28px;
  }

  .upload-workbench {
    display: grid;
    gap: 14px;
  }

  .workbench-head {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: 14px;
  }

  .workbench-head p {
    margin-top: 10px;
    color: rgba(246, 239, 217, 0.58);
    font-size: 13px;
  }

  .manage-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px;
    align-items: stretch;
  }

  .upload-card,
  .batch-card,
  .icon-manager {
    display: grid;
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

  .file-drop span {
    color: rgba(246, 239, 217, 0.58);
  }

  .batch-total {
    color: rgba(246, 239, 217, 0.68);
    font-size: 13px;
  }

  .batch-total.over-limit {
    color: #ff6b4a;
  }

  .manager-head p {
    margin-top: 10px;
    color: rgba(246, 239, 217, 0.54);
    font-size: 13px;
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

  .manager-toolbar .field {
    min-width: 0;
  }

  .manager-footer {
    display: grid;
    grid-template-columns: 1fr;
    gap: 12px;
    padding: 12px;
    border: 1px solid rgba(246, 239, 217, 0.12);
    border-radius: 10px;
    background: rgba(246, 239, 217, 0.035);
  }

  .manager-page-size .input {
    min-height: 38px;
    min-width: 0;
  }

  .selection-bar {
    display: flex;
    align-items: center;
    gap: 12px;
    min-height: 58px;
    padding: 8px 10px;
    border: 1px solid rgba(198, 255, 72, 0.22);
    border-radius: 8px;
    background:
      linear-gradient(90deg, rgba(198, 255, 72, 0.12), transparent 64%),
      rgba(12, 13, 11, 0.78);
  }

  .selection-count {
    display: inline-flex;
    align-items: baseline;
    gap: 8px;
    min-width: 96px;
  }

  .selection-count strong {
    color: #c6ff48;
    font-size: 28px;
    line-height: 1;
  }

  .selection-count span {
    color: rgba(246, 239, 217, 0.58);
    font-size: 12px;
    font-weight: 800;
    text-transform: uppercase;
  }

  .batch-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-left: auto;
    min-width: 0;
  }

  .selection-bar .action {
    min-height: 34px;
    padding: 8px 12px;
    box-shadow: none;
  }

  .selection-clear {
    border: 0;
    color: rgba(246, 239, 217, 0.58);
    background: transparent;
    font-size: 12px;
    font-weight: 800;
  }

  .pagination-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    min-width: 0;
  }

  .page-select-action {
    flex: 0 0 auto;
    min-height: 38px;
    box-shadow: none;
  }

  .pagination-controls {
    display: flex;
    align-items: end;
    justify-content: flex-end;
    gap: 10px;
    min-width: 0;
  }

  .pagination-controls .manager-page-size {
    width: 126px;
  }

  .page-range {
    min-width: 88px;
    color: rgba(246, 239, 217, 0.58);
    font-size: 12px;
    font-weight: 800;
    line-height: 38px;
    text-align: right;
  }

  .pager-strip {
    display: inline-flex;
    align-items: center;
    gap: 10px;
  }

  .pager-strip strong {
    color: #f6efd9;
    font-size: 12px;
    line-height: 38px;
    white-space: nowrap;
  }

  .manager-footer .action:disabled,
  .selection-clear:disabled,
  .mini-action:disabled {
    cursor: not-allowed;
    opacity: 0.42;
  }

  .selection-clear:hover,
  .selection-clear:focus-visible {
    color: #f6efd9;
  }

  .admin-icon-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 14px;
  }

  .admin-icon-card {
    position: relative;
    display: grid;
    gap: 12px;
    min-width: 0;
    padding: 12px;
    border: 1px solid rgba(246, 239, 217, 0.16);
    border-radius: 8px;
    background:
      linear-gradient(180deg, rgba(246, 239, 217, 0.05), transparent 48%),
      rgba(12, 13, 11, 0.62);
  }

  .admin-icon-card.selected {
    border-color: rgba(198, 255, 72, 0.48);
    box-shadow: 0 0 0 1px rgba(198, 255, 72, 0.08) inset;
  }

  .card-topline {
    min-width: 0;
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

  .thumb img {
    max-width: 112px;
    max-height: 112px;
    object-fit: contain;
  }

  .thumb:hover,
  .thumb:focus-visible {
    border-color: rgba(198, 255, 72, 0.36);
    outline: none;
  }

  .select-control {
    position: absolute;
    top: 20px;
    left: 20px;
    z-index: 2;
    display: grid;
    width: 30px;
    height: 30px;
    place-items: center;
  }

  .select-control input {
    position: absolute;
    opacity: 0;
  }

  .select-control span {
    position: relative;
    display: grid;
    width: 22px;
    height: 22px;
    place-items: center;
    border: 1px solid rgba(246, 239, 217, 0.34);
    border-radius: 6px;
    color: transparent;
    background: rgba(12, 13, 11, 0.78);
    box-shadow: 0 8px 20px rgba(0, 0, 0, 0.34);
    font-size: 15px;
    font-weight: 900;
    line-height: 1;
    transition:
      border-color 140ms ease,
      background 140ms ease,
      box-shadow 140ms ease;
  }

  .select-control span.checked {
    border-color: #c6ff48;
    background: #c6ff48;
    box-shadow:
      inset 0 0 0 2px rgba(246, 239, 217, 0.24),
      0 0 0 2px rgba(198, 255, 72, 0.18),
      0 8px 20px rgba(0, 0, 0, 0.34);
  }

  .select-control span.checked::after {
    position: absolute;
    width: 10px;
    height: 6px;
    border-bottom: 3px solid #0c0d0b;
    border-left: 3px solid #0c0d0b;
    content: '';
    transform: translateY(-1px) rotate(-45deg);
  }

  .icon-menu-wrap {
    position: absolute;
    top: 18px;
    right: 18px;
    z-index: 4;
  }

  .icon-menu-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 34px;
    height: 34px;
    gap: 3px;
    border: 1px solid rgba(246, 239, 217, 0.18);
    border-radius: 8px;
    color: #f6efd9;
    background: rgba(12, 13, 11, 0.76);
    box-shadow: 0 10px 24px rgba(0, 0, 0, 0.26);
  }

  .icon-menu-button span {
    display: block;
    width: 4px;
    height: 4px;
    border-radius: 999px;
    background: currentColor;
  }

  .icon-menu-button:hover,
  .icon-menu-button:focus-visible {
    border-color: rgba(198, 255, 72, 0.5);
    color: #c6ff48;
    outline: none;
  }

  .icon-menu {
    position: absolute;
    top: 40px;
    right: 0;
    z-index: 8;
    display: grid;
    width: 150px;
    padding: 6px;
    border: 1px solid rgba(246, 239, 217, 0.18);
    border-radius: 8px;
    background: rgba(18, 19, 16, 0.96);
    box-shadow: 0 22px 52px rgba(0, 0, 0, 0.48);
  }

  .icon-menu button {
    min-height: 34px;
    border: 0;
    border-radius: 6px;
    color: rgba(246, 239, 217, 0.78);
    background: transparent;
    text-align: left;
    font-size: 12px;
    font-weight: 800;
  }

  .icon-menu button:hover,
  .icon-menu button:focus-visible {
    color: #f6efd9;
    background: rgba(246, 239, 217, 0.08);
    outline: none;
  }

  .icon-menu .danger-item {
    color: #ff8b6d;
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
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 22px;
    font-weight: 800;
  }

  .icon-name-button:hover,
  .icon-name-button:focus-visible {
    color: #c6ff48;
    outline: none;
  }

  .mini-action {
    min-height: 34px;
    min-width: 72px;
    padding: 0 10px;
    border: 1px solid rgba(246, 239, 217, 0.2);
    border-radius: 8px;
    color: #f6efd9;
    background: rgba(246, 239, 217, 0.06);
    font-size: 12px;
    font-weight: 900;
  }

  .mini-action:disabled {
    cursor: not-allowed;
    opacity: 0.58;
  }

  code {
    overflow: hidden;
    color: rgba(246, 239, 217, 0.52);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .image-preview-backdrop,
  .rename-backdrop {
    position: fixed;
    inset: 0;
    z-index: 980;
    display: grid;
    place-items: center;
    padding: 20px;
    background: rgba(0, 0, 0, 0.72);
    backdrop-filter: blur(14px);
  }

  .image-preview {
    display: grid;
    gap: 16px;
    width: min(760px, 100%);
    max-height: min(820px, calc(100vh - 40px));
    padding: clamp(18px, 3vw, 28px);
    overflow: auto;
    border: 1px solid rgba(246, 239, 217, 0.2);
    border-radius: 12px;
    background:
      linear-gradient(135deg, rgba(198, 255, 72, 0.08), transparent 42%),
      #10110e;
    box-shadow: 0 32px 90px rgba(0, 0, 0, 0.62);
  }

  .rename-modal {
    width: min(620px, 100%);
    padding: clamp(18px, 3vw, 28px);
    border: 1px solid rgba(198, 255, 72, 0.26);
    border-radius: 12px;
    color: #f6efd9;
    background:
      linear-gradient(135deg, rgba(198, 255, 72, 0.1), transparent 38%), #10110e;
    box-shadow: 0 32px 90px rgba(0, 0, 0, 0.62);
  }

  .rename-form {
    display: grid;
    gap: 18px;
  }

  .rename-modal header,
  .rename-modal-actions,
  .rename-target {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    min-width: 0;
  }

  .rename-modal h2 {
    margin-top: 8px;
  }

  .rename-target {
    justify-content: flex-start;
    padding: 12px;
    border: 1px solid rgba(246, 239, 217, 0.12);
    border-radius: 8px;
    background: rgba(246, 239, 217, 0.045);
  }

  .rename-target strong {
    display: block;
    margin-bottom: 6px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 18px;
  }

  .rename-thumb {
    display: grid;
    flex: 0 0 auto;
    width: 66px;
    height: 66px;
    place-items: center;
    border: 1px solid rgba(246, 239, 217, 0.12);
    border-radius: 8px;
    background: rgba(0, 0, 0, 0.24);
  }

  .rename-thumb img {
    max-width: 48px;
    max-height: 48px;
    object-fit: contain;
  }

  .rename-modal .field small {
    color: rgba(246, 239, 217, 0.5);
    font-size: 12px;
  }

  .rename-modal-actions {
    justify-content: flex-end;
  }

  .rename-modal-actions .action {
    min-width: 132px;
  }

  .image-preview header,
  .image-preview footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    min-width: 0;
  }

  .image-preview h2 {
    max-width: 560px;
    margin-top: 8px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .preview-close {
    display: grid;
    flex: 0 0 auto;
    width: 42px;
    height: 42px;
    place-items: center;
    border: 1px solid rgba(246, 239, 217, 0.2);
    border-radius: 8px;
    color: #f6efd9;
    background: rgba(246, 239, 217, 0.06);
    font-size: 26px;
    line-height: 1;
  }

  .preview-close:hover,
  .preview-close:focus-visible {
    border-color: rgba(198, 255, 72, 0.5);
    color: #c6ff48;
    outline: none;
  }

  .preview-stage {
    display: grid;
    min-height: min(52vh, 460px);
    place-items: center;
    padding: clamp(18px, 4vw, 34px);
    border: 1px solid rgba(246, 239, 217, 0.12);
    border-radius: 8px;
    background:
      repeating-linear-gradient(
        45deg,
        rgba(246, 239, 217, 0.055) 0 10px,
        rgba(246, 239, 217, 0.025) 10px 20px
      ),
      rgba(0, 0, 0, 0.28);
  }

  .preview-stage img {
    max-width: 100%;
    max-height: min(48vh, 430px);
    object-fit: contain;
  }

  .image-preview footer code {
    min-width: 0;
  }

  .preview-actions {
    display: flex;
    flex: 0 0 auto;
    gap: 10px;
  }

  @media (max-width: 920px) {
    .set-admin-hero,
    .hero-actions,
    .manager-head {
      align-items: stretch;
      flex-direction: column;
    }

    .set-admin-hero {
      grid-template-columns: 1fr;
    }

    .hero-meta-form {
      padding-left: 0;
      padding-top: 20px;
      border-top: 1px solid rgba(246, 239, 217, 0.14);
      border-left: 0;
    }

    .hero-meta-head {
      display: contents;
    }

    .hero-meta-head .eyebrow {
      order: 0;
    }

    .hero-meta-form > .field {
      order: 1;
    }

    .hero-meta-head .action {
      order: 2;
      width: 100%;
      margin-top: 6px;
    }

    .manage-grid {
      grid-template-columns: 1fr;
    }

    .manager-toolbar,
    .manager-footer {
      grid-template-columns: 1fr;
    }

    h1 {
      font-size: 48px;
    }

    h2 {
      font-size: 34px;
    }
  }

  @media (max-width: 680px) {
    .admin-icon-grid {
      grid-template-columns: 1fr;
    }

    .admin-icon-card {
      grid-template-columns: 72px minmax(0, 1fr);
      align-items: center;
      padding: 10px 52px 10px 10px;
    }

    .card-topline {
      grid-column: 1;
      grid-row: 1;
    }

    .thumb {
      min-height: 72px;
      padding: 8px;
    }

    .thumb img {
      max-width: 54px;
      max-height: 54px;
    }

    .select-control {
      top: 14px;
      left: 14px;
    }

    .select-control span {
      width: 18px;
      height: 18px;
      border-radius: 5px;
      font-size: 14px;
    }

    .select-control span.checked::after {
      width: 8px;
      height: 5px;
      border-bottom-width: 2px;
      border-left-width: 2px;
    }

    .icon-menu-wrap {
      top: 18px;
      right: 12px;
    }

    .icon-card-body {
      grid-column: 2;
      grid-row: 1;
      align-self: center;
    }

    .icon-name-button {
      font-size: 18px;
    }

    .selection-bar {
      align-items: stretch;
      flex-direction: column;
    }

    .selection-count {
      min-width: 0;
    }

    .batch-actions {
      display: grid;
      grid-template-columns: 1fr;
      margin-left: 0;
    }

    .selection-bar .action,
    .selection-clear {
      grid-column: 1 / -1;
      width: 100%;
    }

    .pagination-bar {
      display: grid;
      grid-template-areas:
        'select size'
        'range range'
        'pager pager';
      grid-template-columns: minmax(0, 1fr) minmax(126px, auto);
      gap: 8px;
      align-items: center;
    }

    .pagination-controls {
      display: contents;
    }

    .page-select-action {
      grid-area: select;
      width: 100%;
      min-height: 40px;
    }

    .pagination-controls .manager-page-size {
      grid-area: size;
      width: 100%;
    }

    .page-range {
      grid-area: range;
      min-width: 0;
      padding-top: 2px;
      line-height: 1.1;
      text-align: center;
    }

    .pager-strip {
      grid-area: pager;
      display: grid;
      grid-template-columns: minmax(0, 1fr) auto minmax(0, 1fr);
      gap: 8px;
      align-items: center;
    }

    .pager-strip strong {
      min-width: 52px;
      line-height: 36px;
      text-align: center;
    }

    .pager-strip .mini-action {
      width: 100%;
      min-height: 36px;
    }

    .image-preview header,
    .image-preview footer,
    .rename-modal header,
    .rename-modal-actions,
    .rename-target {
      align-items: stretch;
      flex-direction: column;
    }

    .image-preview h2 {
      max-width: 100%;
      white-space: normal;
    }

    .preview-actions {
      flex-direction: column;
    }

    .rename-modal-actions .action {
      width: 100%;
    }
  }
</style>
