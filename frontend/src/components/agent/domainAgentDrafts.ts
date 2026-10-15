import type { SetStateAction } from "react";
import { readCurrentSessionId } from "../../api/authStorage";
import { useAuthStore } from "../../store/auth";
import { dirtyRegistry } from "../navigation/dirtyRegistry";
import {
  type PendingAgentMessage,
  type PendingAgentMessagesByOwner,
} from "./AgentMessageList";

export type DomainAgentOperationKind = "refresh" | "clear" | "send" | "control" | "approval";

export interface DomainAgentOperation {
  bindingKey: string;
  kind: DomainAgentOperationKind;
  value?: string;
  completed: Promise<void>;
  complete: () => void;
}

const draftsByOwner = new Map<string, Record<string, string>>();
const pendingMessagesByOwner = new Map<string, PendingAgentMessagesByOwner>();
const listenersByOwner = new Map<string, Set<() => void>>();
const versionsByOwner = new Map<string, number>();
const operationsByOwner = new Map<string, Map<symbol, DomainAgentOperation>>();
const ownerScopes = new Map<string, { userId: number | null; sessionId: string | null }>();

export function domainAgentComposerOwnerKey(key: string): string {
  const userId = useAuthStore.getState().user?.id ?? null;
  const sessionId = readCurrentSessionId();
  const owner = `${key}:${userId ?? "anonymous"}:${sessionId ?? "no-session"}`;
  const previous = ownerScopes.get(owner);
  if (!previous || previous.userId !== userId || previous.sessionId !== sessionId) ownerScopes.set(owner, { userId, sessionId });
  return owner;
}

function isCurrentOwner(owner: string): boolean {
  const scope = ownerScopes.get(owner);
  return scope != null &&
    scope.userId === (useAuthStore.getState().user?.id ?? null) &&
    scope.sessionId === readCurrentSessionId();
}

export function hasDomainAgentComposerDraft(ownerKey: string): boolean {
  return hasDomainAgentComposerDraftForOwner(domainAgentComposerOwnerKey(ownerKey));
}

export function domainAgentOperationInFlight(ownerKey: string): boolean {
  return domainAgentOperationInFlightForOwner(domainAgentComposerOwnerKey(ownerKey));
}

function syncDirty(owner: string): void {
  const key = `domain-agent:${owner}`;
  if (!isCurrentOwner(owner)) {
    dirtyRegistry.set(key, false);
    dirtyRegistry.setDiscardHandler(key, null);
    return;
  }
  const dirty = Object.values(draftsByOwner.get(owner) ?? {}).some((value) => value.trim().length > 0) ||
    (operationsByOwner.get(owner)?.size ?? 0) > 0;
  dirtyRegistry.setDiscardHandler(
    key,
    dirty ? () => {
      if ((operationsByOwner.get(owner)?.size ?? 0) > 0) return false;
      updateDomainAgentDrafts(owner, {});
      return true;
    } : null,
    dirty ? () => (operationsByOwner.get(owner)?.size ?? 0) === 0 : undefined,
  );
  dirtyRegistry.set(key, dirty);
}

function notifyOwner(owner: string): void {
  versionsByOwner.set(owner, (versionsByOwner.get(owner) ?? 0) + 1);
  syncDirty(owner);
  listenersByOwner.get(owner)?.forEach((listener) => listener());
}

function updateDomainAgentDrafts(owner: string, update: SetStateAction<Record<string, string>>): void {
  if (!isCurrentOwner(owner)) return;
  const current = draftsByOwner.get(owner) ?? {};
  const next = typeof update === "function" ? update(current) : update;
  if (next === current) return;
  if (Object.keys(next).length === 0) draftsByOwner.delete(owner);
  else draftsByOwner.set(owner, next);
  notifyOwner(owner);
}

export function clearDomainAgentComposerDraft(ownerKey: string): void {
  updateDomainAgentDrafts(domainAgentComposerOwnerKey(ownerKey), {});
}

export function clearDomainAgentComposerDraftForOwner(owner: string): void {
  updateDomainAgentDrafts(owner, {});
}

export function hasDomainAgentComposerDraftForOwner(owner: string): boolean {
  return isCurrentOwner(owner) &&
    Object.values(draftsByOwner.get(owner) ?? {}).some((value) => value.trim().length > 0);
}

export function isDomainAgentOwnerCurrent(owner: string): boolean {
  return isCurrentOwner(owner);
}

export function domainAgentOperationInFlightForOwner(owner: string): boolean {
  return isCurrentOwner(owner) && (operationsByOwner.get(owner)?.size ?? 0) > 0;
}

export function getDomainAgentOwnerVersion(owner: string): number {
  return versionsByOwner.get(owner) ?? 0;
}

export function getDomainAgentDrafts(owner: string): Record<string, string> {
  return isCurrentOwner(owner) ? draftsByOwner.get(owner) ?? {} : {};
}

export function getDomainAgentPendingMessages(
  owner: string,
  bindingKey: string,
): PendingAgentMessage[] {
  return isCurrentOwner(owner)
    ? pendingMessagesByOwner.get(owner)?.[bindingKey] ?? []
    : [];
}

export function updateDomainAgentPendingMessages(
  owner: string,
  update: (pending: PendingAgentMessagesByOwner) => PendingAgentMessagesByOwner,
): void {
  if (!isCurrentOwner(owner)) return;
  const current = pendingMessagesByOwner.get(owner) ?? {};
  const next = update(current);
  if (next === current) return;
  if (Object.keys(next).length === 0) pendingMessagesByOwner.delete(owner);
  else pendingMessagesByOwner.set(owner, next);
  notifyOwner(owner);
}

export function getDomainAgentOperations(owner: string): DomainAgentOperation[] {
  return isCurrentOwner(owner) ? [...(operationsByOwner.get(owner)?.values() ?? [])] : [];
}

export function subscribeDomainAgentDrafts(owner: string, listener: () => void): () => void {
  const listeners = listenersByOwner.get(owner) ?? new Set<() => void>();
  listeners.add(listener);
  listenersByOwner.set(owner, listeners);
  syncDirty(owner);
  return () => {
    listeners.delete(listener);
    if (listeners.size === 0) {
      listenersByOwner.delete(owner);
      versionsByOwner.delete(owner);
      syncDirty(owner);
    }
  };
}

export function updateDomainAgentDraftsForOwner(
  owner: string,
  update: SetStateAction<Record<string, string>>,
): void {
  updateDomainAgentDrafts(owner, update);
}

export function beginDomainAgentOperation(
  owner: string,
  bindingKey: string,
  kind: DomainAgentOperationKind,
  value?: string,
): symbol | null {
  if (!isCurrentOwner(owner)) return null;
  const operations = operationsByOwner.get(owner) ?? new Map<symbol, DomainAgentOperation>();
  if ([...operations.values()].some((operation) => operation.bindingKey === bindingKey)) return null;
  let complete!: () => void;
  const completed = new Promise<void>((resolve) => { complete = resolve; });
  const token = Symbol(kind);
  operations.set(token, { bindingKey, kind, value, completed, complete });
  operationsByOwner.set(owner, operations);
  notifyOwner(owner);
  return token;
}

export function finishDomainAgentOperation(owner: string, token: symbol): void {
  const operations = operationsByOwner.get(owner);
  const operation = operations?.get(token);
  if (!operation || !operations?.delete(token)) return;
  operation.complete();
  if (operations.size === 0) operationsByOwner.delete(owner);
  notifyOwner(owner);
}

export function hasDomainAgentOperation(
  owner: string,
  bindingKey: string,
  kind?: DomainAgentOperationKind,
): boolean {
  return getDomainAgentOperations(owner).some((operation) =>
    operation.bindingKey === bindingKey && (kind == null || operation.kind === kind),
  );
}

export function pendingDomainAgentClear(owner: string, bindingKey: string): Promise<void> | null {
  return getDomainAgentOperations(owner).find((operation) =>
    operation.bindingKey === bindingKey && operation.kind === "clear",
  )?.completed ?? null;
}

function syncOwner(owner: string): void {
  for (const listener of listenersByOwner.get(owner) ?? []) listener();
}

useAuthStore.subscribe((state) => {
  for (const [owner, scope] of ownerScopes) {
    if (scope.userId === (state.user?.id ?? null) && scope.sessionId === readCurrentSessionId()) continue;
    draftsByOwner.delete(owner);
    pendingMessagesByOwner.delete(owner);
    for (const operation of operationsByOwner.get(owner)?.values() ?? []) operation.complete();
    operationsByOwner.delete(owner);
    versionsByOwner.set(owner, (versionsByOwner.get(owner) ?? 0) + 1);
    dirtyRegistry.set(`domain-agent:${owner}`, false);
    dirtyRegistry.setDiscardHandler(`domain-agent:${owner}`, null);
    syncOwner(owner);
  }
});
