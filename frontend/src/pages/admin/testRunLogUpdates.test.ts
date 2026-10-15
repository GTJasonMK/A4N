import { describe, expect, it } from "vitest";
import type { TestRunDetail } from "../../api/generated/host";
import { mergeTestRunLogUpdate } from "./testRunLogUpdates";

function testRun(logStartIndex: number, logs: string[]): TestRunDetail {
  return {
    log_start_index: logStartIndex,
    logs,
    parameters: {},
    steps: [],
    summary: {
      id: "run-1",
      scenario_id: "scenario",
      scenario_version: 1,
      status: "running",
      target: { mode: "repository" },
      target_project_id: null,
      created_by: 1,
      created_at: "2026-10-11T00:00:00Z",
      started_at: null,
      finished_at: null,
      duration_ms: null,
      message: null,
      impact: [],
    },
  };
}

describe("test run log updates", () => {
  it("appends only new logs and follows the server's retained window", () => {
    const current = testRun(0, Array.from({ length: 500 }, (_, index) => `line ${index}`));
    const update = testRun(1, ["line 500"]);

    const merged = mergeTestRunLogUpdate(current, update, 500);

    expect(merged.log_start_index).toBe(1);
    expect(merged.logs).toHaveLength(500);
    expect(merged.logs[0]).toBe("line 1");
    expect(merged.logs.at(-1)).toBe("line 500");
  });

  it("replaces the window when the requested cursor has expired", () => {
    const current = testRun(0, ["line 0", "line 1", "line 2"]);
    const update = testRun(1, ["line 1", "line 2", "line 3"]);

    expect(mergeTestRunLogUpdate(current, update, 0)).toBe(update);
  });
});
