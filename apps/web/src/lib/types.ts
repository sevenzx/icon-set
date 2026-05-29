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
