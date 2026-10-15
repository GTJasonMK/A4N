import { apiInvoke } from "../index";
import type {
  DeleteTestRunInput,
  GetTestRunInput,
  ListTestRunsInput,
  StartTestRunInput,
  TestRunDetail,
  TestRunIdInput,
  TestRunSummary,
  TestScenarioDescriptor,
} from "../generated/host";

export async function listTestScenarios(): Promise<TestScenarioDescriptor[]> {
  return apiInvoke<TestScenarioDescriptor[]>("list_test_scenarios", {});
}

export async function startTestRun(input: StartTestRunInput): Promise<TestRunSummary> {
  return apiInvoke<TestRunSummary>("start_test_run", { input });
}

export async function getTestRun(
  runId: string,
  afterLogIndex?: number,
): Promise<TestRunDetail> {
  const input: GetTestRunInput = {
    run_id: runId,
    ...(afterLogIndex === undefined ? {} : { after_log_index: afterLogIndex }),
  };
  return apiInvoke<TestRunDetail>("get_test_run", { input });
}

export async function listTestRuns(limit = 20): Promise<TestRunSummary[]> {
  const input: ListTestRunsInput = { limit };
  return apiInvoke<TestRunSummary[]>("list_test_runs", { input });
}

export async function cancelTestRun(runId: string): Promise<TestRunSummary> {
  const input: TestRunIdInput = { run_id: runId };
  return apiInvoke<TestRunSummary>("cancel_test_run", { input });
}

export async function deleteTestRun(input: DeleteTestRunInput): Promise<void> {
  await apiInvoke<null>("delete_test_run", { input });
}
