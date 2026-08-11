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
  type UserSummary,
} from "../api/auth";
import {
  readBootstrapCache,
  writeBootstrapCache,
} from "../utils/bootstrapCache";
import { logger } from "../utils/logger";

const authLog = logger.scoped("AuthStore");

interface AuthStore {
  initialized: boolean;
  loading: boolean;
  user: UserSummary | null;
  error: string | null;
  init: () => Promise<void>;
  reloadStatus: () => Promise<void>;
  refreshMe: () => Promise<void>;
  login: (username: string, password: string) => Promise<void>;
  oauthLogin: (provider: string, code: string) => Promise<UserSummary>;
  register: (username: string, email: string, password: string, inviteCode: string) => Promise<void>;
  logout: () => Promise<void>;
  onUnauthorized: (message?: string) => void;
}

type AuthBootstrapSnapshot = {
  user: UserSummary | null;
};

const AUTH_BOOTSTRAP_CACHE_KEY = "a4n:auth:bootstrap:v3";
const AUTH_BOOTSTRAP_CACHE_TTL_MS = 10 * 60 * 1000;

const saveAuthSnapshot = (user: UserSummary | null) => {
  writeBootstrapCache<AuthBootstrapSnapshot>(AUTH_BOOTSTRAP_CACHE_KEY, {
    user,
  });
};

const readCurrentUserIfStored = async (): Promise<UserSummary | null> => {
  if (readCurrentUserId() == null) return null;
  if (!readCurrentAccessToken() && (!readCurrentRefreshToken() || !readCurrentSessionId())) {
    return null;
  }
  return getCurrentUser();
};

const clearInvalidAuthSession = () => {
  clearCurrentAuth();
  saveAuthSnapshot(null);
  return {
    user: null,
    initialized: true,
    error: null,
  };
};

let authInitPromise: Promise<void> | null = null;

export const useAuthStore = create<AuthStore>((set, get) => ({
  initialized: false,
  loading: false,
  user: null,
  error: null,

  init: async () => {
    if (authInitPromise) return authInitPromise;

    authInitPromise = (async () => {
      set({ error: null });
      try {
        const cached = readBootstrapCache<AuthBootstrapSnapshot>(
          AUTH_BOOTSTRAP_CACHE_KEY,
          AUTH_BOOTSTRAP_CACHE_TTL_MS,
        );
        if (cached) {
          set({
            user: cached.user ?? null,
            initialized: true,
          });
        }

        const shouldBlock = !get().initialized;
        if (shouldBlock) {
          set({ loading: true });
        }

        const me = await readCurrentUserIfStored();
        set({
          user: me,
          initialized: true,
          error: null,
        });
        saveAuthSnapshot(me);
      } catch (error) {
        if (isUnauthorizedError(error)) {
          set(clearInvalidAuthSession());
          return;
        }
        authLog.error("init failed", { error });
        set({ error: extractErrorMessage(error, "认证初始化失败") });
        throw error;
      } finally {
        set({ loading: false });
      }
    })().finally(() => {
      authInitPromise = null;
    });

    return authInitPromise;
  },

  reloadStatus: async () => {
    set({ loading: !get().initialized, error: null });
    try {
      const me = await readCurrentUserIfStored();
      set({
        user: me,
        initialized: true,
        error: null,
      });
      saveAuthSnapshot(me);
    } catch (error) {
      if (isUnauthorizedError(error)) {
        set(clearInvalidAuthSession());
        return;
      }
      authLog.error("reload failed", { error });
      set({ error: extractErrorMessage(error, "重新加载认证状态失败") });
      throw error;
    } finally {
      set({ loading: false });
    }
  },

  refreshMe: async () => {
    set({ error: null });
    try {
      const me = await readCurrentUserIfStored();
      set({ user: me });
      saveAuthSnapshot(me);
    } catch (error) {
      if (isUnauthorizedError(error)) {
        set(clearInvalidAuthSession());
        return;
      }
      authLog.error("refreshMe failed", { error });
      set({ error: extractErrorMessage(error, "刷新当前用户失败") });
      throw error;
    }
  },

  login: async (username: string, password: string) => {
    set({ loading: true });
    try {
      const res = await loginUser(username, password);
      set({ user: res.user, initialized: true });
      saveAuthSnapshot(res.user);
    } finally {
      set({ loading: false });
    }
  },

  oauthLogin: async (provider: string, code: string) => {
    set({ loading: true });
    try {
      const res = await oauth2Callback({ provider, code });
      set({ user: res.user, initialized: true });
      saveAuthSnapshot(res.user);
      return res.user;
    } finally {
      set({ loading: false });
    }
  },

  register: async (username: string, email: string, password: string, inviteCode: string) => {
    set({ loading: true });
    try {
      const res = await registerUser(username, email, password, inviteCode);
      set({ user: res.user, initialized: true });
      saveAuthSnapshot(res.user);
    } finally {
      set({ loading: false });
    }
  },

  logout: async () => {
    set({ loading: true });
    try {
      await logoutUser();
    } finally {
      set({ user: null, loading: false, initialized: true });
      saveAuthSnapshot(null);
    }
  },

  onUnauthorized: () => {
    // 401 时清空登录态，交由 AuthGate 展示登录页
    clearCurrentAuth();
    set({ user: null, error: null });
    saveAuthSnapshot(null);
  },
}));

// 监听全局 401 事件（由 apiClient 拦截器派发）
if (typeof window !== "undefined") {
  window.addEventListener(AUTH_UNAUTHORIZED_EVENT, () => {
    useAuthStore.getState().onUnauthorized();
  });
}
