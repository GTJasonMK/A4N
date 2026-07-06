export interface UsageLog {
  id: number;
  user_id: number;
  project_id: number | null;
  model: string;
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
  cost: number | null;
  created_at: string | null;
}

export interface AdminTaskInfo {
  id: string;
  task_type: string;
  priority: string;
  status: string;
  project_id: number | null;
  root_project_id: number | null;
  source_snapshot_id: number | null;
  task_scope: string | null;
  payload: unknown | null;
  progress: unknown | null;
  error_message: string | null;
  dag_id: string | null;
  created_at: string | null;
  started_at: string | null;
  completed_at: string | null;
}

export interface AdminUser {
  id: number;
  username: string;
  email: string;
  role: string;
  oauth_provider: string | null;
  oauth_subject: string | null;
  avatar_url: string | null;
  is_active: boolean;
  display_name: string | null;
  created_at: string | null;
  updated_at: string | null;
  last_login_at: string | null;
}

export interface CreateAdminUserInput {
  username: string;
  email: string;
  password: string;
  role: string;
}

export interface AdminInviteCode {
  id: number;
  code: string;
  created_by_user_id: number | null;
  used_by_user_id: number | null;
  created_at: string | null;
  used_at: string | null;
}

export interface CreateInviteCodeInput {
  code: string;
}

export interface AdminProject {
  id: number;
  user_id: number;
  title: string;
  description: string;
  project_type: string;
  status: string;
  target_chapters: number;
  current_chapter: number;
  total_chapters: number | null;
  chapters_per_part: number | null;
  avatar_animal: string | null;
  created_at: string | null;
  updated_at: string | null;
}

export interface DailyUsagePoint {
  date: string;
  total_calls: number;
  total_tokens: number;
  success_count: number;
  fail_count: number;
}
