import { apiInvoke } from "./index";

export interface LogReadResult {
  file_path: string;
  total_lines: number;
  returned_lines: number;
  lines: string[];
}

export interface ReadRecentLogsInput {
  lines: number;
  days_ago: number;
}

export interface SetLogLevelInput {
  filter: string;
}

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
