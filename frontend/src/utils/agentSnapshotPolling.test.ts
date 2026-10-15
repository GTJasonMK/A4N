import { afterEach, describe, expect, it, vi } from "vitest";
import { startAgentSnapshotRefreshPolling } from "./agentSnapshotPolling";

afterEach(() => {
  vi.useRealTimers();
});

describe("Agent snapshot refresh polling", () => {
  it("removes periodic snapshots during a live stream while retaining disconnected fallback", () => {
    vi.useFakeTimers();
    const disconnectedRefresh = vi.fn();
    const liveRefresh = vi.fn();
    let lastActivityAt = Date.now();
    const heartbeat = setInterval(() => { lastActivityAt = Date.now(); }, 5_000);

    const stopDisconnected = startAgentSnapshotRefreshPolling(
      "thread-1",
      () => false,
      () => lastActivityAt,
      disconnectedRefresh,
    );
    vi.advanceTimersByTime(28_000);
    stopDisconnected?.();

    const stopConnected = startAgentSnapshotRefreshPolling(
      "thread-1",
      () => true,
      () => lastActivityAt,
      liveRefresh,
    );
    vi.advanceTimersByTime(28_000);
    stopConnected?.();
    clearInterval(heartbeat);

    expect(disconnectedRefresh).toHaveBeenCalledTimes(4);
    expect(liveRefresh).not.toHaveBeenCalled();
  });

  it("restores snapshot polling when an open stream stops producing data", () => {
    vi.useFakeTimers();
    let lastActivityAt = Date.now();
    const refresh = vi.fn();
    const heartbeat = setInterval(() => { lastActivityAt = Date.now(); }, 5_000);
    const stop = startAgentSnapshotRefreshPolling(
      "thread-1",
      () => true,
      () => lastActivityAt,
      refresh,
    );

    vi.advanceTimersByTime(14_000);
    clearInterval(heartbeat);
    vi.advanceTimersByTime(7_000);
    stop?.();

    expect(refresh).toHaveBeenCalledOnce();
  });

  it("does not poll before a thread is selected", () => {
    vi.useFakeTimers();
    const refresh = vi.fn();

    expect(startAgentSnapshotRefreshPolling(null, () => false, () => 0, refresh)).toBeUndefined();
    vi.advanceTimersByTime(28_000);

    expect(refresh).not.toHaveBeenCalled();
  });
});
