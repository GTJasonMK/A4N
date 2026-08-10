import { apiInvoke } from "./index";
import {
  clearCurrentAuth,
  readCurrentRefreshToken,
  readCurrentSessionId,
  requireCurrentUserId,
  writeCurrentAccessToken,
  writeCurrentRefreshToken,
  writeCurrentSessionId,
  writeCurrentUserId,
} from "./authStorage";
import type {
  AuthResult,
  ChangePasswordInput,
  LoginInput,
  RefreshTokenInput,
  RegisterInput,
  SessionSummary,
  TokenPairSummary,
  UpdateProfileInput,
  UserSummary,
} from "./generated/auth";

export type {
  SessionSummary,
  UpdateProfileInput,
  UserSummary,
} from "./generated/auth";

export type TokenPair = TokenPairSummary;

const persistAuthResult = (result: AuthResult): void => {
  writeCurrentUserId(result.user.id);
  writeCurrentSessionId(result.tokens.session_id);
  writeCurrentAccessToken(result.tokens.access_token);
  writeCurrentRefreshToken(result.tokens.refresh_token);
};

export async function getCurrentUser(): Promise<UserSummary> {
  return apiInvoke<UserSummary>("get_current_user", { userId: requireCurrentUserId() });
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
      userId: requireCurrentUserId(),
      sessionId: requireCurrentSessionId(),
    });
  } finally {
    clearCurrentAuth();
  }
}

export async function changeUserPassword(
  oldPassword: string,
  newPassword: string,
): Promise<void> {
  const input: ChangePasswordInput = { old_password: oldPassword, new_password: newPassword };
  await apiInvoke<void>("change_user_password", {
    userId: requireCurrentUserId(),
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
export async function refreshToken(): Promise<TokenPair> {
  const token = readCurrentRefreshToken();
  if (!token) throw new Error("当前刷新令牌不存在，无法刷新令牌");
  const sessionId = requireCurrentSessionId();
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
  return apiInvoke<SessionSummary[]>("list_sessions", { userId: requireCurrentUserId() });
}

export async function oauth2AuthorizeUrl(provider: string): Promise<string> {
  return apiInvoke<string>("oauth2_authorize_url", { input: { provider } });
}

export async function oauth2Callback(input: {
  provider: string;
  code: string;
}): Promise<AuthResult> {
  const result = await apiInvoke<AuthResult>("oauth2_callback", { input });
  persistAuthResult(result);
  return result;
}

function requireCurrentSessionId(): string {
  const sessionId = readCurrentSessionId();
  if (!sessionId) throw new Error("当前会话不存在");
  return sessionId;
}
