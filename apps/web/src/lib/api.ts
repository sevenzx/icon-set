import type {
  CreateSetPayload,
  IconManifest,
  IconSetSummary,
  SessionResponse,
  UpdateSetPayload
} from './types';

const API_BASE_URL = import.meta.env.VITE_API_BASE_URL ?? '';

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

/// 使用管理员密码登录。
export function login(password: string) {
  return request<SessionResponse>('/api/auth/login', {
    method: 'POST',
    body: JSON.stringify({ password })
  });
}

/// 退出当前管理员会话。
export function logout() {
  return request<SessionResponse>('/api/auth/logout', { method: 'POST' });
}

/// 查询当前管理员会话状态。
export function getSession() {
  return request<SessionResponse>('/api/auth/session');
}

/// 创建新的图标集合。
export function createSet(payload: CreateSetPayload) {
  return request<IconSetSummary>('/api/admin/sets', {
    method: 'POST',
    body: JSON.stringify(payload)
  });
}

/// 更新图标集合基础信息。
export function updateSet(setId: string, payload: UpdateSetPayload) {
  return request<IconSetSummary>(`/api/admin/sets/${encodeURIComponent(setId)}`, {
    method: 'PATCH',
    body: JSON.stringify(payload)
  });
}

/// 删除图标集合及其登记的图片。
export function deleteSet(setId: string) {
  return request<IconSetSummary[]>(`/api/admin/sets/${encodeURIComponent(setId)}`, {
    method: 'DELETE'
  });
}

/// 上传图片到指定图标集合。
export function uploadIcon(setId: string, name: string, file: File) {
  const form = new FormData();
  form.append('name', name);
  form.append('file', file);

  return request<IconManifest>(`/api/admin/sets/${encodeURIComponent(setId)}/icons`, {
    method: 'POST',
    body: form
  });
}

/// 修改指定图标名称。
export function renameIcon(setId: string, iconId: string, name: string) {
  return request<IconManifest>(
    `/api/admin/sets/${encodeURIComponent(setId)}/icons/${encodeURIComponent(iconId)}`,
    {
      method: 'PATCH',
      body: JSON.stringify({ name })
    }
  );
}

/// 删除指定图标和对应 GitHub 文件。
export function removeIcon(setId: string, iconId: string) {
  return request<IconManifest>(
    `/api/admin/sets/${encodeURIComponent(setId)}/icons/${encodeURIComponent(iconId)}`,
    { method: 'DELETE' }
  );
}
