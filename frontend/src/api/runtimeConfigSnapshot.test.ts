import { expect, it, vi } from "vitest";

const { unexpectedSnapshotRead } = vi.hoisted(() => ({
  unexpectedSnapshotRead: () => {
    throw new Error("不应读取本机运行时设置");
  },
}));

vi.mock("./localRuntimeSettings", () => ({
  activeLocalEmbeddingConfig: unexpectedSnapshotRead,
  activeLocalImageConfig: unexpectedSnapshotRead,
  activeLocalLlmConfig: unexpectedSnapshotRead,
  activeLocalVideoConfig: unexpectedSnapshotRead,
  getSystemDefaultImageApiKey: unexpectedSnapshotRead,
  getSystemDefaultLlmApiKey: unexpectedSnapshotRead,
  getSystemDefaultVideoApiKey: unexpectedSnapshotRead,
  listLocalImageConfigs: unexpectedSnapshotRead,
  listLocalLlmConfigs: unexpectedSnapshotRead,
  listLocalVideoConfigs: unexpectedSnapshotRead,
  usesSystemDefaultImageConfig: unexpectedSnapshotRead,
  usesSystemDefaultLlmConfig: unexpectedSnapshotRead,
  usesSystemDefaultVideoConfig: unexpectedSnapshotRead,
}));

vi.mock("../utils/runtimeMode", () => ({
  usesLocalRuntimeSettings: () => true,
  workspaceContext: unexpectedSnapshotRead,
}));

import { withRuntimeConfigSnapshot } from "./runtimeConfigSnapshot";

it("does not attach runtime config to source sync or rollback previews", () => {
  for (const command of ["preview_source_sync", "preview_rollback"]) {
    const args = { marker: command };
    expect(withRuntimeConfigSnapshot(command, args)).toBe(args);
  }
});

it("keeps runtime config for source sync decisions that can regenerate Galgame scenes", () => {
  expect(() =>
    withRuntimeConfigSnapshot("apply_source_sync_decisions", {}),
  ).toThrow("不应读取本机运行时设置");
});
