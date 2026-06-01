import type {
  CreateSetPayload,
  IconManifest,
  IconSetSummary,
  RepoConfig,
  RepoConfigPayload,
  SessionResponse,
  UpdateSetPayload
} from './types';
import { authenticated } from './auth-state';

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? '';
const ADMIN_TOKEN_STORAGE_KEY = 'icon-set-admin-token';

let adminTokenMemory = '';

export class ApiError extends Error {
  status: number;

  /// 保存后端返回的 HTTP 状态码，方便页面做登录跳转。
  constructor(message: string, status: number) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
  }
}

/// 拼接 API 地址，支持同源代理和独立后端两种部署方式。
function apiUrl(path: string) {
  return `${API_BASE_URL}${path}`;
}

/// 发送 API 请求并统一解析错误响应。
async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers);
  const isFormData = init.body instanceof FormData;

  if (init.body && !isFormData && !headers.has('Content-Type')) {
    headers.set('Content-Type', 'application/json');
  }

  const response = await fetch(apiUrl(path), {
    ...init,
    headers,
    credentials: 'include'
  });

  if (!response.ok) {
    const message = await readErrorMessage(response);
    throw new ApiError(message, response.status);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return (await response.json()) as T;
}

/// 发送管理员写请求，除 session cookie 外额外携带当前会话的写 token。
async function adminRequest<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers);
  const adminToken = readAdminToken();

  if (adminToken) {
    headers.set('X-Admin-Token', adminToken);
  }

  try {
    return await request<T>(path, { ...init, headers });
  } catch (error) {
    if (error instanceof ApiError && error.status === 401) {
      forgetAdminToken();
    }
    throw error;
  }
}

/// 拼出同源或独立 API 的登录跳转地址。
export function authUrl(path: string) {
  return apiUrl(path);
}

/// 把当前会话写 token 限制在当前浏览器标签页内，关闭标签页后自动失效。
function rememberAdminToken(adminToken: string) {
  adminTokenMemory = adminToken;
  try {
    window.sessionStorage.setItem(ADMIN_TOKEN_STORAGE_KEY, adminToken);
  } catch {
    // sessionStorage 不可用时，仍保留当前页面生命周期内的内存 token。
  }
}

/// 清理当前标签页保存的会话写 token。
function forgetAdminToken() {
  adminTokenMemory = '';
  try {
    window.sessionStorage.removeItem(ADMIN_TOKEN_STORAGE_KEY);
  } catch {
    // 忽略浏览器存储限制导致的清理失败。
  }
}

/// 读取当前标签页保存的会话写 token。
function readAdminToken() {
  if (adminTokenMemory) {
    return adminTokenMemory;
  }

  try {
    adminTokenMemory = window.sessionStorage.getItem(ADMIN_TOKEN_STORAGE_KEY) ?? '';
  } catch {
    adminTokenMemory = '';
  }

  return adminTokenMemory;
}

/// 从错误响应中提取可展示的错误文案。
async function readErrorMessage(response: Response) {
  try {
    const payload = (await response.json()) as { error?: string };
    return payload.error ?? `请求失败：${response.status}`;
  } catch {
    return `请求失败：${response.status}`;
  }
}

/// 读取所有图标集合。
export function listSets() {
  return request<IconSetSummary[]>('/api/sets');
}

/// 读取指定图标集合的 manifest。
export function getSet(setId: string) {
  return request<IconManifest>(`/api/sets/${encodeURIComponent(setId)}`);
}

/// 退出当前管理员会话。
export async function logout() {
  try {
    return await request<SessionResponse>('/api/auth/logout', { method: 'POST' });
  } finally {
    forgetAdminToken();
    authenticated.set(false);
  }
}

/// 查询当前管理员会话状态。
export async function getSession() {
  const session = await request<SessionResponse>('/api/auth/session');

  if (session.authenticated && session.admin_token) {
    rememberAdminToken(session.admin_token);
  }

  const normalizedSession = {
    ...session,
    authenticated: session.authenticated && Boolean(readAdminToken())
  };
  authenticated.set(normalizedSession.authenticated);

  return normalizedSession;
}

/// 读取当前用户的 GitHub 仓库配置。
export function getRepoConfig() {
  return adminRequest<RepoConfig>('/api/admin/config');
}

/// 保存当前用户的 GitHub 仓库配置，token 只会发送给后端加密存储。
export function saveRepoConfig(payload: RepoConfigPayload) {
  return adminRequest<RepoConfig>('/api/admin/config', {
    method: 'PUT',
    body: JSON.stringify(payload)
  });
}

/// 读取当前用户的图标集合。
export function listAdminSets() {
  return adminRequest<IconSetSummary[]>('/api/admin/sets');
}

/// 读取当前用户指定图标集合的 manifest。
export function getAdminSet(setId: string) {
  return adminRequest<IconManifest>(`/api/admin/sets/${encodeURIComponent(setId)}`);
}

/// 创建新的图标集合。
export function createSet(payload: CreateSetPayload) {
  return adminRequest<IconSetSummary>('/api/admin/sets', {
    method: 'POST',
    body: JSON.stringify(payload)
  });
}

/// 更新图标集合基础信息。
export function updateSet(setId: string, payload: UpdateSetPayload) {
  return adminRequest<IconSetSummary>(`/api/admin/sets/${encodeURIComponent(setId)}`, {
    method: 'PATCH',
    body: JSON.stringify(payload)
  });
}

/// 删除图标集合及其登记的图片。
export function deleteSet(setId: string) {
  return adminRequest<IconSetSummary[]>(`/api/admin/sets/${encodeURIComponent(setId)}`, {
    method: 'DELETE'
  });
}

/// 上传图片到指定图标集合。
export function uploadIcon(setId: string, name: string, file: File) {
  const form = new FormData();
  form.append('name', name);
  form.append('file', file);

  return adminRequest<IconManifest>(`/api/admin/sets/${encodeURIComponent(setId)}/icons`, {
    method: 'POST',
    body: form
  });
}

/// 批量上传图片或 zip 压缩包到指定图标集合。
export function uploadIconsBatch(setId: string, files: File[], archive: File | null) {
  const form = new FormData();

  for (const file of files) {
    form.append('files', file);
  }
  if (archive) {
    form.append('archive', archive);
  }

  return adminRequest<IconManifest>(`/api/admin/sets/${encodeURIComponent(setId)}/icons/batch`, {
    method: 'POST',
    body: form
  });
}

/// 修改指定图标名称。
export function renameIcon(setId: string, iconId: string, name: string) {
  return adminRequest<IconManifest>(
    `/api/admin/sets/${encodeURIComponent(setId)}/icons/${encodeURIComponent(iconId)}`,
    {
      method: 'PATCH',
      body: JSON.stringify({ name })
    }
  );
}

/// 删除指定图标和对应 GitHub 文件。
export function removeIcon(setId: string, iconId: string) {
  return adminRequest<IconManifest>(
    `/api/admin/sets/${encodeURIComponent(setId)}/icons/${encodeURIComponent(iconId)}`,
    { method: 'DELETE' }
  );
}
