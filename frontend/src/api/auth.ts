import { apiInvoke } from "./index";
import {
  clearCurrentAuth,
  readCurrentRefreshToken,
  readCurrentSessionId,
  readCurrentUserId,
  writeCurrentAccessToken,
  writeCurrentRefreshToken,
  writeCurrentSessionId,
  writeCurrentUserId,
} from "./authStorage";

export interface UserSummary {
  id: number;
  username: string;
  email: string;
  role: string;
  oauth_provider: string | null;
  oauth_subject: string | null;
  avatar_url: string | null;
  is_active: boolean;
  created_at: string | null;
  updated_at: string | null;
  last_login_at: string | null;
}

interface TokenPair {
  access_token: string;
  refresh_token: string;
  expires_in: number;
  token_type: string;
  session_id: string;
}

interface SessionSummary {
  id: string;
  user_id: number;
  user_agent: string | null;
  is_revoked: boolean;
  created_at: string | null;
  expires_at: string | null;
  revoked_at: string | null;
  last_used_at: string | null;
}

interface AuthResult {
  user: UserSummary;
  tokens: TokenPair;
  session: SessionSummary;
}

interface OAuth2LoginResult {
  user: UserSummary;
  tokens: TokenPair;
  session: SessionSummary;
}

const persistAuthResult = (result: AuthResult | OAuth2LoginResult): void => {
  writeCurrentUserId(result.user.id);
  writeCurrentSessionId(result.tokens.session_id || result.session.id);
  writeCurrentAccessToken(result.tokens.access_token);
  writeCurrentRefreshToken(result.tokens.refresh_token);
};

export async function getCurrentUser(): Promise<UserSummary> {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录");
  return apiInvoke<UserSummary>("get_current_user", { userId: id });
}

export async function loginUser(username: string, password: string): Promise<AuthResult> {
  const result = await apiInvoke<AuthResult>("login_user", {
    input: { username, password },
  });
  persistAuthResult(result);
  return result;
}

export async function registerUser(
  username: string,
  email: string,
  password: string,
  inviteCode: string,
): Promise<AuthResult> {
  const result = await apiInvoke<AuthResult>("register_user", {
    input: { username, email, password, invite_code: inviteCode },
  });
  persistAuthResult(result);
  return result;
}

export async function logoutUser(): Promise<void> {
  try {
    await apiInvoke<void>("logout_user", {
      userId: readCurrentUserId(),
      sessionId: readCurrentSessionId(),
    });
  } finally {
    clearCurrentAuth();
  }
}

export async function changeUserPassword(
  oldPassword: string,
  newPassword: string,
): Promise<void> {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录");
  await apiInvoke<void>("change_user_password", {
    userId: id,
    input: { old_password: oldPassword, new_password: newPassword },
  });
}

/** 后端 get_user(user_id) */
export async function getUser(userId: number): Promise<UserSummary> {
  return apiInvoke<UserSummary>("get_user", { userId });
}

/** 后端 update_user_profile(user_id, input: { email?, avatar_url? }) */
export async function updateUserProfile(
  userId: number,
  input: { email?: string | null; avatar_url?: string | null },
): Promise<UserSummary> {
  return apiInvoke<UserSummary>("update_user_profile", { userId, input });
}

/** 后端 refresh_token(input: { refresh_token, session_id }) */
export async function refreshToken(refreshToken: string): Promise<TokenPair> {
  const token = refreshToken || readCurrentRefreshToken();
  if (!token) throw new Error("当前刷新令牌不存在，无法刷新令牌");
  const sessionId = readCurrentSessionId();
  if (!sessionId) throw new Error("当前会话不存在，无法刷新令牌");
  const pair = await apiInvoke<TokenPair>("refresh_token", {
    input: { refresh_token: token, session_id: sessionId },
  });
  writeCurrentSessionId(pair.session_id);
  writeCurrentAccessToken(pair.access_token);
  writeCurrentRefreshToken(pair.refresh_token);
  return pair;
}

/** 后端 list_sessions(user_id) */
export async function listSessions(): Promise<SessionSummary[]> {
  const userId = readCurrentUserId();
  if (userId == null) return [];
  return apiInvoke<SessionSummary[]>("list_sessions", { userId });
}

/** 后端 revoke_session(user_id, session_id) */
export async function revokeSession(sessionId = readCurrentSessionId()): Promise<void> {
  const userId = readCurrentUserId();
  if (userId == null || !sessionId) throw new Error("当前会话不存在");
  await apiInvoke<void>("revoke_session", { userId, sessionId });
  if (sessionId === readCurrentSessionId()) {
    clearCurrentAuth();
  }
}

export async function oauth2AuthorizeUrl(provider: string): Promise<string> {
  return apiInvoke<string>("oauth2_authorize_url", { input: { provider } });
}

export async function oauth2Callback(input: {
  provider: string;
  code: string;
}): Promise<OAuth2LoginResult> {
  const result = await apiInvoke<OAuth2LoginResult>("oauth2_callback", { input });
  persistAuthResult(result);
  return result;
}
