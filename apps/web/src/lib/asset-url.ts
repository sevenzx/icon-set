const RAW_ASSET_BASE_URL =
  import.meta.env.VITE_RAW_ASSET_BASE_URL ??
  'https://raw.githubusercontent.com/sevenzx/icon-set-assets/refs/heads/main';

/// 生成集合 manifest.json 的 raw GitHub 地址。
export function manifestRawUrl(setId: string) {
  return `${RAW_ASSET_BASE_URL.replace(/\/$/, '')}/sets/${encodeURIComponent(setId)}/manifest.json`;
}
