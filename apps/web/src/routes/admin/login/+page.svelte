<script lang="ts">
  import { authUrl } from '$lib/api';

  const githubLoginUrl = authUrl('/api/auth/github/start');
</script>

<nav class="breadcrumb" aria-label="面包屑">
  <a href="/">图标集合</a>
  <span>/</span>
  <strong>登录</strong>
</nav>

<section class="login-shell">
  <div class="login-card panel panel-pad">
    <span class="eyebrow">GitHub Identity</span>
    <h1>登录控制台</h1>
    <p>使用 GitHub 账号进入自己的图标仓库配置和管理空间。</p>

    <a class="action github-login" href={githubLoginUrl}>
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path
          d="M12 .5a12 12 0 0 0-3.79 23.39c.6.11.82-.26.82-.58v-2.23c-3.34.73-4.04-1.42-4.04-1.42-.55-1.39-1.34-1.76-1.34-1.76-1.09-.75.08-.74.08-.74 1.2.09 1.84 1.24 1.84 1.24 1.07 1.83 2.81 1.3 3.49.99.11-.78.42-1.3.76-1.6-2.66-.3-5.47-1.33-5.47-5.93 0-1.31.47-2.38 1.24-3.22-.12-.3-.54-1.52.12-3.18 0 0 1.01-.32 3.3 1.23a11.5 11.5 0 0 1 6.01 0c2.29-1.55 3.3-1.23 3.3-1.23.66 1.66.24 2.88.12 3.18.77.84 1.23 1.91 1.23 3.22 0 4.61-2.81 5.62-5.49 5.92.43.37.82 1.1.82 2.22v3.3c0 .32.22.7.83.58A12 12 0 0 0 12 .5Z"
          fill="currentColor"
        />
      </svg>
      GitHub 登录
    </a>
  </div>
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
    width: min(560px, 100%);
    border-color: rgba(198, 255, 72, 0.26);
  }

  h1 {
    margin: 0;
    font-family: 'Bricolage Grotesque', ui-sans-serif, system-ui, sans-serif;
    font-size: 68px;
    line-height: 0.94;
    letter-spacing: 0;
  }

  p {
    margin: 0;
    color: rgba(246, 239, 217, 0.68);
    line-height: 1.7;
  }

  .github-login svg {
    width: 20px;
    height: 20px;
  }

  @media (max-width: 620px) {
    h1 {
      font-size: 48px;
    }
  }
</style>
