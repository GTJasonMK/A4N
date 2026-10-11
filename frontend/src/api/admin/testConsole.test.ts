import { beforeEach, describe, expect, it, vi } from "vitest";

const { apiInvoke } = vi.hoisted(() => ({ apiInvoke: vi.fn() }));
vi.mock("../index", () => ({ apiInvoke }));

import {
  cancelTestRun,
  deleteTestRun,
  getTestRun,
  listTestRuns,
  listTestScenarios,
  startTestRun,
} from "./testConsole";

describe("admin test console API", () => {
  beforeEach(() => apiInvoke.mockReset());

  it("uses the declared commands and wraps typed inputs", async () => {
    apiInvoke.mockResolvedValueOnce([]);
    await listTestScenarios();
    expect(apiInvoke).toHaveBeenLastCalledWith("list_test_scenarios", {});

    const input = {
      scenario_id: "project_create_update",
      target: { mode: "test_project" as const, source_project_id: null },
      parameters: { title: "test" },
      confirm_impact: false,
    };
    apiInvoke.mockResolvedValueOnce({ id: "run-1" });
    await startTestRun(input);
    expect(apiInvoke).toHaveBeenLastCalledWith("start_test_run", { input });

    apiInvoke.mockResolvedValueOnce({ summary: { id: "run-1" } });
    await getTestRun("run-1");
    expect(apiInvoke).toHaveBeenLastCalledWith("get_test_run", {
      input: { run_id: "run-1" },
    });

    apiInvoke.mockResolvedValueOnce({ summary: { id: "run-1" } });
    await getTestRun("run-1", 12);
    expect(apiInvoke).toHaveBeenLastCalledWith("get_test_run", {
      input: { run_id: "run-1", after_log_index: 12 },
    });

    apiInvoke.mockResolvedValueOnce([]);
    await listTestRuns(15);
    expect(apiInvoke).toHaveBeenLastCalledWith("list_test_runs", {
      input: { limit: 15 },
    });

    apiInvoke.mockResolvedValueOnce({ id: "run-1" });
    await cancelTestRun("run-1");
    expect(apiInvoke).toHaveBeenLastCalledWith("cancel_test_run", {
      input: { run_id: "run-1" },
    });

    apiInvoke.mockResolvedValueOnce(null);
    await deleteTestRun({ run_id: "run-1", delete_test_project: true });
    expect(apiInvoke).toHaveBeenLastCalledWith("delete_test_run", {
      input: { run_id: "run-1", delete_test_project: true },
    });
  });
});
