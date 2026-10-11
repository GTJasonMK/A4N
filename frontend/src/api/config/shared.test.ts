import { describe, expect, it } from "vitest";
import {
  mergeLocalRuntimeConfigs,
  normalizeOptionalProviderConfigId,
  providerConfigIdOrSystem,
  rejectSystemDefaultMutation,
  requireSystemDefaultKeyUpdate,
  resolveProviderConnectivityTarget,
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

  it("rejects IDs outside JavaScript's exact integer range", () => {
    expect(() => providerConfigIdOrSystem(Number.MAX_SAFE_INTEGER + 1, "图片配置 ID", -1)).toThrow(
      "图片配置 ID 必须是整数",
    );
  });

  it("rejects IDs outside JavaScript's exact integer range", () => {
    expect(() => providerConfigIdOrSystem(Number.MAX_SAFE_INTEGER + 1, "图片配置 ID", -1)).toThrow(
      "图片配置 ID 必须是整数",
    );
  });

  it("rejects system-default mutations explicitly", () => {
    expect(rejectSystemDefaultMutation).toThrow(
      "系统默认配置由管理员维护，不能在用户配置页编辑",
    );
    expect(() => requireSystemDefaultKeyUpdate({ api_key_encrypted: "personal-key" })).not.toThrow();
    expect(() => requireSystemDefaultKeyUpdate({ pricing: { currency: "CNY" } })).toThrow("系统默认配置由管理员维护");
  });
});

describe("provider connectivity target", () => {
  const base = {
    label: "配置 ID",
    systemConfigId: -1,
    isSystemDefault: (configId: number | string) => Number(configId) === -1,
  };

  it("keeps server scope on the server without loading local state", async () => {
    const target = await resolveProviderConnectivityTarget({
      ...base,
      configId: "7",
      scope: "server",
      requireApiKey: true,
      loadSystemDefault: async () => ({ source: "system" }),
      getLocalConfig: () => ({ source: "local" }),
    });
    expect(target).toEqual({ configId: 7, config: null });
  });

  it("loads system and local targets through their owner callbacks", async () => {
    await expect(resolveProviderConnectivityTarget({
      ...base,
      configId: -1,
      scope: "local",
      requireApiKey: false,
      loadSystemDefault: async (requireApiKey) => ({ source: String(requireApiKey) }),
      getLocalConfig: () => ({ source: "local" }),
    })).resolves.toEqual({ configId: -1, config: { source: "false" } });
    await expect(resolveProviderConnectivityTarget({
      ...base,
      configId: 4,
      scope: "local",
      requireApiKey: true,
      loadSystemDefault: async () => ({ source: "system" }),
      getLocalConfig: (configId) => ({ source: String(configId) }),
    })).resolves.toEqual({ configId: 4, config: { source: "4" } });
  });
});
