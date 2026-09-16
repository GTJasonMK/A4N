import { apiInvoke } from "./index";
import type {
  ModelUsageReport,
  ModelUsageReportInput,
} from "./generated/host";

export type {
  ModelUsageCapabilityTotal,
  ModelUsageCurrencyTotal,
  ModelUsageEntry,
  ModelUsageReport,
  ModelUsageReportInput,
} from "./generated/host";

export async function getModelUsageReport(
  input: ModelUsageReportInput,
): Promise<ModelUsageReport> {
  return apiInvoke<ModelUsageReport>("get_model_usage_report", { input });
}
