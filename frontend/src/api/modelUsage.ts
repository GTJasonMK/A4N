import { apiInvoke } from "./index";
import type {
  ModelUsageReport,
  ModelUsageReportInput,
  ModelUsageRequest,
  ModelUsageRequestInput,
} from "./generated/host";

export type {
  ModelUsageCapabilityTotal,
  ModelUsageCurrencyTotal,
  ModelUsageEntry,
  ModelUsageReport,
  ModelUsageReportInput,
  ModelUsageRequest,
  ModelUsageRequestInput,
} from "./generated/host";

export async function getModelUsageReport(
  input: ModelUsageReportInput,
): Promise<ModelUsageReport> {
  return apiInvoke<ModelUsageReport>("get_model_usage_report", { input });
}

export async function getModelUsageRequest(
  input: ModelUsageRequestInput,
): Promise<ModelUsageRequest> {
  return apiInvoke<ModelUsageRequest>("get_model_usage_request", { input });
}
