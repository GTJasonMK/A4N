import { apiInvoke } from "./index";
import type {
  LogReadResult,
  ReadLogsInput,
  SetLogLevelInput,
} from "./generated/host";

export type { LogReadResult, SetLogLevelInput } from "./generated/host";

export type ReadRecentLogsInput = ReadLogsInput;

export async function readRecentLogs(
  callerUserId: number,
  input: ReadRecentLogsInput,
): Promise<LogReadResult> {
  return apiInvoke<LogReadResult>("read_recent_logs", { callerUserId, input });
}

export async function getLogDir(callerUserId: number): Promise<string> {
  return apiInvoke<string>("get_log_dir", { callerUserId });
}

export async function setLogLevel(
  callerUserId: number,
  input: SetLogLevelInput,
): Promise<void> {
  return apiInvoke<void>("set_log_level", { callerUserId, input });
}
