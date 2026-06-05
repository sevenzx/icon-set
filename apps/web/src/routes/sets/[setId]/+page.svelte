<script lang="ts">
  import { page } from '$app/state';
  import SetViewer from '$lib/SetViewer.svelte';
  import { getSet } from '$lib/api';

  /// 按当前路由里的 setId 读取站内集合。
  async function loadManifest() {
    const setId = page.params.setId;

    if (!setId) {
      throw new Error('缺少集合 ID');
    }

    return getSet(setId);
  }
</script>

<SetViewer loadManifest={loadManifest} sourceKey={page.params.setId ?? ''} />
