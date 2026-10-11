import { idNum, integerNum, type ApiId } from "../shared";

export interface ProviderConnectivityTargetOptions<T> {
  configId: ApiId;
  scope: "server" | "local";
  requireApiKey: boolean;
  label: string;
  systemConfigId: number;
  isSystemDefault: (configId: ApiId) => boolean;
  loadSystemDefault: (requireApiKey: boolean) => Promise<T>;
  getLocalConfig: (configId: number) => T | null;
}

export async function resolveProviderConnectivityTarget<T>(
  options: ProviderConnectivityTargetOptions<T>,
): Promise<{ configId: number; config: T | null }> {
  const { configId, scope, label } = options;
  if (scope === "server") {
    return { configId: idNum(configId, label), config: null };
  }
  if (options.isSystemDefault(configId)) {
    return {
      configId: options.systemConfigId,
      config: await options.loadSystemDefault(options.requireApiKey),
    };
  }
  const id = idNum(configId, label);
  return { configId: id, config: options.getLocalConfig(id) };
}

export function providerConfigIdOrSystem(
  configId: ApiId,
  label: string,
  systemConfigId: number,
): number {
  const value = integerNum(configId, label);
  return value === systemConfigId ? value : idNum(value, label);
}

export function normalizeOptionalProviderConfigId(
  configId: ApiId | null | undefined,
  label: string,
  systemConfigId: number,
): number | null {
  if (configId == null) return null;
  const value = providerConfigIdOrSystem(configId, label, systemConfigId);
  return value === systemConfigId ? null : value;
}

export function rejectSystemDefaultMutation(): never {
  throw new Error("系统默认配置由管理员维护，不能在用户配置页编辑");
}

export function requireSystemDefaultKeyUpdate(input: object): void {
  if (Object.keys(input).some((key) => key !== "api_key_encrypted")) {
    rejectSystemDefaultMutation();
  }
}

export function systemDefaultProviderConfig<
  T extends {
    id: number;
    name: string;
    has_api_key: boolean;
    is_default: boolean;
  },
>(
  configs: T[],
  systemConfigId: number,
  hasApiKey: boolean,
  isDefault: boolean,
): T | null {
  const config = configs.find((item) => item.is_default);
  return config
    ? {
        ...config,
        id: systemConfigId,
        name: `系统默认：${config.name}`,
        has_api_key: hasApiKey,
        is_default: isDefault,
      }
    : null;
}

export function requireSystemDefaultProviderConfig<T>(
  config: T | null,
  message: string,
): T {
  if (!config) throw new Error(message);
  return config;
}

export function mergeLocalRuntimeConfigs<T extends { is_default: boolean }>(
  localConfigs: T[],
  systemDefault: T | null,
  usesSystemDefault: boolean,
): T[] {
  const localItems = usesSystemDefault
    ? localConfigs.map((item) => ({ ...item, is_default: false }))
    : localConfigs;
  return systemDefault ? [systemDefault, ...localItems] : localItems;
}
