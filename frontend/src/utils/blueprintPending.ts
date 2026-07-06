import {
  readPortableItem,
  removePortableItem,
  writePortableItem,
} from "./portableStorage";

type BlueprintPendingKind = "novel";

type PendingEnvelope = {
  ts: number;
};

const KEY_PREFIX = "a4n:workflow:blueprint-pending";
const DEFAULT_TTL_MS = 2 * 60 * 60 * 1000;

const normalizeProjectId = (projectId: number | string): string =>
  String(projectId || "").trim();

const canUseStorage = (): boolean => typeof window !== "undefined";

const getBlueprintPendingKey = (
  kind: BlueprintPendingKind,
  projectId: number | string,
): string => `${KEY_PREFIX}:${kind}:${normalizeProjectId(projectId)}:v1`;

export const markBlueprintGenerationPending = (
  kind: BlueprintPendingKind,
  projectId: number | string,
): void => {
  if (!canUseStorage()) return;
  const id = normalizeProjectId(projectId);
  if (!id) return;
  writePortableItem(
    getBlueprintPendingKey(kind, id),
    JSON.stringify({ ts: Date.now() }),
  );
};

export const clearBlueprintGenerationPending = (
  kind: BlueprintPendingKind,
  projectId: number | string,
): void => {
  if (!canUseStorage()) return;
  const id = normalizeProjectId(projectId);
  if (!id) return;
  removePortableItem(getBlueprintPendingKey(kind, id));
};

export const hasRecentBlueprintGenerationPending = (
  kind: BlueprintPendingKind,
  projectId: number | string,
  ttlMs: number = DEFAULT_TTL_MS,
): boolean => {
  if (!canUseStorage()) return false;
  const id = normalizeProjectId(projectId);
  if (!id) return false;

  const raw = readPortableItem(getBlueprintPendingKey(kind, id));
  if (!raw) return false;
  const parsed = JSON.parse(raw) as Partial<PendingEnvelope> | null;
  if (
    !parsed ||
    typeof parsed.ts !== "number" ||
    !Number.isFinite(parsed.ts) ||
    parsed.ts <= 0
  ) {
    throw new Error("蓝图生成 pending 状态格式无效");
  }
  const fresh = Date.now() - parsed.ts <= Math.max(0, ttlMs);
  if (!fresh) clearBlueprintGenerationPending(kind, id);
  return fresh;
};
