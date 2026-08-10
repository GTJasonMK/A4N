import { idNum, integerNum, type ApiId } from "../shared";

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
