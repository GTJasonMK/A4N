import { beforeEach, expect, it, vi } from "vitest";

const choiceDialog = vi.hoisted(() => vi.fn());

vi.mock("../../components/feedback/ConfirmDialog", () => ({ choiceDialog }));

import { chooseDownstreamConflictStrategy } from "./dependencyConflict";

beforeEach(() => choiceDialog.mockReset());

it("rejects newly appeared downstream dependencies when none were observed", async () => {
  await expect(
    chooseDownstreamConflictStrategy({
      title: "冲突",
      assetLabel: "资产",
      downstreamLabel: "下游",
      downstreamCount: 0,
      operation: "更新",
    }),
  ).resolves.toBe("reject");
  expect(choiceDialog).not.toHaveBeenCalled();
});
