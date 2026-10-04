import {
  Channel,
  convertFileSrc,
  invoke as tauriInvoke,
} from "@tauri-apps/api/core";
import {
  listen as tauriListen,
  type UnlistenFn as TauriUnlistenFn,
} from "@tauri-apps/api/event";
import {
  readCurrentAccessToken,
  requireCurrentUserId,
} from "./authStorage";
export {
  apiBaseUrl,
  isTauriRuntime,
  shouldUseHttpBackend,
} from "../utils/runtimeMode";
import {
  apiBaseUrl,
  isTauriRuntime,
  shouldUseHttpBackend,
} from "../utils/runtimeMode";
import { getActiveLocale, translateMessage } from "../i18n";

export type UnlistenFn = () => void;

interface AppEvent<T> {
  payload: T;
}

type EventHandler<T> = (event: AppEvent<T>) => void;

const NATIVE_TAURI_COMMANDS = new Set([
  "invoke_agent_command",
  "register_user",
  "login_user",
  "refresh_token",
  "oauth2_authorize_url",
  "oauth2_available_providers",
  "oauth2_callback",
  "oauth2_browser_login",
  "cancel_oauth2_browser_login",
  "local_list_embedding_models",
  "local_get_embedding_model_download_info",
  "local_download_embedding_model",
  "local_cancel_embedding_model_download",
  "local_delete_embedding_model",
  "local_test_embedding_config",
  "local_embed_texts",
  "read_local_image_file",
  "subscribe_generation_task",
  "unsubscribe_generation_task",
  "get_generation_task_summary",
  "get_generation_task_snapshot",
  "list_active_generation_tasks",
  "list_generation_task_history",
  "get_log_dir",
  "read_recent_logs",
  "set_log_level",
]);
const NATIVE_USER_CONTEXT_COMMANDS = new Set([
  "invoke_agent_command",
  "get_log_dir",
  "read_recent_logs",
  "set_log_level",
]);

export async function runtimeInvoke<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (NATIVE_TAURI_COMMANDS.has(cmd)) {
    const nativeArgs = NATIVE_USER_CONTEXT_COMMANDS.has(cmd)
      ? { ...args, callerUserId: requireCurrentUserId() }
      : args;
    return tauriInvoke<T>(cmd, nativeArgs);
  }
  return tauriInvoke<T>("invoke_host_command", {
    command: cmd,
    args: args ?? {},
    userId: requireCurrentUserId(),
  });
}

export function createRuntimeChannel<T>(): Channel<T> {
  return new Channel<T>();
}

export function canListenAppEvents(): boolean {
  return isTauriRuntime() && !shouldUseHttpBackend();
}

export async function listenAppEvent<T>(
  channel: string,
  handler: EventHandler<T>,
): Promise<UnlistenFn> {
  if (!canListenAppEvents()) {
    throw new Error(translateMessage(getActiveLocale(), "errors.runtimeEventsUnsupported"));
  }
  const unlisten: TauriUnlistenFn = await tauriListen<T>(channel, handler);
  return unlisten;
}

export function toRuntimeMediaSrc(filePath?: string | null): string {
  const value = filePath?.trim();
  if (!value) return "";
  if (/^(?:https?:|data:|blob:|a4n-media:)/i.test(value)) return value;
  if (!shouldUseHttpBackend() && isTauriRuntime()) {
    return convertFileSrc(value, "a4n-media");
  }
  const token = readCurrentAccessToken();
  const query = new URLSearchParams({ path: value });
  if (token) query.set("access_token", token);
  return `${apiBaseUrl()}/media?${query.toString()}`;
}
