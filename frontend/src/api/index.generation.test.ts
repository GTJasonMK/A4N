import { afterEach, expect, it, vi } from "vitest";

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
  writeCurrentTokens: vi.fn(),
}));
vi.mock("../utils/logger", () => ({
  logger: { scoped: () => ({ error: mocks.logError }) },
}));

import { apiInvoke, extractErrorMessage } from "./index";

afterEach(() => {
  vi.unstubAllGlobals();
});

it("rejects unknown generation envelope fields before flattening the web body", async () => {
  await expect(
    apiInvoke("start_chapter_generation", { userId: 7, input: {} }),
  ).rejects.toThrow("start_chapter_generation 不接受参数: userId");
});

it("accepts only the canonical web error contract", async () => {
  const fetchMock = vi.fn()
    .mockResolvedValueOnce(new Response(JSON.stringify({
      code: "invalid_request",
      message: "请求无效",
      retryable: false,
      trace_id: null,
      details: {},
    }), { status: 400 }))
    .mockResolvedValueOnce(new Response(JSON.stringify({
      code: "invalid_request",
      message: "请求无效",
      retryable: false,
      trace_id: null,
    }), { status: 400 }));
  vi.stubGlobal("fetch", fetchMock);

  await expect(apiInvoke("login_user", { input: {} })).rejects.toThrow("请求无效");
  await expect(apiInvoke("login_user", { input: {} })).rejects.toThrow(
    "Server 返回的 HTTP 400 错误体不符合 A4N API 合同",
  );
});

it("does not interpret legacy detail error objects", () => {
  expect(extractErrorMessage({ detail: "legacy" }, "认证失败")).toBe("认证失败");
});
