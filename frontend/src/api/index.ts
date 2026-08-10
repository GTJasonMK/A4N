import {
  readCurrentAccessToken,
  readCurrentRefreshToken,
  readCurrentSessionId,
  writeCurrentAccessToken,
  writeCurrentRefreshToken,
  writeCurrentSessionId,
} from "./authStorage";
import {
  apiBaseUrl,
  runtimeInvoke,
  shouldUseHttpBackend,
} from "./runtime";
import { logger } from "../utils/logger";
import { GENERATION_START_PATHS } from "./generated/host";
import { withRuntimeConfigSnapshot } from "./runtimeConfigSnapshot";

/**
 * Tauri 命令调用通用包装器。
 * 所有 API 模块函数均通过此包装器调用后端命令。
 *
 * 命令命名约定：{动词}_{领域}（如 create_project、list_chapters）
 *
 * 内置错误拦截：任何 invoke 失败都会自动 logger.error 到后端 tracing + 浏览器 console。
 * 业务层 catch 后即使只 toast 不 rethrow，错误也已经被记录。
 */

/** 后端返回的标准分页响应 */
export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  page_size: number;
}

const SENSITIVE_ARG_RE = /(?:api[_-]?key|password|token|secret|credential|authorization|base64)/i;

function redactApiArgs(value: unknown): unknown {
  if (Array.isArray(value)) {
    return value.map(redactApiArgs);
  }
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>).map(([key, child]) => [
        key,
        SENSITIVE_ARG_RE.test(key) ? "[REDACTED]" : redactApiArgs(child),
      ]),
    );
  }
  return value;
}

/** 通用 invoke 包装器 —— 失败时自动 log 后再 rethrow */
export async function apiInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const startedAt = Date.now();
  const effectiveArgs = withRuntimeConfigSnapshot(cmd, args);
  try {
    const result = shouldUseHttpBackend()
      ? await webInvoke<T>(cmd, effectiveArgs)
      : await runtimeInvoke<T>(cmd, effectiveArgs);
    return result;
  } catch (err) {
    // 自动记录：所有失败的后端调用统一进日志系统
    const durationMs = Date.now() - startedAt;
    logger.scoped("apiInvoke").error(`后端调用失败: ${cmd}`, {
      cmd,
      args: redactApiArgs(effectiveArgs),
      duration_ms: durationMs,
      error: err,
    });
    throw err;
  }
}

/** 提取后端错误消息 —— 支持多种响应格式 */
export function extractErrorMessage(err: unknown, defaultMessage = "Unknown error"): string {
  if (typeof err === "string") return err || defaultMessage;
  if (err instanceof Error) return err.message || defaultMessage;
  if (err && typeof err === "object") {
    const e = err as Record<string, unknown>;
    // Tauri 错误格式
    if (typeof e.detail === "string") return e.detail || defaultMessage;
    if (typeof e.message === "string") return e.message || defaultMessage;
    // FastAPI 验证错误数组格式
    if (Array.isArray(e.detail)) {
      const items = e.detail.slice(0, 3) as Array<{ msg?: string; loc?: string[] }>;
      const message = items.map((d) => {
        const loc = Array.isArray(d.loc) ? d.loc.join(".") : "";
        const msg = typeof d.msg === "string" && d.msg.trim() ? d.msg : "验证错误";
        return loc ? `${loc}: ${msg}` : msg;
      }).join("; ");
      return message || defaultMessage;
    }
  }
  const message = err == null ? "" : String(err);
  return message || defaultMessage;
}

/** 全局未授权事件名；store/auth 订阅后统一清理登录状态 */
export const AUTH_UNAUTHORIZED_EVENT = "a4n-auth-unauthorized";

export class ApiUnauthorizedError extends Error {
  constructor(message = "未登录") {
    super(message);
    this.name = "ApiUnauthorizedError";
  }
}

export function isUnauthorizedError(err: unknown): err is ApiUnauthorizedError {
  return err instanceof ApiUnauthorizedError;
}

type WebAuthTokenPair = {
  access_token: string;
  refresh_token: string;
  session_id: string;
};

let webAuthRefreshPromise: Promise<boolean> | null = null;

function generationRequestBody(
  args?: Record<string, unknown>,
): Record<string, unknown> {
  const input = (args?.input ?? {}) as Record<string, unknown>;
  return args?.runtime_config_snapshot == null
    ? input
    : { ...input, runtime_config_snapshot: args.runtime_config_snapshot };
}

async function webInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const generationPath = GENERATION_START_PATHS.get(cmd);
  if (generationPath) {
    return webRequest<T>(generationPath, {
      method: "POST",
      body: generationRequestBody(args),
      auth: true,
    });
  }
  switch (cmd) {
    case "login_user":
      return webRequest<T>("/auth/login", {
        method: "POST",
        body: (args?.input ?? {}) as Record<string, unknown>,
        auth: false,
      });
    case "register_user":
      return webRequest<T>("/auth/register", {
        method: "POST",
        body: (args?.input ?? {}) as Record<string, unknown>,
        auth: false,
      });
    case "get_current_user":
      return webRequest<T>("/auth/me", { method: "GET", auth: true });
    case "logout_user":
      return webRequest<T>("/auth/logout", {
        method: "POST",
        body: { session_id: args?.sessionId },
        auth: true,
      });
    case "refresh_token":
      return webRequest<T>("/auth/refresh", {
        method: "POST",
        body: (args?.input ?? {}) as Record<string, unknown>,
        auth: false,
      });
    case "oauth2_authorize_url":
    case "oauth2_callback":
      return webRequest<T>(`/commands/${encodeURIComponent(cmd)}`, {
        method: "POST",
        body: args ?? {},
        auth: false,
      });
    case "list_projects":
      return webRequest<T>(`/projects${queryString({
        page: args?.page,
        page_size: args?.pageSize,
        project_kind: args?.projectKind,
        root_project_id: args?.rootProjectId,
        access_scope: args?.accessScope,
      })}`, { method: "GET", auth: true });
    case "get_project":
      return webRequest<T>(`/projects/${encodeURIComponent(String(args?.projectId ?? ""))}`, {
        method: "GET",
        auth: true,
      });
    case "create_project":
      return webRequest<T>("/projects", {
        method: "POST",
        body: (args?.input ?? {}) as Record<string, unknown>,
        auth: true,
      });
    case "create_derived_project":
      return webRequest<T>("/projects/derived", {
        method: "POST",
        body: (args?.input ?? {}) as Record<string, unknown>,
        auth: true,
      });
    case "create_freeform_video_project":
      return webRequest<T>("/projects/freeform-video", {
        method: "POST",
        body: (args?.input ?? {}) as Record<string, unknown>,
        auth: true,
      });
    case "list_source_snapshot_chapters":
      return webRequest<T>(
        `/projects/${encodeURIComponent(String(args?.projectId ?? ""))}/source-snapshot-chapters`,
        {
          method: "GET",
          auth: true,
        },
      );
    case "list_all_users":
      return webRequest<T>("/admin/users", { method: "GET", auth: true });
    case "create_user":
      return webRequest<T>("/admin/users", {
        method: "POST",
        body: (args?.input ?? {}) as Record<string, unknown>,
        auth: true,
      });
    case "list_invite_codes":
      return webRequest<T>("/admin/invite-codes", { method: "GET", auth: true });
    case "create_invite_code":
      return webRequest<T>("/admin/invite-codes", {
        method: "POST",
        body: (args?.input ?? {}) as Record<string, unknown>,
        auth: true,
      });
    case "update_user_role":
      return webRequest<T>(`/admin/users/${encodeURIComponent(String(args?.userId ?? ""))}/role`, {
        method: "POST",
        body: (args?.input ?? {}) as Record<string, unknown>,
        auth: true,
      });
    case "disable_user":
      return webRequest<T>(
        `/admin/users/${encodeURIComponent(String(args?.userId ?? ""))}/disable`,
        {
          method: "POST",
          auth: true,
        },
      );
    default:
      return webCommandRequest<T>(cmd, args);
  }
}

async function webCommandRequest<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  return webRequest<T>(`/commands/${encodeURIComponent(cmd)}`, {
    method: "POST",
    body: args ?? {},
    auth: true,
  });
}

async function webRequest<T>(
  path: string,
  options: { method: "GET" | "POST"; body?: Record<string, unknown>; auth: boolean },
): Promise<T> {
  if (
    options.auth
    && !readCurrentAccessToken()
    && readCurrentRefreshToken()
    && readCurrentSessionId()
  ) {
    const refreshed = await refreshWebAuthTokenOnce();
    if (!refreshed) {
      throwWebUnauthorized("登录已过期，请重新登录");
    }
  }

  let response = await fetchWebRequest(path, options);
  if (response.status === 401) {
    const refreshed = options.auth ? await refreshWebAuthTokenOnce() : false;
    if (refreshed) {
      response = await fetchWebRequest(path, options);
      if (response.status !== 401) {
        return readWebResponse<T>(response);
      }
    }
    throwWebUnauthorized(await responseErrorMessage(response));
  }
  return readWebResponse<T>(response);
}

/** Authenticated fetch for streaming endpoints that cannot use the JSON command wrapper. */
export async function authenticatedWebFetch(
  path: string,
  init: RequestInit = {},
): Promise<Response> {
  if (!readCurrentAccessToken() && readCurrentRefreshToken() && readCurrentSessionId()) {
    const refreshed = await refreshWebAuthTokenOnce();
    if (!refreshed) throwWebUnauthorized("登录已过期，请重新登录");
  }

  const request = async (): Promise<Response> => {
    const token = readCurrentAccessToken();
    if (!token) throwWebUnauthorized("未登录");
    const headers = new Headers(init.headers);
    headers.set("Authorization", `Bearer ${token}`);
    return fetch(`${apiBaseUrl()}${path}`, { ...init, headers });
  };

  let response = await request();
  if (response.status === 401 && await refreshWebAuthTokenOnce()) {
    response = await request();
  }
  if (response.status === 401) {
    throwWebUnauthorized(await responseErrorMessage(response));
  }
  return response;
}

function throwWebUnauthorized(message: string): never {
  const error = new ApiUnauthorizedError(message);
  if (typeof window !== "undefined") {
    window.dispatchEvent(new CustomEvent(AUTH_UNAUTHORIZED_EVENT, { detail: error.message }));
  }
  throw error;
}

async function fetchWebRequest(
  path: string,
  options: { method: "GET" | "POST"; body?: Record<string, unknown>; auth: boolean },
): Promise<Response> {
  const headers: Record<string, string> = {};
  if (options.body !== undefined) headers["Content-Type"] = "application/json";
  if (options.auth) {
    const token = readCurrentAccessToken();
    if (!token) throw new Error("未登录");
    headers.Authorization = `Bearer ${token}`;
  }

  return await fetch(`${apiBaseUrl()}${path}`, {
    method: options.method,
    headers,
    body: options.body === undefined ? undefined : JSON.stringify(options.body),
  });
}

async function readWebResponse<T>(response: Response): Promise<T> {
  if (!response.ok) {
    throw new Error(await responseErrorMessage(response));
  }
  if (response.status === 204) return undefined as T;
  const text = await response.text();
  return (text ? JSON.parse(text) : undefined) as T;
}

async function refreshWebAuthTokenOnce(): Promise<boolean> {
  if (!webAuthRefreshPromise) {
    webAuthRefreshPromise = refreshWebAuthToken().finally(() => {
      webAuthRefreshPromise = null;
    });
  }
  return await webAuthRefreshPromise;
}

async function refreshWebAuthToken(): Promise<boolean> {
  const refreshToken = readCurrentRefreshToken();
  const sessionId = readCurrentSessionId();
  if (!refreshToken || !sessionId) return false;

  const response = await fetch(`${apiBaseUrl()}/auth/refresh`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ refresh_token: refreshToken, session_id: sessionId }),
  });

  if (response.status === 401 || response.status === 403) return false;
  if (!response.ok) {
    throw new Error(await responseErrorMessage(response));
  }

  const pair = (await response.json()) as WebAuthTokenPair;
  writeCurrentSessionId(pair.session_id);
  writeCurrentAccessToken(pair.access_token);
  writeCurrentRefreshToken(pair.refresh_token);
  return true;
}

function queryString(params: Record<string, unknown>): string {
  const search = new URLSearchParams();
  for (const [key, value] of Object.entries(params)) {
    if (value == null || value === "") continue;
    search.set(key, String(value));
  }
  const text = search.toString();
  return text ? `?${text}` : "";
}

async function responseErrorMessage(response: Response): Promise<string> {
  const text = await response.text();
  if (!text) return `${response.status} ${response.statusText}`;
  try {
    const body = JSON.parse(text) as { message?: unknown };
    if (typeof body.message === "string" && body.message.trim()) return body.message;
  } catch {
    return text;
  }
  return text;
}
