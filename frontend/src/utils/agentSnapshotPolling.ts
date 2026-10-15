const AGENT_SNAPSHOT_REFRESH_INTERVAL_MS = 7_000;
const AGENT_STREAM_STALE_AFTER_MS = 10_000;

export function startAgentSnapshotRefreshPolling(
  threadId: string | null | undefined,
  isConnected: () => boolean,
  lastActivityAt: () => number,
  refresh: (threadId: string) => void,
): (() => void) | undefined {
  if (!threadId) return undefined;

  const timer = setInterval(() => {
    if (
      isConnected() &&
      Date.now() - lastActivityAt() < AGENT_STREAM_STALE_AFTER_MS
    ) return;
    refresh(threadId);
  }, AGENT_SNAPSHOT_REFRESH_INTERVAL_MS);
  return () => clearInterval(timer);
}
