<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/stores';
  import { onMount } from 'svelte';
  import DeleteConfirmModal from '$lib/DeleteConfirmModal.svelte';
  import {
    getSession,
    getSet,
    removeIcon,
    renameIcon,
    updateSet,
    uploadIcon,
    uploadIconsBatch
  } from '$lib/api';
  import { toast } from '$lib/toast';
  import type { IconEntry, IconManifest } from '$lib/types';

  const batchUploadMaxBytes = 10 * 1024 * 1024;

  let manifest: IconManifest | null = null;
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
  let renameDrafts: Record<string, string> = {};
  const iconNamePattern = /^[A-Za-z0-9 ._-]+$/;

  $: batchTotalBytes =
    batchFiles.reduce((total, file) => total + file.size, 0) + (archiveFile?.size ?? 0);
  $: batchTooLarge = batchTotalBytes > batchUploadMaxBytes;

  /// 校验后台会话，未登录时跳转登录页。
  async function guardSession() {
    const session = await getSession();
    if (!session.authenticated) {
      await goto('/admin/login');
      return false;
    }
    return true;
  }

  /// 读取当前集合 manifest 并初始化表单。
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
      metaForm = {
        name: manifest.name,
        description: manifest.description
      };
      renameDrafts = Object.fromEntries(manifest.icons.map((icon) => [icon.id, icon.name]));
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
      manifest = await uploadIcon(manifest.id, uploadName, selectedFile);
      uploadName = '';
      selectedFile = null;
      renameDrafts = Object.fromEntries(manifest.icons.map((icon) => [icon.id, icon.name]));
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
      manifest = await uploadIconsBatch(manifest.id, batchFiles, archiveFile);
      batchFiles = [];
      archiveFile = null;
      if (batchFilesInput) batchFilesInput.value = '';
      if (archiveInput) archiveInput.value = '';
      renameDrafts = Object.fromEntries(manifest.icons.map((icon) => [icon.id, icon.name]));
      toast.info('批量图片已上传到 GitHub');
    } catch (err) {
      error = err instanceof Error ? err.message : '批量上传失败';
      toast.error(error);
    } finally {
      batchUploading = false;
    }
  }

  /// 保存单个图标名称。
  async function submitRename(iconId: string) {
    if (!manifest) return;
    error = '';

    if (!isValidIconName(renameDrafts[iconId])) {
      error = iconNameError(renameDrafts[iconId]);
      toast.error(error);
      return;
    }

    try {
      manifest = await renameIcon(manifest.id, iconId, renameDrafts[iconId]);
      renameDrafts = Object.fromEntries(manifest.icons.map((icon) => [icon.id, icon.name]));
      toast.info('图标名称已更新');
    } catch (err) {
      error = err instanceof Error ? err.message : '改名失败';
      toast.error(error);
    }
  }

  /// 打开删除图标确认弹窗。
  function openDeleteIconModal(icon: IconEntry) {
    deleteIconTarget = icon;
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
      manifest = await removeIcon(manifest.id, deleteIconTarget.id);
      renameDrafts = Object.fromEntries(manifest.icons.map((icon) => [icon.id, icon.name]));
      deleteIconTarget = null;
      toast.info(`图标 ${deletedName} 已删除`);
    } catch (err) {
      error = err instanceof Error ? err.message : '删除失败';
      toast.error(error);
    } finally {
      deletingIcon = false;
    }
  }

  /// 校验图标名称只能包含英文字母、空格和 .-_。
  function isValidIconName(value: string) {
    const name = value.trim();
    return name.length > 0 && name.length <= 120 && !value.endsWith(' ') && iconNamePattern.test(name);
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

{#if loading}
  <div class="notice">正在读取 manifest.json...</div>
{:else if error && !manifest}
  <div class="notice error">{error}</div>
{:else if manifest}
  <nav class="breadcrumb" aria-label="面包屑">
    <a href="/">图标集合</a>
    <span>/</span>
    <a href="/admin">图标后台</a>
    <span>/</span>
    <strong>{manifest.name}</strong>
  </nav>

  <section class="set-admin-hero panel panel-pad">
    <div class="hero-copy">
      <span class="eyebrow">Admin / {manifest.id}</span>
      <h1>{manifest.name}</h1>
      <p>{manifest.icons.length} icons · manifest.json</p>

      <div class="hero-actions">
        <a class="action secondary" href={`/sets/${manifest.id}`}>查看前台</a>
        <a class="action secondary" href="/admin">返回后台</a>
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
        <textarea class="textarea hero-textarea" bind:value={metaForm.description}></textarea>
      </label>
    </form>
  </section>

  <section class="manage-grid">
    <form class="panel panel-pad upload-card" on:submit|preventDefault={submitUpload}>
      <span class="eyebrow">Upload</span>
      <h2>上传图片</h2>
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
        <input type="file" accept="image/png,image/jpeg,image/webp,image/svg+xml" on:change={handleFileChange} />
        <strong>{selectedFile ? selectedFile.name : '选择图片文件'}</strong>
        <span>支持 png / jpg / webp / svg</span>
      </label>
      <button class="action" type="submit" disabled={uploading || !selectedFile}>
        {uploading ? '上传中...' : '上传到 GitHub'}
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
        {batchUploading ? '批量上传中...' : '批量上传到 GitHub'}
      </button>
    </form>
  </section>

  <section class="panel panel-pad icon-manager">
    <div class="manager-head">
      <div>
        <span class="eyebrow">Icons</span>
        <h2>图片管理</h2>
      </div>
      <button class="action secondary" type="button" on:click={refreshManifest}>刷新</button>
    </div>

    {#if manifest.icons.length === 0}
      <div class="notice">这个集合还没有图片。</div>
    {:else}
      <div class="admin-icon-grid">
        {#each manifest.icons as icon}
          <article class="admin-icon-card">
            <div class="thumb">
              <img src={icon.url} alt={icon.name} loading="lazy" />
            </div>
            <label class="field">
              <span>Name</span>
              <input
                class="input"
                bind:value={renameDrafts[icon.id]}
                maxlength="120"
                pattern="[A-Za-z0-9 ._-]+"
                title="只能包含英文字母、数字、空格和 .、-、_"
              />
            </label>
            <code>{icon.path}</code>
            <div class="icon-actions">
              <button class="action secondary" type="button" on:click={() => submitRename(icon.id)}>
                保存 name
              </button>
              <button class="action danger" type="button" on:click={() => openDeleteIconModal(icon)}>
                删除
              </button>
            </div>
          </article>
        {/each}
      </div>
    {/if}
  </section>
{/if}

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
    gap: clamp(24px, 4vw, 56px);
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
    padding-left: clamp(18px, 3vw, 34px);
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
    letter-spacing: -0.06em;
  }

  h1 {
    margin-top: 12px;
    font-size: clamp(44px, 8vw, 96px);
    line-height: 0.9;
  }

  h2 {
    font-size: clamp(30px, 4vw, 52px);
  }

  p {
    margin: 14px 0 0;
    color: rgba(246, 239, 217, 0.68);
  }

  .hero-actions,
  .icon-actions,
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

  .manage-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 18px;
    align-items: stretch;
  }

  .upload-card,
  .batch-card,
  .icon-manager {
    display: grid;
    gap: 18px;
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
    border-radius: 24px;
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

  .admin-icon-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(250px, 1fr));
    gap: 14px;
  }

  .admin-icon-card {
    display: grid;
    gap: 12px;
    padding: 14px;
    border: 1px solid rgba(246, 239, 217, 0.16);
    border-radius: 22px;
    background: rgba(246, 239, 217, 0.04);
  }

  .thumb {
    display: grid;
    min-height: 160px;
    place-items: center;
    border-radius: 18px;
    background: rgba(0, 0, 0, 0.22);
  }

  .thumb img {
    max-width: 96px;
    max-height: 96px;
    object-fit: contain;
  }

  code {
    overflow: hidden;
    color: rgba(246, 239, 217, 0.52);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  @media (max-width: 920px) {
    .set-admin-hero,
    .hero-actions,
    .manager-head,
    .icon-actions {
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
  }
</style>
