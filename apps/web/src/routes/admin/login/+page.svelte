<script lang="ts">
  import { goto } from '$app/navigation';
  import { login } from '$lib/api';

  let password = '';
  let loading = false;
  let error = '';

  /// 提交管理员密码并进入后台。
  async function submitLogin() {
    loading = true;
    error = '';

    try {
      await login(password);
      await goto('/admin');
    } catch (err) {
      error = err instanceof Error ? err.message : '登录失败';
    } finally {
      loading = false;
    }
  }
</script>

<nav class="breadcrumb" aria-label="面包屑">
  <a href="/">图标集合</a>
  <span>/</span>
  <strong>后台入口</strong>
</nav>

<section class="login-shell">
  <form class="login-card panel panel-pad" on:submit|preventDefault={submitLogin}>
    <span class="eyebrow">Admin Gate</span>
    <h1>后台入口</h1>

    <label class="field">
      <span>管理员密码</span>
      <input
        class="input"
        bind:value={password}
        type="password"
        autocomplete="current-password"
        placeholder="输入密码"
      />
    </label>

    {#if error}
      <div class="notice error">{error}</div>
    {/if}

    <button class="action" type="submit" disabled={loading || !password}>
      {loading ? '验证中...' : '登录后台'}
    </button>
  </form>
</section>

<style>
  .login-shell {
    display: grid;
    min-height: min(760px, calc(100vh - 130px));
    place-items: center;
  }

  .login-card {
    display: grid;
    gap: 18px;
    width: min(520px, 100%);
    border-color: rgba(198, 255, 72, 0.26);
  }

  h1 {
    margin: 0;
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: clamp(48px, 9vw, 88px);
    line-height: 0.9;
    letter-spacing: -0.07em;
  }

</style>
