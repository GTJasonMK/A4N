import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  runGenerationProgressTaskThen: vi.fn(),
  runProgressTask: vi.fn(),
  setProgress: vi.fn(),
  startOutlineGeneration: vi.fn(),
  syncOutlineMaterials: vi.fn(),
}));

vi.mock("react", () => ({
  useCallback: <T,>(callback: T) => callback,
  useRef: <T,>(value: T) => ({ current: value }),
  useState: <T,>(value: T) => [value, vi.fn()] as const,
}));
vi.mock("../../../api/outline", () => ({
  startOutlineGeneration: mocks.startOutlineGeneration,
  syncOutlineMaterials: mocks.syncOutlineMaterials,
}));
vi.mock("../../../hooks/runGenerationProgressTask", () => ({
  runGenerationProgressTaskThen: mocks.runGenerationProgressTaskThen,
}));
vi.mock("../../../hooks/useProgressTask", () => ({
  useProgressTask: () => ({ runProgressTask: mocks.runProgressTask }),
}));
vi.mock("../../../hooks/useGenerationTask", () => ({
  useGenerationTask: () => ({}),
}));
vi.mock("../../../i18n", () => ({
  useI18n: () => ({
    t: (key: string) => key,
  }),
}));

import { useOutlineGeneration } from "./useOutlineGeneration";

describe("useOutlineGeneration", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.startOutlineGeneration.mockResolvedValue({ task_id: "outline-task" });
    mocks.runGenerationProgressTaskThen.mockImplementation(
      async (_runProgressTask, _handle, _options, afterTerminal) =>
        afterTerminal({}, { setProgress: mocks.setProgress }),
    );
  });

  it("syncs current formal outline materials after terminal and returns warnings", async () => {
    const order: string[] = [];
    mocks.syncOutlineMaterials.mockImplementation(async () => {
      order.push("sync");
      return ["索引待恢复"];
    });
    const afterCompletion = vi.fn(async () => {
      order.push("refresh");
    });

    const result = await useOutlineGeneration().run(
      {
        title: "生成大纲",
        startMessage: "生成中",
        successMessage: "已完成",
        errorMessage: "失败",
      },
      {
        project_id: 7,
        operation: {
          kind: "regenerate_last_part",
          variables: null,
        },
      },
      afterCompletion,
    );

    expect(mocks.syncOutlineMaterials).toHaveBeenCalledWith(7);
    expect(mocks.setProgress).toHaveBeenCalledWith("writing.syncingMaterials");
    expect(order).toEqual(["sync", "refresh"]);
    expect(result).toEqual(["索引待恢复"]);
  });
});
