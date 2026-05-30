<script lang="ts">
  import { goto } from '$app/navigation';
  import { onMount } from 'svelte';
  import DeleteConfirmModal from '$lib/DeleteConfirmModal.svelte';
  import { createSet, deleteSet, getSession, listSets, logout } from '$lib/api';
  import { toast } from '$lib/toast';
  import type { IconSetSummary } from '$lib/types';

  let sets: IconSetSummary[] = [];
  let loading = true;
  let saving = false;
  let deleting = false;
  let listError = '';
  let deleteTarget: IconSetSummary | null = null;
  let newSet = {
    id: '',
    name: '',
    description: ''
  };

  /// 校验当前管理员会话，未登录时跳转登录页。
  async function guardSession() {
    const session = await getSession();
    if (!session.authenticated) {
      await goto('/admin/login');
      return false;
    }
    return true;
  }

  /// 加载后台集合列表。
  async function refreshSets() {
    loading = true;
    listError = '';

    try {
      sets = await listSets();
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
      await goto(`/admin/sets/${created.id}`);
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

  /// 退出管理员后台。
  async function submitLogout() {
    await logout();
    await goto('/admin/login');
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
      await refreshSets();
    }
  });
</script>

<nav class="breadcrumb" aria-label="面包屑">
  <a href="/">图标集合</a>
  <span>/</span>
  <strong>图标后台</strong>
</nav>

<section class="admin-hero panel panel-pad">
  <div>
    <span class="eyebrow">Control Room</span>
    <h1>图标后台</h1>
    <p>创建 set、进入集合上传图片、编辑 name 或删除资源。所有写入都会提交到 GitHub 仓库。</p>
  </div>
  <button class="action secondary" type="button" on:click={submitLogout}>退出登录</button>
</section>

<section class="admin-grid">
  <form class="panel panel-pad create-card" on:submit|preventDefault={submitCreateSet}>
    <span class="eyebrow">New Set</span>
    <h2>创建集合</h2>

    <label class="field">
      <span>集合 ID</span>
      <input class="input" bind:value={newSet.id} placeholder="emby" />
    </label>

    <label class="field">
      <span>集合名称</span>
      <input class="input" bind:value={newSet.name} placeholder="Emby图标库@seven" />
    </label>

    <label class="field">
      <span>描述</span>
      <textarea class="textarea" bind:value={newSet.description} placeholder="这个集合的用途和命名规则"></textarea>
    </label>

    <div class="form-actions">
      <button class="action secondary" type="button" on:click={fillSuggestedId}>生成 ID</button>
      <button class="action" type="submit" disabled={saving || !newSet.id || !newSet.name}>
        {saving ? '创建中...' : '创建集合'}
      </button>
    </div>
  </form>

  <div class="panel panel-pad list-card">
    <div class="list-head">
      <div>
        <span class="eyebrow">Sets</span>
        <h2>已登记集合</h2>
      </div>
      <button class="action secondary" type="button" on:click={refreshSets}>刷新</button>
    </div>

    {#if loading}
      <div class="notice">正在读取 sets.json...</div>
    {:else if listError}
      <div class="notice error">{listError}</div>
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
              <a class="action secondary" href={`/admin/sets/${set.id}`}>管理</a>
              <button class="action danger" type="button" on:click={() => openDeleteSetModal(set)}>
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
    letter-spacing: -0.06em;
  }

  h1 {
    margin-top: 12px;
    font-size: clamp(48px, 8vw, 104px);
    line-height: 0.9;
  }

  h2 {
    font-size: clamp(32px, 4vw, 54px);
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
    gap: 18px;
    align-items: start;
  }

  .create-card,
  .list-card {
    display: grid;
    gap: 18px;
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
    border-radius: 20px;
    background: rgba(246, 239, 217, 0.04);
  }

  .set-row strong,
  .set-row span {
    display: block;
  }

  .set-row strong {
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: 24px;
    letter-spacing: -0.04em;
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
  }
</style>
