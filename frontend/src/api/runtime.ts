import {
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
interface ListenAppEventOptions {
  onError?: () => void;
  source?: "auto" | "tauri" | "server";
}

const SSE_OPEN_TIMEOUT_MS = 10_000;

export async function runtimeInvoke<T>(
  cmd: string,
  args?: Record<string, unknown>,
): Promise<T> {
  return tauriInvoke<T>(cmd, args);
}

export async function listenAppEvent<T>(
  channel: string,
  handler: EventHandler<T>,
  options?: ListenAppEventOptions,
): Promise<UnlistenFn> {
  const eventSource = options?.source ?? "auto";
  if (eventSource === "tauri" && !isTauriRuntime()) {
    throw new Error("当前环境不支持 Tauri 事件监听");
  }
  if (
    isTauriRuntime() &&
    (eventSource === "tauri" || (eventSource === "auto" && !shouldUseHttpBackend()))
  ) {
    const unlisten: TauriUnlistenFn = await tauriListen<T>(channel, handler);
    return unlisten;
  }

  const token = readCurrentAccessToken();
  if (!token) throw new Error("未登录");
  const sse = new EventSource(
    `${apiBaseUrl()}/events/${encodeURIComponent(channel)}?access_token=${encodeURIComponent(token)}`,
  );
  return await new Promise<UnlistenFn>((resolve, reject) => {
    let opened = false;
    const openTimer = window.setTimeout(() => {
      if (opened) return;
      sse.close();
      reject(new Error(`事件通道连接超时：${channel}`));
    }, SSE_OPEN_TIMEOUT_MS);

    const clearOpenTimer = () => window.clearTimeout(openTimer);

    sse.onopen = () => {
      opened = true;
      clearOpenTimer();
      resolve(() => sse.close());
    };

    sse.onmessage = (event: MessageEvent<string>) => {
      handler({ payload: JSON.parse(event.data) as T });
    };

    sse.onerror = () => {
      if (!opened) {
        clearOpenTimer();
        sse.close();
        reject(new Error(`事件通道连接失败：${channel}`));
        return;
      }
      options?.onError?.();
      sse.close();
    };
  });
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
