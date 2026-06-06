<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import DeleteConfirmModal from '$lib/DeleteConfirmModal.svelte';
  import {
    createSet,
    deleteSet,
    getRepoConfig,
    getSession,
    listAdminSets,
    saveRepoConfig
  } from '$lib/api';
  import { toast } from '$lib/toast';
  import type { IconSetSummary, RepoConfig, UserProfile } from '$lib/types';

  let sets: IconSetSummary[] = [];
  let sessionUser: UserProfile | null = null;
  let repoConfig: RepoConfig | null = null;
  let loading = true;
  let saving = false;
  let savingConfig = false;
  let deleting = false;
  let listError = '';
  let deleteTarget: IconSetSummary | null = null;
  let newSet = {
    id: '',
    name: '',
    description: ''
  };
  let repoForm = {
    owner: '',
    repo: '',
    branch: 'main',
    token: ''
  };

  $: repoConfigured = Boolean(repoConfig?.configured);

  /// 校验当前管理员会话，未登录时跳转登录页。
  async function guardSession() {
    const session = await getSession();
    if (!session.authenticated) {
      await goto('/console/login');
      return false;
    }
    sessionUser = session.user ?? null;
    return true;
  }

  /// 加载当前用户的 GitHub 仓库配置。
  async function refreshRepoConfig() {
    repoConfig = await getRepoConfig();
    repoForm = {
      owner: repoConfig.owner || sessionUser?.login || '',
      repo: repoConfig.repo || '',
      branch: repoConfig.branch || 'main',
      token: ''
    };
  }

  /// 保存当前用户的 GitHub 仓库配置。
  async function submitRepoConfig() {
    savingConfig = true;

    try {
      repoConfig = await saveRepoConfig(repoForm);
      repoForm.token = '';
      toast.info('GitHub 仓库配置已保存');
      await refreshSets();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : '仓库配置保存失败');
    } finally {
      savingConfig = false;
    }
  }

  /// 加载控制台集合列表。
  async function refreshSets() {
    loading = true;
    listError = '';

    try {
      if (!repoConfigured) {
        sets = [];
        return;
      }
      sets = await listAdminSets();
    } catch (err) {
      const message = err instanceof Error ? err.message : '集合加载失败';
      if (sets.length > 0) {
        toast.error(message);
      } else {
        listError = message;
      }
    } finally {
      loading = false;
    }
  }

  /// 创建新图标集合并跳转到集合管理页。
  async function submitCreateSet() {
    saving = true;

    try {
      const created = await createSet(newSet);
      newSet = { id: '', name: '', description: '' };
      await goto(`/console/sets/${created.id}`);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : '创建集合失败');
    } finally {
      saving = false;
    }
  }

  /// 根据集合名称生成推荐 ID。
  function fillSuggestedId() {
    newSet.id = slugifyClient(newSet.name || newSet.id);
  }

  /// 打开删除集合确认弹窗。
  function openDeleteSetModal(set: IconSetSummary) {
    deleteTarget = set;
  }

  /// 关闭删除集合确认弹窗。
  function closeDeleteSetModal() {
    if (deleting) return;
    deleteTarget = null;
  }

  /// 删除空集合或不再需要的集合。
  async function confirmDeleteSet() {
    if (!deleteTarget) return;
    deleting = true;

    try {
      const deletedName = deleteTarget.name;
      sets = await deleteSet(deleteTarget.id);
      deleteTarget = null;
      toast.info(`集合 ${deletedName} 已删除`);
    } catch (err) {
      toast.error(err instanceof Error ? err.message : '删除集合失败');
    } finally {
      deleting = false;
    }
  }

  /// 在前端生成和后端规则一致的 slug 预览。
  function slugifyClient(value: string) {
    return value
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9_-]+/g, '-')
      .replace(/^-+|-+$/g, '')
      .slice(0, 64);
  }

  onMount(async () => {
    if (await guardSession()) {
      await refreshRepoConfig();
      await refreshSets();
    }
  });
</script>

<nav class="breadcrumb" aria-label="面包屑">
  <a href="/">图标集合</a>
  <span>/</span>
  <strong>控制台</strong>
</nav>

<section class="admin-hero panel panel-pad">
  <div>
    <span class="eyebrow">Control Room</span>
    <h1>控制台</h1>
    <p>
      登录用户拥有独立的 GitHub 仓库配置。所有写入都会提交到当前账号配置的仓库。
    </p>
  </div>
  <a class="guide-link" href="/collab" title="进入协作管理">
    协作管理
  </a>
</section>

<form
  class="panel panel-pad repo-card"
  on:submit|preventDefault={submitRepoConfig}
>
  <div class="list-head">
    <div>
      <span class="eyebrow">Repository</span>
      <h2>GitHub 仓库配置</h2>
      <p>
        {#if repoConfigured}
          当前仓库：{repoConfig?.owner}/{repoConfig?.repo} · {repoConfig?.branch}
        {:else}
          首次使用需要配置一个可写入的 GitHub 仓库。
        {/if}
      </p>
    </div>
    <a
      class="guide-link"
      href="/docs/github-repo-config"
      title="查看 GitHub 仓库配置指引"
    >
      配置指引
    </a>
  </div>

  <div class="repo-form-grid">
    <label class="field">
      <span>Owner</span>
      <input
        class="input"
        bind:value={repoForm.owner}
        placeholder={sessionUser?.login ?? 'github-user'}
      />
    </label>
    <label class="field">
      <span>Repo</span>
      <input
        class="input"
        bind:value={repoForm.repo}
        placeholder="icon-set-assets"
      />
    </label>
    <label class="field">
      <span>Branch</span>
      <input class="input" bind:value={repoForm.branch} placeholder="main" />
    </label>
    <label class="field token-field">
      <span>GitHub Token</span>
      <input
        class="input"
        bind:value={repoForm.token}
        type="password"
        autocomplete="off"
        placeholder={repoConfig?.token_configured
          ? '留空则继续使用已保存 token'
          : 'Contents 读写 token'}
      />
    </label>
  </div>

  <div class="form-actions">
    <span class="repo-hint">Token 会在后端加密存储，不会返回给前端。</span>
    <button
      class="action"
      type="submit"
      disabled={savingConfig ||
        !repoForm.owner ||
        !repoForm.repo ||
        !repoForm.branch}
    >
      {savingConfig ? '保存中...' : '保存仓库配置'}
    </button>
  </div>
</form>

<section class="admin-grid">
  <form
    class="panel panel-pad create-card"
    on:submit|preventDefault={submitCreateSet}
  >
    <span class="eyebrow">New Set</span>
    <h2>创建集合</h2>

    <label class="field">
      <span>集合 ID</span>
      <input class="input" bind:value={newSet.id} placeholder="example_id" />
    </label>

    <label class="field">
      <span>集合名称</span>
      <input
        class="input"
        bind:value={newSet.name}
        placeholder="你的集合名称"
      />
    </label>

    <label class="field">
      <span>描述</span>
      <textarea
        class="textarea"
        bind:value={newSet.description}
        placeholder="这个集合的用途和命名规则"
      ></textarea>
    </label>

    <div class="form-actions">
      <button class="action secondary" type="button" on:click={fillSuggestedId}
        >生成 ID</button
      >
      <button
        class="action"
        type="submit"
        disabled={saving || !repoConfigured || !newSet.id || !newSet.name}
      >
        创建集合
      </button>
    </div>
  </form>

  <div class="panel panel-pad list-card">
    <div class="list-head">
      <div>
        <span class="eyebrow">Sets</span>
        <h2>已登记集合</h2>
      </div>
      <button class="action secondary" type="button" on:click={refreshSets}
        >刷新</button
      >
    </div>

    {#if loading}
      <div class="notice">正在读取 sets.json...</div>
    {:else if listError}
      <div class="notice error">{listError}</div>
    {:else if !repoConfigured}
      <div class="notice">请先保存 GitHub 仓库配置。</div>
    {:else if sets.length === 0}
      <div class="notice">还没有集合，请先创建一个。</div>
    {:else}
      <div class="set-list">
        {#each sets as set}
          <article class="set-row">
            <div>
              <strong>{set.name}</strong>
              <span>/{set.id} · {set.icon_count} icons</span>
              <p>{set.description || '暂无描述'}</p>
            </div>
            <div class="row-actions">
              <a class="action secondary" href={`/console/sets/${set.id}`}
                >管理</a
              >
              <a class="action secondary" href={`/collab?set_id=${set.id}`}
                >协作</a
              >
              <button
                class="action danger"
                type="button"
                on:click={() => openDeleteSetModal(set)}
              >
                删除
              </button>
            </div>
          </article>
        {/each}
      </div>
    {/if}
  </div>
</section>

<DeleteConfirmModal
  open={deleteTarget !== null}
  eyebrow="Delete Set"
  title="删除图标集合"
  target={deleteTarget ? `${deleteTarget.name} / ${deleteTarget.id}` : ''}
  description="这个操作会删除集合配置和它登记的图标文件。请先确认影响范围，再输入集合 ID 执行删除。"
  impact={deleteTarget
    ? [
        `集合 ID：/${deleteTarget.id}`,
        `登记图标：${deleteTarget.icon_count} 个`,
        '删除后需要从 GitHub 历史或备份中恢复。'
      ]
    : []}
  confirmLabel={deleteTarget?.id ?? ''}
  confirmHint={deleteTarget ? `输入 ${deleteTarget.id} 继续删除` : ''}
  actionLabel="永久删除集合"
  submitting={deleting}
  onCancel={closeDeleteSetModal}
  onConfirm={confirmDeleteSet}
/>

<style>
  .admin-hero {
    display: flex;
    align-items: end;
    justify-content: space-between;
    gap: 20px;
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
    max-width: 720px;
    margin: 16px 0 0;
    color: rgba(246, 239, 217, 0.68);
    line-height: 1.8;
  }

  .admin-grid {
    display: grid;
    grid-template-columns: minmax(320px, 0.8fr) minmax(0, 1.2fr);
    gap: 14px;
    align-items: start;
  }

  .repo-card {
    display: grid;
    gap: 14px;
  }

  .repo-form-grid {
    display: grid;
    grid-template-columns: repeat(3, minmax(150px, 1fr));
    gap: 12px;
  }

  .token-field {
    grid-column: 1 / -1;
  }

  .repo-hint {
    color: rgba(246, 239, 217, 0.58);
    font-size: 12px;
    line-height: 1.6;
  }

  .guide-link {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-height: 38px;
    padding: 8px 12px;
    border: 1px solid rgba(198, 255, 72, 0.28);
    border-radius: 10px;
    color: #c6ff48;
    background: rgba(198, 255, 72, 0.07);
    font-size: 12px;
    font-weight: 800;
    white-space: nowrap;
    transition:
      border-color 160ms ease,
      background 160ms ease,
      color 160ms ease;
  }

  .guide-link:hover {
    border-color: rgba(246, 239, 217, 0.36);
    color: #f6efd9;
    background: rgba(246, 239, 217, 0.08);
  }

  .create-card,
  .list-card {
    display: grid;
    gap: 14px;
  }

  .form-actions,
  .list-head,
  .row-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .set-list {
    display: grid;
    gap: 12px;
  }

  .set-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 16px;
    padding: 16px;
    border: 1px solid rgba(246, 239, 217, 0.16);
    border-radius: 8px;
    background: rgba(246, 239, 217, 0.04);
  }

  .set-row strong,
  .set-row span {
    display: block;
  }

  .set-row strong {
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: 24px;
    letter-spacing: 0;
  }

  .set-row span {
    margin-top: 6px;
    color: #c6ff48;
    font-size: 12px;
  }

  .set-row p {
    margin-top: 10px;
    font-size: 13px;
  }

  @media (max-width: 960px) {
    .admin-grid,
    .repo-form-grid,
    .set-row {
      grid-template-columns: 1fr;
    }

    .admin-hero,
    .form-actions,
    .list-head,
    .row-actions {
      align-items: stretch;
      flex-direction: column;
    }

    h1 {
      font-size: 48px;
    }

    h2 {
      font-size: 34px;
    }
  }
</style>
