export interface PromptEntry {
  name: string;
  title: string;
  content: string;
  description: string;
  tags: string[];
  category: string;
  dependencies: string[];
  is_modified: boolean;
  version: string;
  modified_at: string;
  used_by?: string[];
  path?: string | null;
}

export interface PromptVersion {
  version: string;
  timestamp: string;
  author: string;
  message: string;
  content_snapshot: string;
}

export interface PromptDependencyGraph {
  nodes: unknown[];
  edges: unknown[];
}

export interface CategoryInfo {
  key: string;
  label?: string | null;
  order: number;
  description?: string | null;
}
