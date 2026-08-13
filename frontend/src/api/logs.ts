import { apiInvoke } from "./index";
import type {
  LogReadResult,
  ReadLogsInput,
  SetLogLevelInput,
} from "./generated/host";

export type { LogReadResult, ReadLogsInput, SetLogLevelInput } from "./generated/host";

export async function readRecentLogs(
  input: ReadLogsInput,
): Promise<LogReadResult> {
  return apiInvoke<LogReadResult>("read_recent_logs", { input });
}

export async function getLogDir(): Promise<string> {
  return apiInvoke<string>("get_log_dir");
}

export async function setLogLevel(
  input: SetLogLevelInput,
): Promise<void> {
  return apiInvoke<void>("set_log_level", { input });
}
