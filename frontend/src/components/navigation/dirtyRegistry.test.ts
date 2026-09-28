import { describe, expect, it, vi } from "vitest";
import { dirtyRegistry } from "./dirtyRegistry";

describe("dirty registry discard", () => {
  it("checks every owner before discarding and stops when an owner rejects", async () => {
    const firstKey = "test:dirty-registry:first";
    const secondKey = "test:dirty-registry:second";
    const discarded: string[] = [];

    try {
      dirtyRegistry.setDiscardHandler(firstKey, () => { discarded.push(firstKey); });
      dirtyRegistry.setDiscardHandler(
        secondKey,
        () => { discarded.push(secondKey); return false; },
        () => false,
      );
      dirtyRegistry.set(firstKey, true);
      dirtyRegistry.set(secondKey, true);

      expect(dirtyRegistry.canDiscardDirty()).toBe(false);
      expect(await dirtyRegistry.discardDirty()).toBe(false);
      expect(discarded).toEqual([]);

      dirtyRegistry.setDiscardHandler(secondKey, () => { discarded.push(secondKey); });
      expect(await dirtyRegistry.discardDirty()).toBe(true);
      expect(discarded).toEqual([firstKey, secondKey]);
    } finally {
      dirtyRegistry.set(firstKey, false);
      dirtyRegistry.set(secondKey, false);
      dirtyRegistry.setDiscardHandler(firstKey, null);
      dirtyRegistry.setDiscardHandler(secondKey, null);
    }
  });

  it("notifies subscribers when an existing dirty owner's discard becomes unavailable", () => {
    const key = "test:dirty-registry:busy";
    let canDiscard = true;
    const listener = vi.fn();
    const unsubscribe = dirtyRegistry.subscribe(listener);
    dirtyRegistry.setDiscardHandler(key, () => {}, () => canDiscard);
    dirtyRegistry.set(key, true);
    listener.mockClear();

    try {
      canDiscard = false;
      dirtyRegistry.setDiscardHandler(key, () => {}, () => canDiscard);
      expect(dirtyRegistry.canDiscardDirty()).toBe(false);
      expect(listener).toHaveBeenCalledOnce();
    } finally {
      dirtyRegistry.set(key, false);
      dirtyRegistry.setDiscardHandler(key, null);
      unsubscribe();
    }
  });
});
