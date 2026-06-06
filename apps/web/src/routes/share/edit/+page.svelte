<script lang="ts">
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { onMount } from 'svelte';
  import {
    authorizeShareAccess,
    getShareAccessSession,
    inspectShareAccess
  } from '$lib/api';
  import { toast } from '$lib/toast';
  import type { ShareAccessInspect } from '$lib/types';

  let loading = true;
  let entering = false;
  let error = '';
  let password = '';
  let info: ShareAccessInspect | null = null;

  $: shareToken = page.url.searchParams.get('token')?.trim() ?? '';

  /// 如果当前浏览器已存在协作会话，则直接跳协作编辑页。
  async function tryRestoreSession() {
    if (shareToken) {
      return false;
    }

    const session = await getShareAccessSession();
    if (session.active) {
      await goto('/share/editor');
      return true;
    }

    return false;
  }

  /// 检查 token 对应的协作链接是否还能进入。
  async function refreshInfo() {
    if (!shareToken) {
      error = '缺少 token 参数';
      return;
    }

    info = await inspectShareAccess(shareToken);
    if (!info.active) {
      error = '当前协作链接已失效或已过期';
    }
  }

  /// 使用 token 和可选 password 进入协作编辑页。
  async function submitAuthorize() {
    if (!shareToken) return;
    entering = true;

    try {
      await authorizeShareAccess(shareToken, password);
      toast.info('已进入协作编辑');
      await goto('/share/editor');
    } catch (err) {
      error = err instanceof Error ? err.message : '协作链接验证失败';
      toast.error(error);
    } finally {
      entering = false;
    }
  }

  onMount(async () => {
    loading = true;
    error = '';

    try {
      if (await tryRestoreSession()) return;
      await refreshInfo();
    } catch (err) {
      error = err instanceof Error ? err.message : '协作链接读取失败';
    } finally {
      loading = false;
    }
  });
</script>

<nav class="breadcrumb" aria-label="面包屑">
  <a href="/">图标集合</a>
  <span>/</span>
  <strong>协作进入</strong>
</nav>

<section class="enter-shell">
  <div class="panel panel-pad enter-card">
    <span class="eyebrow">Shared Editing</span>
    <h1>进入协作编辑</h1>

    {#if loading}
      <div class="notice">正在检查协作链接...</div>
    {:else if error}
      <div class="notice error">{error}</div>
    {:else if info}
      <div class="intro">
        <strong>{info.set_name}</strong>
        <span>/{info.set_id}</span>
        <p>
          {#if info.password_enabled}
            这个协作链接启用了 password 保护，请输入口令后进入共享编辑。
          {:else}
            这个协作链接无需 password，点击下方按钮即可进入共享编辑。
          {/if}
        </p>
      </div>

      <form class="enter-form" on:submit|preventDefault={submitAuthorize}>
        {#if info.password_enabled}
          <label class="field">
            <span>Password</span>
            <input class="input" bind:value={password} type="password" placeholder="输入访问口令" />
          </label>
        {/if}

        <button class="action" type="submit" disabled={entering}>
          {entering ? '进入中...' : '进入协作编辑'}
        </button>
      </form>
    {/if}
  </div>
</section>

<style>
  .enter-shell {
    display: grid;
    min-height: min(760px, calc(100vh - 130px));
    place-items: center;
  }

  .enter-card,
  .enter-form,
  .intro {
    display: grid;
    gap: 16px;
  }

  .enter-card {
    width: min(620px, 100%);
    border-color: rgba(198, 255, 72, 0.24);
  }

  .intro strong {
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: 34px;
    letter-spacing: 0;
  }

  .intro span {
    color: #c6ff48;
    font-size: 12px;
  }

  h1 {
    margin: 0;
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: 64px;
    line-height: 0.94;
    letter-spacing: 0;
  }

  p {
    margin: 0;
    color: rgba(246, 239, 217, 0.68);
    line-height: 1.7;
  }

  @media (max-width: 640px) {
    h1 {
      font-size: 46px;
    }
  }
</style>
