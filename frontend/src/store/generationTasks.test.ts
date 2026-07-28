import { afterEach, describe, expect, it, vi } from "vitest";
import type { GenerationTaskHandle } from "../api/generated/generationContracts";
import type { GenerationTaskReducerAction } from "../utils/generationEventReducer";
import {
  generationTaskKey,
  getGenerationTaskViewState,
  useGenerationTasksStore,
} from "./generationTasks";

const handle = (taskId: string): GenerationTaskHandle => ({
  event_schema_version: 1,
  task_id: taskId,
  attempt: 1,
  status: "reserved",
});

const progress = (taskId: string, current: number): GenerationTaskReducerAction => ({
  type: "event" as const,
  event: {
    schema_version: 1,
    task_id: taskId,
    attempt: 1,
    sequence: 1,
    emitted_at_ms: 1,
    event: {
      kind: "progress" as const,
      data: {
        current,
        total: 10,
        message: `step ${current}`,
        unit_label: null,
        stage_code: "stream",
        phase_kind: "stream",
        severity: "info",
        estimated: false,
        item_index: null,
        item_key: null,
        item_total: null,
      },
    },
  },
});

describe("generation task store", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("keeps one authoritative task while retaining and releasing multiple views", () => {
    const task = handle("123e4567-e89b-42d3-a456-426614174000");
    const key = generationTaskKey(task.task_id, task.attempt);
    const store = useGenerationTasksStore.getState();

    store.retain(task);
    store.retain(task);
    expect(useGenerationTasksStore.getState().references[key]).toBe(2);

    expect(store.release(key)).toBe(1);
    expect(useGenerationTasksStore.getState().tasks[key].clientState).toBe("attached");
    expect(store.release(key)).toBe(0);
    expect(useGenerationTasksStore.getState().tasks[key].clientState).toBe("detached");
    expect(getGenerationTaskViewState(task.task_id, task.attempt)?.clientState).toBe("detached");

    store.clear(key);
  });

  it("batches ordinary events for 50ms while flushing terminal state immediately", () => {
    vi.useFakeTimers();
    const task = handle("223e4567-e89b-42d3-a456-426614174000");
    const key = generationTaskKey(task.task_id, task.attempt);
    const store = useGenerationTasksStore.getState();
    store.retain(task);

    store.dispatch(key, progress(task.task_id, 1));
    expect(useGenerationTasksStore.getState().tasks[key].progress).toBeNull();
    vi.advanceTimersByTime(49);
    expect(useGenerationTasksStore.getState().tasks[key].progress).toBeNull();
    vi.advanceTimersByTime(1);
    expect(useGenerationTasksStore.getState().tasks[key].progress?.current).toBe(1);

    store.dispatch(key, {
      type: "event",
      event: {
        schema_version: 1,
        task_id: task.task_id,
        attempt: 1,
        sequence: 2,
        emitted_at_ms: 2,
        event: {
          kind: "cancelled",
          data: {
            batch_summary: null,
            draft_disposition: "discarded",
            reason: "user_requested",
            summary: null,
          },
        },
      },
    });
    expect(useGenerationTasksStore.getState().tasks[key].terminalReceived).toBe(true);

    store.clear(key);
  });

  it("clears both the view and authoritative state", () => {
    const task = handle("323e4567-e89b-42d3-a456-426614174000");
    const key = generationTaskKey(task.task_id, task.attempt);
    const store = useGenerationTasksStore.getState();
    store.retain(task);
    store.clear(key);

    expect(useGenerationTasksStore.getState().tasks[key]).toBeUndefined();
    expect(useGenerationTasksStore.getState().references[key]).toBeUndefined();
    expect(getGenerationTaskViewState(task.task_id, task.attempt)).toBeNull();
    expect(store.dispatch(key, { type: "recovery_state", state: "failed" })).toBeNull();
  });
});
