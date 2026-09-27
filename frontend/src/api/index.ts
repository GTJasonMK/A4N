import {
  readCurrentAccessToken,
  readCurrentRefreshToken,
  readCurrentSessionId,
  writeCurrentTokens,
} from "./authStorage";
import {
  apiBaseUrl,
  runtimeInvoke,
  shouldUseHttpBackend,
} from "./runtime";
import { logger } from "../utils/logger";
import {
  GENERATION_START_PATHS,
  type HostApiCommand,
} from "./generated/host";
import type { TokenPairSummary } from "./generated/auth";
import { withRuntimeConfigSnapshot } from "./runtimeConfigSnapshot";

export { extractErrorMessage } from "../utils/errorMessage";

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
export async function apiInvoke<T>(
  cmd: HostApiCommand,
  args?: Record<string, unknown>,
): Promise<T> {
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

let webAuthRefreshPromise: Promise<boolean> | null = null;

function generationRequestBody(
  cmd: string,
  args?: Record<string, unknown>,
): Record<string, unknown> {
  const envelope = args ?? {};
  const unknownField = Object.keys(envelope).find(
    (field) => field !== "input" && field !== "runtime_config_snapshot",
  );
  if (unknownField) throw new Error(`${cmd} 不接受参数: ${unknownField}`);
  const input = (envelope.input ?? {}) as Record<string, unknown>;
  return envelope.runtime_config_snapshot == null
    ? input
    : { ...input, runtime_config_snapshot: envelope.runtime_config_snapshot };
}

async function webInvoke<T>(
  cmd: HostApiCommand,
  args?: Record<string, unknown>,
): Promise<T> {
  const generationPath = GENERATION_START_PATHS.get(cmd);
  if (generationPath) {
    return webRequest<T>(generationPath, {
      method: "POST",
      body: generationRequestBody(cmd, args),
      auth: true,
    });
  }
  switch (cmd) {
    case "login_user":
    case "register_user":
    case "refresh_token":
    case "oauth2_authorize_url":
    case "oauth2_available_providers":
    case "oauth2_callback":
      return webRequest<T>(`/commands/${encodeURIComponent(cmd)}`, {
        method: "POST",
        body: args ?? {},
        auth: false,
      });
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
  const sessionId = readCurrentSessionId();
  if (
    options.auth
    && !readCurrentAccessToken()
    && readCurrentRefreshToken()
    && readCurrentSessionId()
  ) {
    const refreshed = await refreshWebAuthTokenOnce(null, sessionId);
    if (!refreshed) {
      throwWebUnauthorized("登录已过期，请重新登录");
    }
  }

  const accessToken = options.auth ? readCurrentAccessToken() : null;
  if (options.auth) requireUnchangedSession(sessionId);
  let response = await fetchWebRequest(path, options);
  if (response.status === 401) {
    requireUnchangedSession(sessionId);
    const refreshed = options.auth ? await refreshWebAuthTokenOnce(accessToken, sessionId) : false;
    if (refreshed) {
      requireUnchangedSession(sessionId);
      response = await fetchWebRequest(path, options);
    }
  }
  if (response.status === 401) {
    const message = await responseErrorMessage(response);
    requireUnchangedSession(sessionId);
    throwWebUnauthorized(message);
  }
  if (options.auth) requireUnchangedSession(sessionId);
  const result = await readWebResponse<T>(response);
  if (options.auth) requireUnchangedSession(sessionId);
  return result;
}

/** Authenticated fetch for streaming endpoints that cannot use the JSON command wrapper. */
export async function authenticatedWebFetch(
  path: string,
  init: RequestInit = {},
): Promise<Response> {
  const sessionId = readCurrentSessionId();
  if (!readCurrentAccessToken() && readCurrentRefreshToken() && readCurrentSessionId()) {
    const refreshed = await refreshWebAuthTokenOnce(null, sessionId);
    if (!refreshed) throwWebUnauthorized("登录已过期，请重新登录");
  }

  let accessToken: string | null = null;
  const request = async (): Promise<Response> => {
    requireUnchangedSession(sessionId);
    const token = readCurrentAccessToken();
    accessToken = token;
    if (!token) throwWebUnauthorized("未登录");
    const headers = new Headers(init.headers);
    headers.set("Authorization", `Bearer ${token}`);
    return fetch(`${apiBaseUrl()}${path}`, { ...init, headers });
  };

  let response = await request();
  requireUnchangedSession(sessionId);
  if (response.status === 401 && await refreshWebAuthTokenOnce(accessToken, sessionId)) {
    response = await request();
  }
  if (response.status === 401) {
    const message = await responseErrorMessage(response);
    requireUnchangedSession(sessionId);
    throwWebUnauthorized(message);
  }
  requireUnchangedSession(sessionId);
  return response;
}

function throwWebUnauthorized(message: string): never {
  const error = new ApiUnauthorizedError(message);
  if (typeof window !== "undefined") {
    window.dispatchEvent(new Event(AUTH_UNAUTHORIZED_EVENT));
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
  return (await response.json()) as T;
}

export async function refreshWebAuthentication(): Promise<void> {
  if (!await refreshWebAuthTokenOnce(readCurrentAccessToken(), readCurrentSessionId())) {
    throwWebUnauthorized("登录已过期，请重新登录");
  }
}

function requireUnchangedSession(sessionId: string | null): void {
  if (readCurrentSessionId() !== sessionId) {
    throw new DOMException("登录会话已变更", "AbortError");
  }
}

async function refreshWebAuthTokenOnce(accessToken: string | null, sessionId: string | null): Promise<boolean> {
  requireUnchangedSession(sessionId);
  if (!webAuthRefreshPromise) {
    if (!navigator.locks) {
      throw new Error("当前环境不支持跨标签登录续期，请使用 HTTPS 或 localhost");
    }
    webAuthRefreshPromise = Promise.resolve(navigator.locks.request("a4n:auth-refresh", async () => {
      requireUnchangedSession(sessionId);
      const currentToken = readCurrentAccessToken();
      if (currentToken && currentToken !== accessToken) return true;
      return refreshWebAuthToken();
    })).finally(() => {
      webAuthRefreshPromise = null;
    });
  }
  return await webAuthRefreshPromise;
}

async function refreshWebAuthToken(): Promise<boolean> {
  const refreshToken = readCurrentRefreshToken();
  const sessionId = readCurrentSessionId();
  if (!refreshToken || !sessionId) return false;

  const response = await fetch(`${apiBaseUrl()}/commands/refresh_token`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      input: { refresh_token: refreshToken, session_id: sessionId },
    }),
  });

  requireUnchangedSession(sessionId);

  if (response.status === 401 || response.status === 403) {
    await responseErrorMessage(response);
    requireUnchangedSession(sessionId);
    return false;
  }
  if (!response.ok) {
    throw new Error(await responseErrorMessage(response));
  }

  const pair = (await response.json()) as TokenPairSummary;
  requireUnchangedSession(sessionId);
  await writeCurrentTokens(pair);
  return true;
}

async function responseErrorMessage(response: Response): Promise<string> {
  let value: unknown;
  try {
    value = await response.json();
  } catch {
    throw invalidServerErrorContract(response.status);
  }
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw invalidServerErrorContract(response.status);
  }
  const body = value as Record<string, unknown>;
  if (
    Object.keys(body).length !== 5
    || typeof body.code !== "string"
    || !body.code.trim()
    || typeof body.message !== "string"
    || !body.message.trim()
    || typeof body.retryable !== "boolean"
    || (body.trace_id !== null
      && (typeof body.trace_id !== "string" || !body.trace_id.trim()))
    || !body.details
    || typeof body.details !== "object"
    || Array.isArray(body.details)
  ) {
    throw invalidServerErrorContract(response.status);
  }
  return body.message;
}

function invalidServerErrorContract(status: number): Error {
  return new Error(`Server 返回的 HTTP ${status} 错误体不符合 A4N API 合同`);
}
