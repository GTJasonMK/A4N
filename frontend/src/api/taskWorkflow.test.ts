import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ apiInvoke: vi.fn() }));

vi.mock("./index", () => ({ apiInvoke: mocks.apiInvoke }));

import {
  cancelTask,
  getQueueStats,
  getTaskHistory,
  getTaskProgressEvents,
} from "./task";
import { cancelAdminTask } from "./admin/tasks";
import {
  cancelWorkflow,
  cancelWorkflowTask,
  getWorkflowRunDetail,
  getWorkflowRunProgressEvents,
  listActiveWorkflows,
  listWorkflowRuns,
  resumeWorkflowRun,
} from "./streaming";

beforeEach(() => {
  mocks.apiInvoke.mockReset();
  mocks.apiInvoke.mockImplementation((command: string) => {
    if (command === "cancel_task" || command === "cancel_admin_task") {
      return Promise.resolve(true);
    }
    if (command === "cancel_workflow") return Promise.resolve(false);
    if (command === "get_workflow_progress_events") {
      return Promise.resolve({ events: [], stream_text: null });
    }
    return Promise.resolve(null);
  });
});

describe("task and workflow host contracts", () => {
  it("uses generated inputs without caller-controlled user identity", async () => {
    await expect(cancelTask("task-1")).resolves.toBe(true);
    await expect(cancelAdminTask("admin-task-1")).resolves.toBe(true);
    await getQueueStats();
    await getTaskHistory(7, 0, 30);
    await getTaskProgressEvents("task-1", 20);
    await getTaskProgressEvents("task-2", 20, "task-2-event-14");
    await listActiveWorkflows();
    await listWorkflowRuns({ resumable_only: false, limit: 40 });
    await getWorkflowRunDetail(9, true);
    await getWorkflowRunDetail(10, false, true);
    await getWorkflowRunDetail(11, false, true, false);
    await getWorkflowRunDetail(12, true, false, true, false);
    await resumeWorkflowRun(9);
    await getWorkflowRunProgressEvents(9, 20);
    await getWorkflowRunProgressEvents(9, 20, "81");
    await expect(cancelWorkflowTask("task-1")).resolves.toBe(false);
    await expect(cancelWorkflow({ task_id: null, workflow_run_id: 9 })).resolves.toBe(false);

    expect(mocks.apiInvoke.mock.calls).toEqual([
      ["cancel_task", { taskId: "task-1" }],
      ["cancel_admin_task", { taskId: "admin-task-1" }],
      ["get_queue_stats"],
      ["get_task_history", { projectId: 7, offset: 0, limit: 30 }],
      [
        "get_workflow_progress_events",
        { input: { task_id: "task-1", workflow_run_id: null, limit: 20 } },
      ],
      [
        "get_workflow_progress_events",
        {
          input: {
            task_id: "task-2",
            workflow_run_id: null,
            limit: 20,
            after_event_id: "task-2-event-14",
          },
        },
      ],
      ["list_active_workflows"],
      ["list_workflow_runs", { input: { resumable_only: false, limit: 40 } }],
      [
        "get_workflow_run_detail",
        { input: { workflow_run_id: 9, include_generation_runs: true, include_generation_run_count: false, include_checkpoint_details: true } },
      ],
      [
        "get_workflow_run_detail",
        { input: { workflow_run_id: 10, include_generation_runs: false, include_generation_run_count: true, include_checkpoint_details: true } },
      ],
      [
        "get_workflow_run_detail",
        { input: { workflow_run_id: 11, include_generation_runs: false, include_generation_run_count: true, include_checkpoint_details: false } },
      ],
      [
        "get_workflow_run_detail",
        {
          input: {
            workflow_run_id: 12,
            include_generation_runs: true,
            include_generation_run_count: false,
            include_checkpoint_details: true,
            include_generation_run_payloads: false,
          },
        },
      ],
      ["resume_workflow_run", { input: { workflow_run_id: 9 } }],
      [
        "get_workflow_progress_events",
        { input: { task_id: null, workflow_run_id: 9, limit: 20 } },
      ],
      [
        "get_workflow_progress_events",
        { input: { task_id: null, workflow_run_id: 9, limit: 20, after_event_id: "81" } },
      ],
      [
        "cancel_workflow",
        { input: { task_id: "task-1", workflow_run_id: null } },
      ],
      [
        "cancel_workflow",
        { input: { task_id: null, workflow_run_id: 9 } },
      ],
    ]);
  });
});
