import { expect, it, vi } from "vitest";

import { deleteThemeIdsInParallel } from "./useThemeTabController";

it("runs theme deletes concurrently while preserving per-id failures", async () => {
  let active = 0;
  let maxActive = 0;
  const remove = vi.fn(async (id: number) => {
    active += 1;
    maxActive = Math.max(maxActive, active);
    await new Promise((resolve) => setTimeout(resolve, 5));
    active -= 1;
    if (id === 2) throw new Error("delete failed");
  });

  const outcomes = await deleteThemeIdsInParallel([1, 2, 3], remove);

  expect(maxActive).toBe(3);
  expect(outcomes).toEqual([
    { id: 1, ok: true },
    { id: 2, ok: false, error: expect.any(Error) },
    { id: 3, ok: true },
  ]);
  expect(remove).toHaveBeenCalledTimes(3);
});
