export type IconSetSummary = {
  id: string;
  name: string;
  description: string;
  icon_count: number;
  updated_at: string;
};

export type IconEntry = {
  id: string;
  name: string;
  path: string;
  url: string;
  md5?: string;
};

export type IconManifest = {
  id: string;
  name: string;
  description: string;
  icons: IconEntry[];
  updated_at: string;
};

export type SessionResponse = {
  authenticated: boolean;
  admin_token?: string;
  user?: UserProfile;
  repo_config?: RepoConfig;
};

export type UserProfile = {
  id: string;
  login: string;
  name?: string;
  email?: string;
  avatar_url?: string;
};

export type RepoConfig = {
  configured: boolean;
  owner: string;
  repo: string;
  branch: string;
  token_configured: boolean;
};

export type RepoConfigPayload = {
  owner: string;
  repo: string;
  branch: string;
  token: string;
};

export type CreateSetPayload = {
  id: string;
  name: string;
  description: string;
};

export type UpdateSetPayload = {
  name?: string;
  description?: string;
};
