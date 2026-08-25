import { create } from "zustand";
import {
  AUTH_UNAUTHORIZED_EVENT,
  extractErrorMessage,
  isUnauthorizedError,
} from "../api";
import {
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
  oauth2Callback,
  registerUser,
  updateUserProfile,
  type UpdateProfileInput,
  type UserSummary,
} from "../api/auth";
import { getActiveLocale, translateMessage } from "../i18n";
import { logger } from "../utils/logger";

const authLog = logger.scoped("AuthStore");

interface AuthStore {
  initialized: boolean;
  user: UserSummary | null;
  error: string | null;
  init: () => Promise<void>;
  login: (username: string, password: string) => Promise<void>;
  oauthLogin: (provider: string, code: string) => Promise<UserSummary>;
  register: (username: string, email: string, password: string, inviteCode: string) => Promise<void>;
  updateProfile: (input: UpdateProfileInput) => Promise<UserSummary>;
  logout: () => Promise<void>;
  onUnauthorized: () => void;
}

const readCurrentUserIfStored = async (): Promise<UserSummary | null> => {
  if (readCurrentUserId() == null) return null;
  if (!readCurrentAccessToken() && (!readCurrentRefreshToken() || !readCurrentSessionId())) {
    return null;
  }
  return getCurrentUser();
};

let authInitPromise: Promise<void> | null = null;

export const useAuthStore = create<AuthStore>((set) => ({
  initialized: false,
  user: null,
  error: null,

  init: async () => {
    if (authInitPromise) return authInitPromise;

    authInitPromise = (async () => {
      set({ error: null });
      try {
        const me = await readCurrentUserIfStored();
        set({
          user: me,
          initialized: true,
          error: null,
        });
      } catch (error) {
        if (isUnauthorizedError(error)) {
          clearCurrentAuth();
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
      authInitPromise = null;
    });

    return authInitPromise;
  },

  login: async (username: string, password: string) => {
    const res = await loginUser(username, password);
    set({ user: res.user });
  },

  oauthLogin: async (provider: string, code: string) => {
    const res = await oauth2Callback({ provider, code });
    set({ user: res.user });
    return res.user;
  },

  register: async (username: string, email: string, password: string, inviteCode: string) => {
    const res = await registerUser(username, email, password, inviteCode);
    set({ user: res.user });
  },

  updateProfile: async (input: UpdateProfileInput) => {
    const user = await updateUserProfile(input);
    set({ user });
    return user;
  },

  logout: async () => {
    try {
      await logoutUser();
    } finally {
      set({ user: null });
    }
  },

  onUnauthorized: () => {
    // 401 时清空登录态，交由 AuthGate 展示登录页
    clearCurrentAuth();
    set({ user: null, error: null });
  },
}));

// 监听全局 401 事件（由 apiClient 拦截器派发）
if (typeof window !== "undefined") {
  window.addEventListener(AUTH_UNAUTHORIZED_EVENT, () => {
    useAuthStore.getState().onUnauthorized();
  });
}
