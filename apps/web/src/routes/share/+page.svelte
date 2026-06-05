<script lang="ts">
  import { page } from '$app/state';
  import SetViewer from '$lib/SetViewer.svelte';
  import { getSharedSet } from '$lib/api';
  import type { IconManifest } from '$lib/types';

  /// 从 query 参数读取分享链接里的 manifest 地址。
  function currentIconSetUrl() {
    return page.url.searchParams.get('icon_set_url')?.trim() ?? '';
  }

  /// 按分享链接加载外部 manifest。
  async function loadManifest(): Promise<IconManifest> {
    const iconSetUrl = currentIconSetUrl();

    if (!iconSetUrl) {
      throw new Error('缺少 icon_set_url 参数');
    }

    return getSharedSet(iconSetUrl);
  }

  /// 分享页直接复制传入的 manifest 地址，避免依赖当前登录仓库配置。
  function resolveManifestUrl() {
    return currentIconSetUrl();
  }
</script>

<SetViewer
  loadManifest={loadManifest}
  sourceKey={currentIconSetUrl()}
  breadcrumbHref="/"
  breadcrumbLabel="分享查看"
  resolveManifestUrl={resolveManifestUrl}
  shouldRefreshRepoConfig={false}
  shouldRefreshOnAuthChange={false}
  showShareButton={false}
/>
