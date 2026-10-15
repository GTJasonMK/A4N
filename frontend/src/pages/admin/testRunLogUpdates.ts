import type { TestRunDetail } from "../../api/generated/host";

export function mergeTestRunLogUpdate(
  current: TestRunDetail | null,
  update: TestRunDetail,
  afterLogIndex: number,
): TestRunDetail {
  if (
    !current
    || current.summary.id !== update.summary.id
    || afterLogIndex < update.log_start_index
  ) return update;

  const startIndex = Math.max(current.log_start_index, update.log_start_index);
  const retainedLogs = current.logs.slice(startIndex - current.log_start_index);
  const currentEndIndex = current.log_start_index + current.logs.length;
  const updateStartIndex = Math.max(afterLogIndex, update.log_start_index);
  const overlap = Math.max(0, currentEndIndex - updateStartIndex);
  return {
    ...update,
    log_start_index: startIndex,
    logs: [...retainedLogs, ...update.logs.slice(overlap)],
  };
}
