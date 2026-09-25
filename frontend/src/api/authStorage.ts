import {
  readPortableItem,
  removePortableItems,
  writePortableItems,
} from "../utils/portableStorage";
import type { AuthResult, TokenPairSummary } from "./generated/auth";

const CURRENT_USER_ID_KEY = "a4n:current_user_id";
export const CURRENT_SESSION_ID_KEY = "a4n:current_session_id";
const CURRENT_ACCESS_TOKEN_KEY = "a4n:current_access_token";
const CURRENT_REFRESH_TOKEN_KEY = "a4n:current_refresh_token";

export const readCurrentUserId = (): number | null => {
  if (typeof window === "undefined") return null;
  const raw = readPortableItem(CURRENT_USER_ID_KEY);
  if (!raw) return null;
  const n = Number(raw);
  return Number.isSafeInteger(n) && n > 0 ? n : null;
};

export const requireCurrentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录");
  return id;
};

export const readCurrentSessionId = (): string | null => {
  if (typeof window === "undefined") return null;
  const raw = readPortableItem(CURRENT_SESSION_ID_KEY);
  return raw && raw.trim() ? raw : null;
};

export const readCurrentAccessToken = (): string | null => {
  if (typeof window === "undefined") return null;
  const raw = readPortableItem(CURRENT_ACCESS_TOKEN_KEY);
  return raw && raw.trim() ? raw : null;
};

export const readCurrentRefreshToken = (): string | null => {
  if (typeof window === "undefined") return null;
  const raw = readPortableItem(CURRENT_REFRESH_TOKEN_KEY);
  return raw && raw.trim() ? raw : null;
};

const tokenEntries = (tokens: TokenPairSummary): Record<string, string> => ({
  [CURRENT_SESSION_ID_KEY]: tokens.session_id,
  [CURRENT_ACCESS_TOKEN_KEY]: tokens.access_token,
  [CURRENT_REFRESH_TOKEN_KEY]: tokens.refresh_token,
});

export const writeCurrentTokens = (tokens: TokenPairSummary): Promise<void> =>
  writePortableItems(tokenEntries(tokens));

export const writeCurrentAuth = (result: AuthResult): Promise<void> => {
  if (!Number.isSafeInteger(result.user.id) || result.user.id <= 0) {
    throw new Error("当前用户 ID 必须是正整数");
  }
  return writePortableItems({
    ...tokenEntries(result.tokens),
    [CURRENT_USER_ID_KEY]: String(result.user.id),
  });
};

export const clearCurrentAuth = (): Promise<void> => removePortableItems([
  CURRENT_USER_ID_KEY,
  CURRENT_SESSION_ID_KEY,
  CURRENT_ACCESS_TOKEN_KEY,
  CURRENT_REFRESH_TOKEN_KEY,
]);
