import { create } from "zustand";
import {
  AUTH_UNAUTHORIZED_EVENT,
  extractErrorMessage,
  isUnauthorizedError,
} from "../api";
import {
  CURRENT_SESSION_ID_KEY,
  clearCurrentAuth,
  readCurrentAccessToken,
  readCurrentRefreshToken,
  readCurrentSessionId,
  readCurrentUserId,
} from "../api/authStorage";
import {
  getCurrentUser,
  loginUser,
  logoutUser,
  registerUser,
  updateUserProfile,
  type UpdateProfileInput,
  type UserSummary,
} from "../api/auth";
import { completeOAuthLogin } from "../api/oauth";
import { getActiveLocale, translateMessage } from "../i18n";
import { logger } from "../utils/logger";

const authLog = logger.scoped("AuthStore");

interface AuthStore {
  initialized: boolean;
  user: UserSummary | null;
  error: string | null;
  init: () => Promise<void>;
  login: (username: string, password: string) => Promise<void>;
  oauthLogin: (callbackUrl: string) => Promise<{ user: UserSummary; returnTo: string }>;
  register: (username: string, email: string, password: string, inviteCode: string) => Promise<void>;
  updateProfile: (input: UpdateProfileInput) => Promise<UserSummary>;
  logout: () => Promise<void>;
  onUnauthorized: () => Promise<void>;
}

const readCurrentUserIfStored = async (): Promise<UserSummary | null> => {
  if (readCurrentUserId() == null) return null;
  if (!readCurrentAccessToken() && (!readCurrentRefreshToken() || !readCurrentSessionId())) {
    return null;
  }
  return getCurrentUser();
};

let authInitPromise: Promise<void> | null = null;
let authInitSessionId: string | null = null;

export const useAuthStore = create<AuthStore>((set, get) => ({
  initialized: false,
  user: null,
  error: null,

  init: async () => {
    const sessionId = readCurrentSessionId();
    if (authInitPromise && authInitSessionId === sessionId) return authInitPromise;

    authInitSessionId = sessionId;
    const initialization = (async () => {
      set({ error: null });
      try {
        const me = await readCurrentUserIfStored();
        if (readCurrentSessionId() !== sessionId) return;
        set({
          user: me,
          initialized: true,
          error: null,
        });
      } catch (error) {
        if (readCurrentSessionId() !== sessionId) return;
        if (isUnauthorizedError(error)) {
          await clearCurrentAuth();
          set({ user: null, initialized: true, error: null });
          return;
        }
        authLog.error("init failed", { error });
        set({
          error: extractErrorMessage(
            error,
            translateMessage(getActiveLocale(), "auth.initializationFailed"),
          ),
        });
        throw error;
      }
    })().finally(() => {
      if (authInitPromise === initialization) authInitPromise = null;
    });

    authInitPromise = initialization;
    return initialization;
  },

  login: async (username: string, password: string) => {
    const res = await loginUser(username, password);
    set({ user: res.user, initialized: true, error: null });
  },

  oauthLogin: async (callbackUrl: string) => {
    const { auth, returnTo } = await completeOAuthLogin(callbackUrl);
    if (readCurrentSessionId() !== auth.tokens.session_id) {
      throw new Error(translateMessage(getActiveLocale(), "auth.sessionChanged"));
    }
    set({ user: auth.user, initialized: true, error: null });
    return { user: auth.user, returnTo };
  },

  register: async (username: string, email: string, password: string, inviteCode: string) => {
    const res = await registerUser(username, email, password, inviteCode);
    set({ user: res.user, initialized: true, error: null });
  },

  updateProfile: async (input: UpdateProfileInput) => {
    const sessionId = readCurrentSessionId();
    const user = await updateUserProfile(input);
    if (readCurrentSessionId() !== sessionId || get().user?.id !== user.id) {
      throw new DOMException(
        translateMessage(getActiveLocale(), "auth.sessionChanged"),
        "AbortError",
      );
    }
    set({ user });
    return user;
  },

  logout: async () => {
    const sessionId = readCurrentSessionId();
    try {
      await logoutUser();
    } finally {
      if (readCurrentSessionId() === null || readCurrentSessionId() === sessionId) {
        set({ user: null });
      }
    }
  },

  onUnauthorized: async () => {
    // 401 时清空登录态，交由 AuthGate 展示登录页
    set({ user: null, initialized: true, error: null });
    try {
      await clearCurrentAuth();
    } catch (error) {
      authLog.error("clearing stored session failed", { error });
      set({ error: extractErrorMessage(error, translateMessage(getActiveLocale(), "auth.initializationFailed")) });
    }
  },
}));

// 监听全局 401 事件（由 apiClient 拦截器派发）
if (typeof window !== "undefined") {
  window.addEventListener(AUTH_UNAUTHORIZED_EVENT, () => {
    void useAuthStore.getState().onUnauthorized();
  });
  window.addEventListener("storage", (event) => {
    if (event.storageArea !== window.localStorage) return;
    if (event.key !== null && event.key !== CURRENT_SESSION_ID_KEY) return;
    useAuthStore.setState({ user: null, initialized: false, error: null });
    void useAuthStore.getState().init().catch(() => undefined);
  });
}
