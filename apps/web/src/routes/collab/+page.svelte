<script lang="ts">
  import {
    CalendarClock,
    Copy,
    Eye,
    EyeOff,
    KeyRound,
    Link2,
    Save,
    Settings2,
    ShieldCheck,
    ShieldX,
    Trash2,
    WandSparkles,
    X
  } from '@lucide/svelte';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { onMount } from 'svelte';
  import DeleteConfirmModal from '$lib/DeleteConfirmModal.svelte';
  import {
    createCollabLink,
    deleteCollabLink,
    getSession,
    listAdminSets,
    listCollabLinks,
    revokeAllCollabLinks,
    revokeCollabLink,
    updateCollabLink
  } from '$lib/api';
  import { copyText } from '$lib/clipboard';
  import { toast } from '$lib/toast';
  import type { CollabLink, IconSetSummary, UpdateCollabLinkPayload } from '$lib/types';

  type ExpireMode = '1d' | '7d' | '30d' | 'never' | 'custom';
  type PasswordEditMode = 'keep' | 'clear' | 'set';
  type CollabLinkStatus = 'active' | 'expired' | 'revoked';

  const maxDateTimeInputValue = '9999/12/31 23:59';
  const maxDateTimeLocalValue = '9999-12-31T23:59';
  const dateTimeInputPattern = /^(\d{4})\/(\d{2})\/(\d{2})\s(\d{2}):(\d{2})$/;

  let sets: IconSetSummary[] = [];
  let links: CollabLink[] = [];
  let loading = true;
  let creating = false;
  let revokingAll = false;
  let selectedSetId = '';
  let listError = '';
  let formMode: ExpireMode = '7d';
  let customExpiresAt = '';
  let passwordEnabled = false;
  let password = '';
  let passwordVisible = false;
  let revokeAllOpen = false;
  let revokeTarget: CollabLink | null = null;
  let revokingLink = false;
  let deleteAllOpen = false;
  let deleteTarget: CollabLink | null = null;
  let deletingLink = false;
  let deletingAll = false;
  let detailTarget: CollabLink | null = null;
  let detailExpiresMode: ExpireMode = 'custom';
  let detailCustomExpiresAt = '';
  let detailPasswordMode: PasswordEditMode = 'keep';
  let detailPassword = '';
  let detailPasswordVisible = false;
  let detailSaving = false;
  let customExpiresPicker: HTMLInputElement | null = null;
  let detailCustomExpiresPicker: HTMLInputElement | null = null;

  $: selectedSet = sets.find((set) => set.id === selectedSetId) ?? null;
  $: initialSetId = page.url.searchParams.get('set_id')?.trim() ?? '';
  $: activeLinks = links.filter((link) => link.active);
  $: inactiveLinks = links.filter((link) => !link.active);
  $: bulkSubmitting = revokingAll || deletingAll;

  /// 校验 owner 登录态，未登录时跳转控制台登录页。
  async function guardSession() {
    const session = await getSession();
    if (!session.authenticated) {
      await goto('/console/login');
      return false;
    }

    return true;
  }

  /// 加载 owner 可管理的集合列表。
  async function refreshSets() {
    sets = await listAdminSets();
    if (!selectedSetId && sets.length > 0) {
      selectedSetId =
        (initialSetId && sets.some((set) => set.id === initialSetId)
          ? initialSetId
          : sets[0].id) ?? '';
    }
  }

  /// 加载当前选中集合的协作链接列表。
  async function refreshLinks() {
    if (!selectedSetId) {
      links = [];
      return;
    }

    links = await listCollabLinks(selectedSetId);
  }

  /// 生成一段适合协作链接使用的随机强 password 字符串。
  function buildStrongPassword() {
    const alphabet = 'ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789!@#$%^&*-_=+';
    return Array.from({ length: 18 }, () => alphabet[Math.floor(Math.random() * alphabet.length)]).join('');
  }

  /// 为创建表单生成随机强 password。
  function generateStrongPassword() {
    password = buildStrongPassword();
    passwordEnabled = true;
    toast.info('已生成强密码');
  }

  /// 复制当前输入框里的 password 明文。
  async function copyPasswordValue(value: string) {
    const currentPassword = value.trim();
    if (!currentPassword) {
      toast.error('请先输入 password');
      return;
    }

    try {
      await copyText(currentPassword);
      toast.info('Password 已复制');
    } catch {
      toast.error('复制 password 失败');
    }
  }

  /// 为详情编辑生成随机强 password。
  function generateDetailStrongPassword() {
    detailPassword = buildStrongPassword();
    detailPasswordMode = 'set';
    toast.info('已生成强密码');
  }

  /// 根据相对有效期模式计算 ISO 到期时间。
  function resolveRelativeExpiresAt(mode: Exclude<ExpireMode, 'never' | 'custom'>) {
    const days = mode === '1d' ? 1 : mode === '7d' ? 7 : 30;
    const expiresAt = new Date();
    expiresAt.setDate(expiresAt.getDate() + days);
    return expiresAt.toISOString();
  }

  /// 将自定义时间输入转换成后端需要的 ISO 时间。
  function parseCustomExpiresAt(value: string) {
    const parts = parseDateTimeInputParts(value);
    if (!parts) {
      throw new Error('请选择有效的自定义到期时间');
    }

    const date = new Date(`${parts.year}-${parts.month}-${parts.day}T${parts.hour}:${parts.minute}`);
    if (
      Number.isNaN(date.getTime()) ||
      date.getFullYear() !== Number(parts.year) ||
      date.getMonth() + 1 !== Number(parts.month) ||
      date.getDate() !== Number(parts.day) ||
      date.getHours() !== Number(parts.hour) ||
      date.getMinutes() !== Number(parts.minute)
    ) {
      throw new Error('请选择有效的自定义到期时间');
    }

    if (parts.normalized > maxDateTimeInputValue) {
      throw new Error('年份不能超过 9999');
    }

    if (date <= new Date()) {
      throw new Error('到期时间必须晚于当前时间');
    }

    return date.toISOString();
  }

  /// 计算创建协作链接时提交给后端的过期时间。
  function resolveExpiresAt() {
    if (formMode === 'never') return undefined;
    if (formMode === 'custom') {
      return parseCustomExpiresAt(customExpiresAt);
    }

    return resolveRelativeExpiresAt(formMode);
  }

  /// 计算详情编辑提交给后端的过期时间，null 表示永久有效。
  function resolveDetailExpiresAt() {
    if (detailExpiresMode === 'never') return null;
    if (detailExpiresMode === 'custom') {
      return parseCustomExpiresAt(detailCustomExpiresAt);
    }

    return resolveRelativeExpiresAt(detailExpiresMode);
  }

  /// 创建新的协作链接并刷新列表。
  async function submitCreateLink() {
    if (!selectedSetId) {
      toast.error('请先选择一个集合');
      return;
    }

    let expiresAt: string | undefined;

    try {
      expiresAt = resolveExpiresAt();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : '到期时间无效');
      return;
    }

    const submittedPassword = passwordEnabled ? password.trim() : '';
    if (passwordEnabled && !submittedPassword) {
      toast.error('请输入 password，或关闭 password 保护');
      return;
    }
    creating = true;

    try {
      const link = await createCollabLink({
        set_id: selectedSetId,
        expires_at: expiresAt,
        password: submittedPassword
      });
      // 启用 password 时保留明文输入，避免创建后立刻忘记。
      if (!submittedPassword) {
        password = '';
        passwordEnabled = false;
      }
      passwordVisible = false;
      await refreshLinks();
      try {
        await copyText(formatShareUrl(link.share_url));
        toast.info(submittedPassword ? '已创建并复制链接，password 已保留' : '已创建并复制');
      } catch {
        toast.error('协作链接已创建，但复制失败');
      }
    } catch (err) {
      toast.error(err instanceof Error ? err.message : '协作链接创建失败');
    } finally {
      creating = false;
    }
  }

  /// 复制某一条协作链接。
  async function copyLink(url: string) {
    try {
      const fullUrl = formatShareUrl(url);
      await copyText(fullUrl);
      toast.info('协作链接已复制');
    } catch {
      toast.error('复制协作链接失败');
    }
  }

  /// 把后端返回的分享路径补全为当前站点可复制的完整链接。
  function formatShareUrl(url: string) {
    if (typeof window === 'undefined') return url;
    return new URL(url, window.location.origin).toString();
  }

  /// 从协作分享链接里提取 token，列表里只展示这段稳定识别码。
  function shareTokenLabel(url: string) {
    const fullUrl = formatShareUrl(url);

    try {
      const token = new URL(fullUrl).searchParams.get('token')?.trim();
      return token || fullUrl;
    } catch {
      const token = url.match(/[?&]token=([^&]+)/)?.[1];
      return token ? decodeURIComponent(token) : url;
    }
  }

  /// 失效某一条协作链接。
  async function revokeLink(linkId: string) {
    revokingLink = true;

    try {
      await revokeCollabLink(linkId);
      toast.info('协作链接已失效');
      revokeTarget = null;
      await refreshLinks();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : '协作链接失效失败');
    } finally {
      revokingLink = false;
    }
  }

  /// 删除某一条协作链接记录。
  async function deleteLink(linkId: string) {
    deletingLink = true;

    try {
      await deleteCollabLink(linkId);
      toast.info('协作链接已删除');
      deleteTarget = null;
      await refreshLinks();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : '协作链接删除失败');
    } finally {
      deletingLink = false;
    }
  }

  /// 删除当前集合里所有已经失效或过期的协作链接记录。
  async function deleteAllInactiveLinks() {
    if (inactiveLinks.length === 0) return;
    deletingAll = true;

    try {
      await Promise.all(inactiveLinks.map((link) => deleteCollabLink(link.id)));
      toast.info('协作链接已全部删除');
      deleteAllOpen = false;
    } catch (err) {
      toast.error(err instanceof Error ? err.message : '批量删除失败');
    } finally {
      deletingAll = false;
      await refreshLinks();
    }
  }

  /// 失效当前集合的全部协作链接。
  async function revokeAllLinks() {
    if (!selectedSetId) return;
    revokingAll = true;

    try {
      await revokeAllCollabLinks(selectedSetId);
      toast.info('当前集合全部协作链接已失效');
      await refreshLinks();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : '批量失效失败');
    } finally {
      revokingAll = false;
    }
  }

  /// 打开协作链接详情编辑弹窗。
  function openLinkDetails(link: CollabLink) {
    detailTarget = link;
    detailExpiresMode = link.expires_at ? 'custom' : 'never';
    detailCustomExpiresAt = toDateTimeInputValue(link.expires_at);
    detailPasswordMode = 'keep';
    detailPassword = '';
    detailPasswordVisible = false;
  }

  /// 关闭协作链接详情编辑弹窗。
  function closeLinkDetails() {
    if (detailSaving) return;
    detailTarget = null;
  }

  /// 保存协作链接详情配置。
  async function saveLinkDetails() {
    if (!detailTarget) return;

    let expiresAt: string | null | undefined;
    try {
      expiresAt = resolveDetailExpiresAt();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : '到期时间无效');
      return;
    }

    const payload: UpdateCollabLinkPayload = { expires_at: expiresAt };
    const expiresChanged = detailExpiresMode !== 'custom'
      ? (detailExpiresMode === 'never' ? Boolean(detailTarget.expires_at) : true)
      : !sameDateTimeMinute(detailTarget.expires_at, expiresAt);
    if (expiresChanged) {
      payload.expires_at = expiresAt;
    } else {
      delete payload.expires_at;
    }
    if (detailPasswordMode === 'clear') {
      payload.clear_password = true;
    } else if (detailPasswordMode === 'set') {
      if (!detailPassword.trim()) {
        toast.error('请输入新的 password');
        return;
      }
      payload.password = detailPassword.trim();
    }

    if (
      payload.expires_at === undefined &&
      !payload.clear_password &&
      payload.password === undefined
    ) {
      toast.info('没有需要保存的更改');
      return;
    }

    detailSaving = true;

    try {
      await updateCollabLink(detailTarget.id, payload);
      toast.info('协作链接已更新');
      detailTarget = null;
      await refreshLinks();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : '协作链接更新失败');
    } finally {
      detailSaving = false;
    }
  }

  /// 点击详情弹窗遮罩时关闭弹窗。
  function handleDetailBackdropClick(event: MouseEvent) {
    if (event.target === event.currentTarget) {
      closeLinkDetails();
    }
  }

  /// 支持按 Escape 关闭详情弹窗。
  function handleDetailKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape' && detailTarget) {
      closeLinkDetails();
    }
  }

  /// 打开全部失效确认弹窗。
  function openRevokeAllModal() {
    if (!selectedSetId || activeLinks.length === 0) return;
    revokeAllOpen = true;
  }

  /// 根据当前链接状态打开批量失效或批量删除确认弹窗。
  function openBulkActionModal() {
    if (activeLinks.length > 0) {
      openRevokeAllModal();
      return;
    }
    if (inactiveLinks.length > 0) {
      deleteAllOpen = true;
    }
  }

  /// 打开单条协作链接失效确认弹窗。
  function openRevokeLinkModal(link: CollabLink) {
    revokeTarget = link;
  }

  /// 打开单条协作链接删除确认弹窗。
  function openDeleteLinkModal(link: CollabLink) {
    deleteTarget = link;
  }

  /// 关闭单条协作链接失效确认弹窗。
  function closeRevokeLinkModal() {
    if (revokingLink) return;
    revokeTarget = null;
  }

  /// 关闭单条协作链接删除确认弹窗。
  function closeDeleteLinkModal() {
    if (deletingLink) return;
    deleteTarget = null;
  }

  /// 确认失效单条协作链接。
  async function confirmRevokeLink() {
    if (!revokeTarget) return;
    await revokeLink(revokeTarget.id);
  }

  /// 确认删除单条协作链接。
  async function confirmDeleteLink() {
    if (!deleteTarget) return;
    await deleteLink(deleteTarget.id);
  }

  /// 关闭全部失效确认弹窗。
  function closeRevokeAllModal() {
    if (revokingAll) return;
    revokeAllOpen = false;
  }

  /// 关闭全部删除确认弹窗。
  function closeDeleteAllModal() {
    if (deletingAll) return;
    deleteAllOpen = false;
  }

  /// 确认失效当前集合的全部协作链接。
  async function confirmRevokeAllLinks() {
    revokeAllOpen = false;
    await revokeAllLinks();
  }

  /// 确认删除当前集合所有失效或过期链接。
  async function confirmDeleteAllLinks() {
    await deleteAllInactiveLinks();
  }

  /// 切换集合时刷新对应协作链接列表。
  async function handleSetChange() {
    listError = '';

    try {
      await refreshLinks();
    } catch (err) {
      listError = err instanceof Error ? err.message : '协作链接加载失败';
    }
  }

  function formatDateTime(value?: string) {
    if (!value) return '永久有效';
    return new Date(value).toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit'
    });
  }

  function formatDateTimeInput(value: string) {
    const digits = value.replace(/\D/g, '').slice(0, 12);
    const year = digits.slice(0, 4);
    const month = digits.slice(4, 6);
    const day = digits.slice(6, 8);
    const hour = digits.slice(8, 10);
    const minute = digits.slice(10, 12);

    if (digits.length <= 4) return year;
    if (digits.length <= 6) return `${year}/${month}`;
    if (digits.length <= 8) return `${year}/${month}/${day}`;
    if (digits.length <= 10) return `${year}/${month}/${day} ${hour}`;
    return `${year}/${month}/${day} ${hour}:${minute}`;
  }

  function normalizeDateTimeInput(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const formatted = formatDateTimeInput(input.value);
    input.value = formatted;
    return formatted;
  }

  function handleCustomExpiresAtInput(event: Event) {
    customExpiresAt = normalizeDateTimeInput(event);
  }

  function handleDetailCustomExpiresAtInput(event: Event) {
    detailCustomExpiresAt = normalizeDateTimeInput(event);
  }

  function minDateTimeLocal() {
    return toDateTimeLocal(new Date(Date.now() + 60_000).toISOString());
  }

  function openDateTimePicker(picker: HTMLInputElement | null, value: string) {
    if (!picker) return;
    picker.min = minDateTimeLocal();
    picker.value = toComparableDateTimeLocal(value);

    try {
      if (typeof picker.showPicker === 'function') {
        picker.showPicker();
        return;
      }
    } catch {
      // Fall back to click for browsers that expose showPicker but reject it.
    }

    picker.click();
  }

  function handleCustomExpiresAtPickerChange(event: Event) {
    customExpiresAt = toDateTimeInputValueFromLocal((event.currentTarget as HTMLInputElement).value);
  }

  function handleDetailCustomExpiresAtPickerChange(event: Event) {
    detailCustomExpiresAt = toDateTimeInputValueFromLocal((event.currentTarget as HTMLInputElement).value);
  }

  function linkStatus(link: CollabLink): CollabLinkStatus {
    if (link.revoked_at) return 'revoked';
    if (link.active) return 'active';
    return 'expired';
  }

  function linkStatusLabel(link: CollabLink) {
    const status = linkStatus(link);
    if (status === 'revoked') return '已失效';
    if (status === 'expired') return '已过期';
    return '有效';
  }

  function linkCanEdit(link: CollabLink) {
    return linkStatus(link) !== 'revoked';
  }

  function sameDateTimeMinute(left?: string | null, right?: string | null) {
    if (!left && !right) return true;
    if (!left || !right) return false;
    return toDateTimeLocal(left) === toComparableDateTimeLocal(right);
  }

  /// 将 ISO 时间转成 datetime-local 可编辑值。
  function toDateTimeLocal(value?: string) {
    if (!value) return '';
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return '';
    const local = new Date(date.getTime() - date.getTimezoneOffset() * 60_000);
    return local.toISOString().slice(0, 16);
  }

  function toComparableDateTimeLocal(value?: string) {
    const parts = parseDateTimeInputParts(value ?? '');
    if (parts) {
      return `${parts.year}-${parts.month}-${parts.day}T${parts.hour}:${parts.minute}`;
    }

    return toDateTimeLocal(value);
  }

  function toDateTimeInputValue(value?: string) {
    const localValue = toDateTimeLocal(value);
    if (!localValue) return '';
    return toDateTimeInputValueFromLocal(localValue);
  }

  function toDateTimeInputValueFromLocal(value: string) {
    if (!value) return '';
    const [date, time] = value.split('T');
    return `${date.replaceAll('-', '/')} ${time}`;
  }

  function parseDateTimeInputParts(value: string) {
    const normalized = value.trim();
    const match = normalized.match(dateTimeInputPattern);
    if (!match) return null;

    const [, year, month, day, hour, minute] = match;
    return { normalized, year, month, day, hour, minute };
  }

  onMount(async () => {
    loading = true;
    listError = '';

    try {
      if (!(await guardSession())) return;
      await refreshSets();
      await refreshLinks();
    } catch (err) {
      listError = err instanceof Error ? err.message : '协作管理页加载失败';
    } finally {
      loading = false;
    }
  });
</script>

<svelte:window on:keydown={handleDetailKeydown} />

<nav class="breadcrumb" aria-label="面包屑">
  <a href="/">图标集合</a>
  <span>/</span>
  <a href="/console">控制台</a>
  <span>/</span>
  <strong>协作管理</strong>
</nav>

<section class="collab-hero panel panel-pad">
  <div>
    <span class="eyebrow">Collab Access</span>
    <h1>协作管理</h1>
    <p>为单个集合创建带有效期的协作链接，让未登录用户进入共享编辑流程，共同维护图标。</p>
  </div>
</section>

{#if loading}
  <div class="notice">正在加载协作管理...</div>
{:else}
  <section class="collab-grid">
    <form class="panel panel-pad create-card" on:submit|preventDefault={submitCreateLink}>
      <div class="section-copy">
        <span class="eyebrow">Create Link</span>
        <h2>创建协作链接</h2>
      </div>

      <label class="field">
        <span>集合</span>
        <select class="input" bind:value={selectedSetId} on:change={handleSetChange}>
          {#each sets as set}
            <option value={set.id}>{set.name} / {set.id}</option>
          {/each}
        </select>
      </label>

      <div class="field-group padded-block">
        <span>有效期</span>
        <div class="expire-options">
          <label><input type="radio" bind:group={formMode} value="1d" /> 1 天</label>
          <label><input type="radio" bind:group={formMode} value="7d" /> 7 天</label>
          <label><input type="radio" bind:group={formMode} value="30d" /> 30 天</label>
          <label><input type="radio" bind:group={formMode} value="never" /> 永久</label>
          <label><input type="radio" bind:group={formMode} value="custom" /> 自定义</label>
        </div>
      </div>

      {#if formMode === 'custom'}
        <div class="field">
          <span>自定义到期时间</span>
          <div class="datetime-input-wrap">
            <input
              class="input datetime-input"
              bind:value={customExpiresAt}
              type="text"
              inputmode="numeric"
              maxlength="16"
              autocomplete="off"
              placeholder="yyyy/mm/dd hh:mm"
              on:input={handleCustomExpiresAtInput}
            />
            <button
              class="input-icon-button date-picker-button"
              type="button"
              aria-label="选择自定义到期时间"
              title="选择自定义到期时间"
              on:click={() => openDateTimePicker(customExpiresPicker, customExpiresAt)}
            >
              <CalendarClock size={16} strokeWidth={2.2} />
            </button>
            <input
              class="native-date-picker"
              bind:this={customExpiresPicker}
              type="datetime-local"
              min={minDateTimeLocal()}
              max={maxDateTimeLocalValue}
              aria-label="选择自定义到期时间"
              on:change={handleCustomExpiresAtPickerChange}
            />
          </div>
        </div>
      {/if}

      <div class="password-row padded-block">
        <label class="password-toggle">
          <input type="checkbox" bind:checked={passwordEnabled} />
          <span>启用 password 保护</span>
        </label>
      </div>

      {#if passwordEnabled}
        <label class="field">
          <span>Password</span>
          <div class="password-input-wrap">
            <input
              class="input password-input"
              bind:value={password}
              type={passwordVisible ? 'text' : 'password'}
              placeholder="输入协作者访问口令"
            />
            <div class="password-input-actions">
              <button
                class="input-icon-button"
                type="button"
                aria-label={passwordVisible ? '隐藏密码' : '查看密码'}
                title={passwordVisible ? '隐藏密码' : '查看密码'}
                on:click={() => (passwordVisible = !passwordVisible)}
              >
                {#if passwordVisible}
                  <Eye size={15} strokeWidth={2.2} />
                {:else}
                  <EyeOff size={15} strokeWidth={2.2} />
                {/if}
              </button>
              <button
                class="input-icon-button"
                type="button"
                aria-label="生成随机强密码"
                title="生成随机强密码"
                on:click={generateStrongPassword}
              >
                <WandSparkles size={15} strokeWidth={2.2} />
              </button>
            </div>
          </div>
        </label>
      {/if}

      <div class="form-actions padded-block top-divider">
        <span class="hint">协作者进入后只允许上传、批量上传和重命名 icon。</span>
        <button class="action compact-action" type="submit" disabled={creating || !selectedSetId}>
          <Link2 size={15} strokeWidth={2.2} />
          创建
        </button>
      </div>
    </form>

    <div class="panel panel-pad links-card">
      <div class="list-head">
        <div class="section-copy">
          <span class="eyebrow">Links</span>
          <h2>{selectedSet?.name || '协作链接'}</h2>
          <p>{selectedSet ? `/${selectedSet.id}` : '请选择一个集合'}</p>
        </div>
        <button
          class:danger={activeLinks.length === 0 && links.length > 0}
          class:secondary={activeLinks.length > 0}
          class="action compact-action"
          type="button"
          disabled={bulkSubmitting || links.length === 0}
          on:click={openBulkActionModal}
        >
          {#if activeLinks.length > 0}
            <ShieldCheck size={15} strokeWidth={2.2} />
            全部失效
          {:else}
            <Trash2 size={15} strokeWidth={2.2} />
            全部删除
          {/if}
        </button>
      </div>

      {#if listError}
        <div class="notice error">{listError}</div>
      {:else if !selectedSetId}
        <div class="notice">还没有可管理的集合。</div>
      {:else if links.length === 0}
        <div class="notice">当前集合还没有协作链接。</div>
      {:else}
        <div class="link-list">
          {#each links as link}
            <article class:inactive={linkStatus(link) !== 'active'} class="link-row">
              <button
                class="link-main"
                type="button"
                aria-label={linkCanEdit(link) ? `编辑协作链接 ${link.id}` : `查看已失效协作链接 ${link.id}`}
                title={linkCanEdit(link) ? '编辑协作链接详情' : '已失效链接不能编辑'}
                disabled={!linkCanEdit(link)}
                on:click={() => linkCanEdit(link) && openLinkDetails(link)}
              >
                <strong>{linkStatusLabel(link)}</strong>
                <span class="link-meta">
                  <span>{link.password_enabled ? '需要 password' : '仅 token'} · 到期：{formatDateTime(link.expires_at)}</span>
                  {#if link.revoked_at}
                    <span>失效：{formatDateTime(link.revoked_at)}</span>
                  {/if}
                  {#if linkCanEdit(link)}
                    <span class="detail-cue">
                      <Settings2 size={13} strokeWidth={2.2} />
                      点击编辑
                    </span>
                  {/if}
                </span>
                <code title={formatShareUrl(link.share_url)}>{shareTokenLabel(link.share_url)}</code>
              </button>

              <div class="row-actions">
                <button
                  class="icon-action"
                  type="button"
                  aria-label="复制协作链接"
                  title="复制协作链接"
                  disabled={linkStatus(link) !== 'active'}
                  on:click={() => copyLink(link.share_url)}
                >
                  <Copy size={15} strokeWidth={2.2} />
                </button>
                {#if linkStatus(link) === 'active'}
                  <button
                    class="icon-action danger-icon"
                    type="button"
                    aria-label="失效协作链接"
                    title="失效协作链接"
                    on:click={() => openRevokeLinkModal(link)}
                  >
                    <ShieldX size={15} strokeWidth={2.2} />
                  </button>
                {:else}
                  <button
                    class="icon-action danger-icon"
                    type="button"
                    aria-label="删除协作链接"
                    title="删除协作链接"
                    on:click={() => openDeleteLinkModal(link)}
                  >
                    <Trash2 size={15} strokeWidth={2.2} />
                  </button>
                {/if}
              </div>
            </article>
          {/each}
        </div>
      {/if}
    </div>
  </section>
{/if}

<DeleteConfirmModal
  open={revokeAllOpen}
  eyebrow="Revoke All"
  title="失效全部协作链接"
  description="这个操作会让当前集合所有仍有效的协作链接立即失效，并清理已经进入的协作者会话。"
  impact={selectedSet
    ? [
        `当前有效链接：${activeLinks.length} 条`,
        '协作者将无法继续进入共享编辑。'
      ]
    : []}
  actionLabel="全部失效"
  submitting={revokingAll}
  onCancel={closeRevokeAllModal}
  onConfirm={confirmRevokeAllLinks}
/>

<DeleteConfirmModal
  open={deleteAllOpen}
  eyebrow="Delete All"
  title="删除全部协作链接"
  description="这个操作会永久删除当前集合里所有已失效或已过期的协作链接记录。"
  impact={[
    `待删除链接：${inactiveLinks.length} 条`,
    '删除后这些记录不会再出现在协作管理列表。'
  ]}
  actionLabel="全部删除"
  submitting={deletingAll}
  onCancel={closeDeleteAllModal}
  onConfirm={confirmDeleteAllLinks}
/>

<DeleteConfirmModal
  open={revokeTarget !== null}
  eyebrow="Revoke Link"
  title="失效协作链接"
  description="这个操作会让当前协作链接立即失效，并清理已经进入的协作者会话。"
  impact={revokeTarget
    ? [
        `到期时间：${formatDateTime(revokeTarget.expires_at)}`,
        revokeTarget.password_enabled ? '该链接启用了 password 保护。' : '该链接仅通过 token 访问。'
      ]
    : []}
  actionLabel="确认失效"
  submitting={revokingLink}
  onCancel={closeRevokeLinkModal}
  onConfirm={confirmRevokeLink}
/>

<DeleteConfirmModal
  open={deleteTarget !== null}
  eyebrow="Delete Link"
  title="删除协作链接"
  description="这个操作会永久删除这条协作链接记录，并清理对应的协作者会话。"
  impact={deleteTarget
    ? [
        `当前状态：${linkStatusLabel(deleteTarget)}`,
        `到期时间：${formatDateTime(deleteTarget.expires_at)}`
      ]
    : []}
  submitting={deletingLink}
  onCancel={closeDeleteLinkModal}
  onConfirm={confirmDeleteLink}
/>

{#if detailTarget}
  <div class="detail-backdrop" role="presentation" on:click={handleDetailBackdropClick}>
    <div
      class="detail-modal"
      role="dialog"
      aria-modal="true"
      aria-labelledby="collab-detail-title"
    >
      <form class="detail-form" on:submit|preventDefault={saveLinkDetails}>
        <div class="detail-head">
          <span class="eyebrow">Share Detail</span>
          <button
            class="close-button"
            type="button"
            aria-label="关闭详情"
            disabled={detailSaving}
            on:click={closeLinkDetails}
          >
            <X size={18} strokeWidth={2.2} />
          </button>
        </div>

        <div class="detail-title-block">
          <h2 id="collab-detail-title">编辑协作链接</h2>
          <p>更新到期时间或 password 后，已有协作者会话会被清理，需要重新进入。</p>
        </div>

        <div class="detail-stats">
          <div>
            <span>集合</span>
            <strong>/{detailTarget.set_id}</strong>
          </div>
          <div>
            <span>创建时间</span>
            <strong>{formatDateTime(detailTarget.created_at)}</strong>
          </div>
          <div>
            <span>当前状态</span>
            <strong>{linkStatusLabel(detailTarget)}</strong>
          </div>
        </div>

        <div class="field detail-field">
          <span>token</span>
          <code class="readonly-link" title={formatShareUrl(detailTarget.share_url)}>
            {shareTokenLabel(detailTarget.share_url)}
          </code>
        </div>

        <div class="detail-edit-grid">
          <div class="field-group detail-block">
            <span class="detail-block-title">
              <CalendarClock size={15} strokeWidth={2.2} />
              到期时间
            </span>
            <div class="expire-options detail-options">
              <label><input type="radio" bind:group={detailExpiresMode} value="1d" /> 1 天</label>
              <label><input type="radio" bind:group={detailExpiresMode} value="7d" /> 7 天</label>
              <label><input type="radio" bind:group={detailExpiresMode} value="30d" /> 30 天</label>
              <label><input type="radio" bind:group={detailExpiresMode} value="never" /> 永久</label>
              <label><input type="radio" bind:group={detailExpiresMode} value="custom" /> 自定义</label>
            </div>
            {#if detailExpiresMode === 'custom'}
              <div class="datetime-input-wrap">
                <input
                  class="input datetime-input"
                  bind:value={detailCustomExpiresAt}
                  type="text"
                  inputmode="numeric"
                  maxlength="16"
                  autocomplete="off"
                  placeholder="yyyy/mm/dd hh:mm"
                  on:input={handleDetailCustomExpiresAtInput}
                />
                <button
                  class="input-icon-button date-picker-button"
                  type="button"
                  aria-label="选择自定义到期时间"
                  title="选择自定义到期时间"
                  on:click={() => openDateTimePicker(detailCustomExpiresPicker, detailCustomExpiresAt)}
                >
                  <CalendarClock size={16} strokeWidth={2.2} />
                </button>
                <input
                  class="native-date-picker"
                  bind:this={detailCustomExpiresPicker}
                  type="datetime-local"
                  min={minDateTimeLocal()}
                  max={maxDateTimeLocalValue}
                  aria-label="选择自定义到期时间"
                  on:change={handleDetailCustomExpiresAtPickerChange}
                />
              </div>
            {/if}
          </div>

          <div class="field-group detail-block">
            <span class="detail-block-title">
              <KeyRound size={15} strokeWidth={2.2} />
              Password
            </span>
            <div class="password-mode-options">
              <div class="password-mode-row">
                <label>
                  <input type="radio" bind:group={detailPasswordMode} value="keep" />
                  保持当前（{detailTarget.password_enabled ? '已启用' : '未启用'}）
                </label>
                {#if detailTarget.password_enabled}
                  <button
                    class="inline-copy-button"
                    type="button"
                    aria-label="复制当前 password"
                    title={detailTarget.password ? '复制当前 password' : '旧链接未保存 password 明文'}
                    disabled={!detailTarget.password}
                    on:click={() => copyPasswordValue(detailTarget?.password ?? '')}
                  >
                    <Copy size={14} strokeWidth={2.2} />
                    复制
                  </button>
                {/if}
              </div>
              {#if detailTarget.password_enabled && !detailTarget.password}
                <span class="password-copy-hint">旧链接没有可复制的 password 明文，重置后可复制。</span>
              {/if}
              {#if detailTarget.password_enabled}
                <label><input type="radio" bind:group={detailPasswordMode} value="clear" /> 移除 password</label>
              {/if}
              <label><input type="radio" bind:group={detailPasswordMode} value="set" /> 设置新 password</label>
            </div>

            {#if detailPasswordMode === 'set'}
              <div class="password-input-wrap">
                <input
                  class="input password-input"
                  bind:value={detailPassword}
                  type={detailPasswordVisible ? 'text' : 'password'}
                  placeholder="输入新的协作者访问口令"
                />
                <div class="password-input-actions">
                  <button
                    class="input-icon-button"
                    type="button"
                    aria-label={detailPasswordVisible ? '隐藏密码' : '查看密码'}
                    title={detailPasswordVisible ? '隐藏密码' : '查看密码'}
                    on:click={() => (detailPasswordVisible = !detailPasswordVisible)}
                  >
                    {#if detailPasswordVisible}
                      <Eye size={15} strokeWidth={2.2} />
                    {:else}
                      <EyeOff size={15} strokeWidth={2.2} />
                    {/if}
                  </button>
                  <button
                    class="input-icon-button"
                    type="button"
                    aria-label="复制 password"
                    title="复制 password"
                    on:click={() => copyPasswordValue(detailPassword)}
                  >
                    <Copy size={15} strokeWidth={2.2} />
                  </button>
                  <button
                    class="input-icon-button"
                    type="button"
                    aria-label="生成随机强密码"
                    title="生成随机强密码"
                    on:click={generateDetailStrongPassword}
                  >
                    <WandSparkles size={15} strokeWidth={2.2} />
                  </button>
                </div>
              </div>
            {/if}
          </div>
        </div>

        <div class="detail-actions top-divider">
          <button class="action secondary compact-action" type="button" disabled={detailSaving} on:click={closeLinkDetails}>
            <X size={15} strokeWidth={2.2} />
            取消
          </button>
          <button class="action compact-action" type="submit" disabled={detailSaving}>
            <Save size={15} strokeWidth={2.2} />
            {detailSaving ? '保存中...' : '保存'}
          </button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  .collab-hero {
    display: grid;
    gap: 22px;
  }

  .collab-grid {
    display: grid;
    grid-template-columns: minmax(320px, 0.82fr) minmax(0, 1.18fr);
    gap: 20px;
    align-items: start;
  }

  .create-card,
  .links-card {
    display: grid;
    gap: 18px;
  }

  .section-copy {
    display: grid;
    gap: 8px;
  }

  .field-group,
  .form-actions,
  .list-head,
  .row-actions,
  .detail-head,
  .detail-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .field-group {
    align-items: start;
    flex-direction: column;
  }

  .padded-block {
    padding-top: 4px;
  }

  .top-divider {
    padding-top: 16px;
    border-top: 1px solid rgba(246, 239, 217, 0.1);
  }

  .detail-backdrop {
    position: fixed;
    inset: 0;
    z-index: 1000;
    display: grid;
    place-items: center;
    padding: 20px;
    background: rgba(0, 0, 0, 0.64);
    backdrop-filter: blur(16px);
  }

  .detail-modal {
    display: grid;
    gap: 18px;
    width: min(760px, 100%);
    max-height: min(820px, calc(100vh - 40px));
    overflow: auto;
    padding: clamp(20px, 4vw, 32px);
    border: 1px solid rgba(198, 255, 72, 0.28);
    border-radius: 26px;
    color: #f6efd9;
    background:
      radial-gradient(circle at top right, rgba(198, 255, 72, 0.14), transparent 34%),
      linear-gradient(135deg, rgba(246, 239, 217, 0.08), rgba(12, 13, 11, 0.94) 42%),
      #10110e;
    box-shadow: 0 32px 90px rgba(0, 0, 0, 0.62);
  }

  .detail-form {
    display: grid;
    gap: 18px;
  }

  .detail-title-block {
    display: grid;
    gap: 10px;
  }

  .close-button {
    display: grid;
    width: 40px;
    height: 40px;
    place-items: center;
    border: 1px solid rgba(246, 239, 217, 0.2);
    border-radius: 10px;
    color: #f6efd9;
    background: rgba(246, 239, 217, 0.06);
  }

  .close-button:hover,
  .close-button:focus-visible {
    border-color: rgba(198, 255, 72, 0.45);
    background: rgba(198, 255, 72, 0.12);
    outline: none;
  }

  .close-button:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .detail-stats {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 10px;
  }

  .detail-stats div,
  .detail-block,
  .detail-field {
    padding: 14px;
    border: 1px solid rgba(246, 239, 217, 0.12);
    border-radius: 12px;
    background: rgba(246, 239, 217, 0.045);
  }

  .detail-stats span,
  .detail-field > span {
    display: block;
    color: rgba(246, 239, 217, 0.52);
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .detail-stats strong {
    display: block;
    margin-top: 8px;
    overflow-wrap: anywhere;
    color: #f6efd9;
    font-size: 14px;
  }

  .detail-edit-grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 12px;
  }

  .detail-block {
    align-items: stretch;
    gap: 14px;
  }

  .detail-block-title {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    color: #f6efd9;
    font-size: 13px;
    font-weight: 800;
  }

  .detail-options {
    gap: 10px 14px;
  }

  .password-mode-options {
    display: grid;
    gap: 10px;
    color: rgba(246, 239, 217, 0.7);
    font-size: 13px;
    line-height: 1.6;
  }

  .password-mode-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }

  .inline-copy-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    min-height: 30px;
    padding: 0 10px;
    border: 1px solid rgba(246, 239, 217, 0.16);
    border-radius: 8px;
    color: rgba(246, 239, 217, 0.72);
    background: rgba(246, 239, 217, 0.05);
    font-size: 12px;
    font-weight: 800;
    box-shadow: none;
  }

  .inline-copy-button:hover,
  .inline-copy-button:focus-visible {
    border-color: rgba(198, 255, 72, 0.42);
    color: #c6ff48;
    background: rgba(198, 255, 72, 0.1);
    outline: none;
  }

  .inline-copy-button:disabled {
    cursor: not-allowed;
    opacity: 0.45;
  }

  .password-copy-hint {
    color: rgba(246, 239, 217, 0.48);
    font-size: 12px;
    line-height: 1.6;
  }

  .readonly-link {
    display: block;
    margin-top: 10px;
    overflow-wrap: anywhere;
    color: rgba(246, 239, 217, 0.68);
    font-size: 12px;
    line-height: 1.8;
  }

  .detail-actions {
    justify-content: flex-end;
  }

  .expire-options {
    display: flex;
    flex-wrap: wrap;
    gap: 12px 18px;
    color: rgba(246, 239, 217, 0.7);
    font-size: 13px;
    line-height: 1.7;
  }

  .expire-options label,
  .password-toggle,
  .password-mode-options label {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }

  .password-toggle {
    color: rgba(246, 239, 217, 0.78);
    font-size: 13px;
  }

  .password-row {
    display: flex;
    align-items: center;
    justify-content: flex-start;
    gap: 12px;
  }

  .password-input-wrap {
    position: relative;
    display: flex;
    align-items: center;
    width: 100%;
  }

  .password-input {
    width: 100%;
    padding-right: 116px;
  }

  .datetime-input-wrap {
    position: relative;
    display: flex;
    align-items: center;
    width: 100%;
  }

  .datetime-input {
    width: 100%;
    padding-right: 54px;
  }

  .date-picker-button {
    position: absolute;
    top: 50%;
    right: 10px;
    transform: translateY(-50%);
  }

  .native-date-picker {
    position: absolute;
    right: 12px;
    bottom: 0;
    width: 1px;
    height: 1px;
    opacity: 0;
    pointer-events: none;
  }

  .password-input-actions {
    position: absolute;
    top: 50%;
    right: 10px;
    display: inline-flex;
    align-items: center;
    gap: 2px;
    transform: translateY(-50%);
  }

  .input-icon-button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: 8px;
    color: rgba(246, 239, 217, 0.58);
    background: transparent;
    box-shadow: none;
    transition:
      color 160ms ease,
      background 160ms ease;
  }

  .input-icon-button:hover,
  .input-icon-button:focus-visible {
    color: #f6efd9;
    background: rgba(246, 239, 217, 0.08);
    outline: none;
  }

  .input-icon-button:active {
    background: rgba(246, 239, 217, 0.12);
  }

  .compact-action {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    letter-spacing: 0.04em;
  }

  .compact-action :global(svg) {
    flex: 0 0 auto;
  }

  .hint,
  .list-head p {
    color: rgba(246, 239, 217, 0.58);
    font-size: 12px;
    line-height: 1.6;
  }

  .link-list {
    display: grid;
    gap: 14px;
  }

  .link-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 20px;
    padding: 18px;
    border: 1px solid rgba(246, 239, 217, 0.14);
    border-radius: 10px;
    background: rgba(246, 239, 217, 0.04);
    transition:
      border-color 160ms ease,
      background 160ms ease;
  }

  .link-row:hover {
    border-color: rgba(198, 255, 72, 0.28);
    background: rgba(246, 239, 217, 0.06);
  }

  .link-row.inactive {
    border-color: rgba(246, 239, 217, 0.1);
    background: rgba(246, 239, 217, 0.025);
  }

  .link-main {
    min-width: 0;
    padding: 0;
    border: 0;
    color: inherit;
    text-align: left;
    background: transparent;
    box-shadow: none;
    cursor: pointer;
  }

  .link-main:disabled {
    cursor: default;
    opacity: 0.72;
  }

  .link-main:focus-visible {
    outline: 2px solid rgba(198, 255, 72, 0.82);
    outline-offset: 8px;
  }

  .link-main strong,
  .link-main code {
    display: block;
  }

  .link-main strong {
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: 22px;
    letter-spacing: 0;
  }

  .link-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 8px 12px;
    margin-top: 6px;
    color: #c6ff48;
    font-size: 12px;
  }

  .detail-cue {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: rgba(246, 239, 217, 0.52);
  }

  .link-main code {
    margin-top: 12px;
    overflow-wrap: anywhere;
    color: rgba(246, 239, 217, 0.58);
    font-size: 12px;
    line-height: 1.8;
  }

  .row-actions {
    align-items: flex-start;
    justify-content: flex-end;
  }

  .icon-action {
    display: inline-grid;
    width: 40px;
    height: 40px;
    place-items: center;
    border: 1px solid rgba(246, 239, 217, 0.16);
    border-radius: 10px;
    color: #f6efd9;
    background: rgba(246, 239, 217, 0.05);
    box-shadow: none;
    transition:
      color 160ms ease,
      border-color 160ms ease,
      background 160ms ease;
  }

  .icon-action:hover,
  .icon-action:focus-visible {
    border-color: rgba(198, 255, 72, 0.44);
    color: #c6ff48;
    background: rgba(198, 255, 72, 0.1);
    outline: none;
  }

  .icon-action:disabled {
    cursor: not-allowed;
    opacity: 0.42;
  }

  .icon-action:disabled:hover {
    border-color: rgba(246, 239, 217, 0.16);
    color: #f6efd9;
    background: rgba(246, 239, 217, 0.05);
  }

  .danger-icon:hover,
  .danger-icon:focus-visible {
    border-color: rgba(255, 85, 36, 0.5);
    color: #ffb39b;
    background: rgba(255, 85, 36, 0.12);
  }

  h1,
  h2 {
    margin: 0;
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    letter-spacing: 0;
  }

  h1 {
    font-size: 70px;
    line-height: 0.94;
  }

  h2 {
    font-size: 38px;
  }

  p {
    margin: 14px 0 0;
    color: rgba(246, 239, 217, 0.68);
    line-height: 1.8;
  }

  @media (max-width: 980px) {
    .collab-grid,
    .link-row,
    .detail-edit-grid,
    .detail-stats {
      grid-template-columns: 1fr;
    }

    .form-actions,
    .list-head,
    .password-row,
    .detail-actions {
      align-items: stretch;
      flex-direction: column;
    }

    .row-actions {
      justify-content: flex-start;
    }

    h1 {
      font-size: 48px;
    }
  }
</style>
