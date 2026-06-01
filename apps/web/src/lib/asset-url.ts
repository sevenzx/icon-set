import { browser } from '$app/environment';

/// 生成公开演示集合的 manifest API 地址。
export function manifestRawUrl(setId: string) {
  const path = `/api/sets/${encodeURIComponent(setId)}`;
  return browser ? `${window.location.origin}${path}` : path;
}
