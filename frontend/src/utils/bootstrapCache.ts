import { scheduleIdleTask } from "./scheduleIdleTask";
import { readPortableItem, writePortableItem } from "./portableStorage";

type CacheEnvelope<T> = {
  ts: number;
  data: T;
};

const canUseStorage = () => typeof window !== "undefined";
const BOOTSTRAP_WRITE_DEBOUNCE_MS = 300;

type PendingBootstrapWrite = {
  data: unknown;
  timerId: number | null;
  cancelIdle: (() => void) | null;
};

const pendingWrites = new Map<string, PendingBootstrapWrite>();
const lastCommittedDataPayloads = new Map<string, string>();

const cancelPendingWrite = (key: string) => {
  const pending = pendingWrites.get(key);
  if (!pending) return;
  if (pending.timerId !== null) {
    window.clearTimeout(pending.timerId);
  }
  pending.cancelIdle?.();
  pendingWrites.delete(key);
};

const commitBootstrapWrite = (key: string, data: unknown) => {
  const serializedData = JSON.stringify(data);
  if (lastCommittedDataPayloads.get(key) === serializedData) return;

  const payload = JSON.stringify({
    ts: Date.now(),
    data,
  } satisfies CacheEnvelope<unknown>);

  writePortableItem(key, payload);
  lastCommittedDataPayloads.set(key, serializedData);
};

export const readBootstrapCache = <T>(
  key: string,
  maxAgeMs = 5 * 60 * 1000,
): T | null => {
  if (!canUseStorage()) return null;
  const raw = readPortableItem(key);
  if (!raw) return null;
  const parsed = JSON.parse(raw) as CacheEnvelope<T>;
  if (!parsed || typeof parsed !== "object" || !Number.isFinite(parsed.ts)) {
    throw new Error("启动缓存格式无效");
  }
  if (Date.now() - parsed.ts > Math.max(0, maxAgeMs)) return null;
  lastCommittedDataPayloads.set(key, JSON.stringify(parsed.data ?? null));
  return parsed.data ?? null;
};

export const writeBootstrapCache = <T>(key: string, data: T) => {
  if (!canUseStorage()) return;

  cancelPendingWrite(key);

  const pending: PendingBootstrapWrite = {
    data,
    timerId: null,
    cancelIdle: null,
  };

  pending.timerId = window.setTimeout(() => {
    pending.timerId = null;
    pending.cancelIdle = scheduleIdleTask(
      () => {
        pending.cancelIdle = null;
        commitBootstrapWrite(key, pending.data);
        pendingWrites.delete(key);
      },
      { timeout: 1200 },
    );
  }, BOOTSTRAP_WRITE_DEBOUNCE_MS);

  pendingWrites.set(key, pending);
};
