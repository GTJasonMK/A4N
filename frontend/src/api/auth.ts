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
import type {
  AuthResult as GeneratedAuthResult,
  ChangePasswordInput,
  LoginInput,
  OAuth2LoginResult as GeneratedOAuth2LoginResult,
  RefreshTokenInput,
  RegisterInput,
  SessionSummary as GeneratedSessionSummary,
  TokenPairSummary,
  UpdateProfileInput,
  UserSummary as GeneratedUserSummary,
} from "./generated/auth";

export type {
  UpdateProfileInput,
} from "./generated/auth";

export type UserSummary = Omit<
  GeneratedUserSummary,
  | "oauth_provider"
  | "oauth_subject"
  | "avatar_url"
  | "created_at"
  | "updated_at"
  | "last_login_at"
> & {
  oauth_provider: string | null;
  oauth_subject: string | null;
  avatar_url: string | null;
  created_at: string | null;
  updated_at: string | null;
  last_login_at: string | null;
};
export type SessionSummary = Omit<
  GeneratedSessionSummary,
  | "user_agent"
  | "created_at"
  | "expires_at"
  | "revoked_at"
  | "last_used_at"
> & {
  user_agent: string | null;
  created_at: string | null;
  expires_at: string | null;
  revoked_at: string | null;
  last_used_at: string | null;
};
export type TokenPair = TokenPairSummary;
export type AuthResult = Omit<GeneratedAuthResult, "user" | "session"> & {
  user: UserSummary;
  session: SessionSummary;
};
export type OAuth2LoginResult = Omit<GeneratedOAuth2LoginResult, "user" | "session"> & {
  user: UserSummary;
  session: SessionSummary;
};

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
  const input: LoginInput = { username, password };
  const result = await apiInvoke<AuthResult>("login_user", {
    input,
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
  const input: RegisterInput = {
    username,
    email,
    password,
    invite_code: inviteCode,
  };
  const result = await apiInvoke<AuthResult>("register_user", {
    input,
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
  const input: ChangePasswordInput = { old_password: oldPassword, new_password: newPassword };
  await apiInvoke<void>("change_user_password", {
    userId: id,
    input,
  });
}

/** 后端 get_user(user_id) */
export async function getUser(userId: number): Promise<UserSummary> {
  return apiInvoke<UserSummary>("get_user", { userId });
}

/** 后端 update_user_profile(user_id, input: { email?, avatar_url? }) */
export async function updateUserProfile(
  userId: number,
  input: UpdateProfileInput,
): Promise<UserSummary> {
  return apiInvoke<UserSummary>("update_user_profile", { userId, input });
}

/** 后端 refresh_token(input: { refresh_token, session_id }) */
export async function refreshToken(refreshToken: string): Promise<TokenPair> {
  const token = refreshToken || readCurrentRefreshToken();
  if (!token) throw new Error("当前刷新令牌不存在，无法刷新令牌");
  const sessionId = readCurrentSessionId();
  if (!sessionId) throw new Error("当前会话不存在，无法刷新令牌");
  const input: RefreshTokenInput = { refresh_token: token, session_id: sessionId };
  const pair = await apiInvoke<TokenPair>("refresh_token", {
    input,
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
