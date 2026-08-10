import { describe, expect, it } from "vitest";
import {
  mergeLocalRuntimeConfigs,
  normalizeOptionalProviderConfigId,
  providerConfigIdOrSystem,
  rejectSystemDefaultMutation,
} from "./shared";

type Config = { id: number; is_default: boolean };

describe("mergeLocalRuntimeConfigs", () => {
  it("puts the system config first and clears local defaults", () => {
    const system: Config = { id: -1, is_default: true };
    const local: Config[] = [
      { id: 1, is_default: true },
      { id: 2, is_default: false },
    ];

    expect(mergeLocalRuntimeConfigs(local, system, true)).toEqual([
      system,
      { id: 1, is_default: false },
      { id: 2, is_default: false },
    ]);
  });

  it("keeps local default selection when system config is not active", () => {
    const local: Config[] = [{ id: 1, is_default: true }];
    expect(mergeLocalRuntimeConfigs(local, null, false)).toBe(local);
  });
});

describe("provider config IDs", () => {
  it("accepts the system sentinel and positive IDs", () => {
    expect(providerConfigIdOrSystem("-1", "图片配置 ID", -1)).toBe(-1);
    expect(providerConfigIdOrSystem("12", "图片配置 ID", -1)).toBe(12);
  });

  it("normalizes an omitted or system config to the backend default", () => {
    expect(normalizeOptionalProviderConfigId(null, "图片配置 ID", -1)).toBeNull();
    expect(normalizeOptionalProviderConfigId(undefined, "图片配置 ID", -1)).toBeNull();
    expect(normalizeOptionalProviderConfigId(-1, "图片配置 ID", -1)).toBeNull();
    expect(normalizeOptionalProviderConfigId("12", "图片配置 ID", -1)).toBe(12);
  });

  it("rejects other non-positive IDs", () => {
    expect(() => providerConfigIdOrSystem(0, "图片配置 ID", -1)).toThrow(
      "图片配置 ID 必须是正整数",
    );
    expect(() => normalizeOptionalProviderConfigId(-2, "图片配置 ID", -1)).toThrow(
      "图片配置 ID 必须是正整数",
    );
  });

  it("rejects system-default mutations explicitly", () => {
    expect(rejectSystemDefaultMutation).toThrow(
      "系统默认配置由管理员维护，不能在用户配置页编辑",
    );
  });
});
