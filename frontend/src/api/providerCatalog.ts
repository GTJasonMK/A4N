import { apiInvoke } from ".";
import type { ModelProviderCatalog } from "./generated/config";

let catalogPromise: Promise<ModelProviderCatalog> | null = null;

export function listModelProviderCatalog(): Promise<ModelProviderCatalog> {
  catalogPromise ??= apiInvoke<ModelProviderCatalog>("list_model_provider_catalog").catch(
    (error) => {
      catalogPromise = null;
      throw error;
    },
  );
  return catalogPromise;
}

export function assertCatalogProvider(
  capability: keyof ModelProviderCatalog,
  providerId: string,
  catalog: ModelProviderCatalog,
): void {
  if (!catalog[capability].some((provider) => provider.id === providerId)) {
    throw new Error(`${capability} provider 未注册：${providerId}`);
  }
}
