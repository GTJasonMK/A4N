import { expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ logError: vi.fn() }));

vi.mock("./runtime", () => ({
  apiBaseUrl: () => "http://localhost/api",
  runtimeInvoke: vi.fn(),
  shouldUseHttpBackend: () => true,
}));
vi.mock("./runtimeConfigSnapshot", () => ({
  withRuntimeConfigSnapshot: (_cmd: string, args?: Record<string, unknown>) => args,
}));
vi.mock("./authStorage", () => ({
  readCurrentAccessToken: vi.fn(() => null),
  readCurrentRefreshToken: vi.fn(() => null),
  readCurrentSessionId: vi.fn(() => null),
  writeCurrentAccessToken: vi.fn(),
  writeCurrentRefreshToken: vi.fn(),
  writeCurrentSessionId: vi.fn(),
}));
vi.mock("../utils/logger", () => ({
  logger: { scoped: () => ({ error: mocks.logError }) },
}));

import { apiInvoke } from "./index";

it("rejects unknown generation envelope fields before flattening the web body", async () => {
  await expect(
    apiInvoke("start_chapter_generation", { userId: 7, input: {} }),
  ).rejects.toThrow("start_chapter_generation 不接受参数: userId");
});
