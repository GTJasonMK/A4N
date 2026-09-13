import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useToast } from "./Toast";

describe("toast actions", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    useToast.setState({ toasts: [] });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("keeps an optional action on the toast and expires it normally", () => {
    const onRetry = vi.fn();
    useToast.getState().addToast("加载失败", "error", { label: "重试", onClick: onRetry });

    const toast = useToast.getState().toasts[0];
    expect(toast.action?.label).toBe("重试");
    toast.action?.onClick();
    expect(onRetry).toHaveBeenCalledOnce();

    vi.advanceTimersByTime(3000);
    expect(useToast.getState().toasts).toHaveLength(0);
  });
});
