import {
  Channel,
  convertFileSrc,
  invoke as tauriInvoke,
} from "@tauri-apps/api/core";
import {
  listen as tauriListen,
  type UnlistenFn as TauriUnlistenFn,
} from "@tauri-apps/api/event";
import { readCurrentAccessToken } from "./authStorage";
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

export type UnlistenFn = () => void;

interface AppEvent<T> {
  payload: T;
}

type EventHandler<T> = (event: AppEvent<T>) => void;

export async function runtimeInvoke<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  return tauriInvoke<T>(cmd, args);
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
    throw new Error("当前环境不支持 Tauri 事件监听");
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
