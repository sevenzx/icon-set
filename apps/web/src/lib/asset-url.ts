import { browser } from '$app/environment';
import type { RepoConfig } from './types';

/// 生成集合 manifest 的可复制地址。
export function manifestRawUrl(
  setId: string,
  repoConfig?: Pick<
    RepoConfig,
    'configured' | 'owner' | 'repo' | 'branch'
  > | null
) {
  if (
    repoConfig?.configured &&
    repoConfig.owner &&
    repoConfig.repo &&
    repoConfig.branch
  ) {
    return `https://raw.githubusercontent.com/${repoConfig.owner}/${repoConfig.repo}/refs/heads/${repoConfig.branch}/sets/${setId}/manifest.json`;
  }

  const path = `/api/sets/${encodeURIComponent(setId)}`;
  return browser ? `${window.location.origin}${path}` : path;
}
