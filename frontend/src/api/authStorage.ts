import {
  readPortableItem,
  removePortableItem,
  writePortableItem,
} from "../utils/portableStorage";

const CURRENT_USER_ID_KEY = "a4n:current_user_id";
const CURRENT_SESSION_ID_KEY = "a4n:current_session_id";
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

export const writeCurrentUserId = (id: number | null): void => {
  if (typeof window === "undefined") return;
  if (id == null) {
    removePortableItem(CURRENT_USER_ID_KEY);
  } else {
    if (!Number.isSafeInteger(id) || id <= 0) {
      throw new Error("当前用户 ID 必须是正整数");
    }
    writePortableItem(CURRENT_USER_ID_KEY, String(id));
  }
};

export const readCurrentSessionId = (): string | null => {
  if (typeof window === "undefined") return null;
  const raw = readPortableItem(CURRENT_SESSION_ID_KEY);
  return raw && raw.trim() ? raw : null;
};

export const writeCurrentSessionId = (id: string | null): void => {
  if (typeof window === "undefined") return;
  if (!id) removePortableItem(CURRENT_SESSION_ID_KEY);
  else writePortableItem(CURRENT_SESSION_ID_KEY, id);
};

export const readCurrentAccessToken = (): string | null => {
  if (typeof window === "undefined") return null;
  const raw = readPortableItem(CURRENT_ACCESS_TOKEN_KEY);
  return raw && raw.trim() ? raw : null;
};

export const writeCurrentAccessToken = (token: string | null): void => {
  if (typeof window === "undefined") return;
  if (!token) removePortableItem(CURRENT_ACCESS_TOKEN_KEY);
  else writePortableItem(CURRENT_ACCESS_TOKEN_KEY, token);
};

export const readCurrentRefreshToken = (): string | null => {
  if (typeof window === "undefined") return null;
  const raw = readPortableItem(CURRENT_REFRESH_TOKEN_KEY);
  return raw && raw.trim() ? raw : null;
};

export const writeCurrentRefreshToken = (token: string | null): void => {
  if (typeof window === "undefined") return;
  if (!token) removePortableItem(CURRENT_REFRESH_TOKEN_KEY);
  else writePortableItem(CURRENT_REFRESH_TOKEN_KEY, token);
};

export const clearCurrentAuth = (): void => {
  writeCurrentUserId(null);
  writeCurrentSessionId(null);
  removePortableItem(CURRENT_ACCESS_TOKEN_KEY);
  removePortableItem(CURRENT_REFRESH_TOKEN_KEY);
};
