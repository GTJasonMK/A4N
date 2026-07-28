import { describe, expect, it } from "vitest";
import type {
  GenerationEvent,
  GenerationTaskEvent,
  GenerationTaskHandle,
  GenerationTaskSnapshot,
  GenerationTaskSummary,
  StartedEvent,
} from "../api/generated/generationContracts";
import {
  createGenerationTaskViewState,
  generationTaskRecoveryIssue,
  generationTaskRecoveryIssueMessage,
  reduceGenerationTask,
  resolveFormalResultRecovery,
} from "./generationEventReducer";

const taskId = "123e4567-e89b-42d3-a456-426614174000";
const handle: GenerationTaskHandle = {
  event_schema_version: 1,
  task_id: taskId,
  attempt: 1,
  status: "reserved",
};
const started: StartedEvent = {
  delivery_mode: "stream",
  root_task_id: taskId,
  task_kind: "chapter_generation",
};

const event = (sequence: number, payload: GenerationEvent): GenerationTaskEvent => ({
  schema_version: 1,
  task_id: taskId,
  attempt: 1,
  sequence,
  emitted_at_ms: sequence,
  event: payload,
});

describe("generation event reducer", () => {
  it("applies ordered UTF-8 text and ignores duplicates", () => {
    let state = createGenerationTaskViewState(handle);
    state = reduceGenerationTask(state, {
      type: "event",
      event: event(1, { kind: "started", data: started }),
    });
    state = reduceGenerationTask(state, {
      type: "event",
      event: event(2, { kind: "status_changed", data: { status: "running" } }),
    });
    const delta = event(3, {
      kind: "text_delta",
      data: { stream_id: "main", chunk_index: 0, offset: 0, delta: "你" },
    });
    state = reduceGenerationTask(state, { type: "event", event: delta });
    state = reduceGenerationTask(state, { type: "event", event: delta });

    expect(state.streams.main).toMatchObject({ text: "你", nextOffset: 3, version: 1 });
    expect(state.nextSequence).toBe(4);
  });

  it("requires snapshot for a gap and drains queued events after recovery", () => {
    let state = createGenerationTaskViewState(handle);
    state = reduceGenerationTask(state, {
      type: "event",
      event: event(2, { kind: "status_changed", data: { status: "running" } }),
    });
    expect(state.recoveryState).toBe("required");

    state = reduceGenerationTask(state, { type: "snapshot", snapshot: snapshot(1) });
    expect(state.taskStatus).toBe("running");
    expect(state.nextSequence).toBe(3);
    expect(state.recoveryState).toBe("idle");
  });

  it("does not append a mismatched text offset", () => {
    let state = reduceGenerationTask(createGenerationTaskViewState(handle), {
      type: "snapshot",
      snapshot: snapshot(1),
    });
    state = reduceGenerationTask(state, {
      type: "event",
      event: event(2, {
        kind: "text_delta",
        data: { stream_id: "main", chunk_index: 0, offset: 1, delta: "bad" },
      }),
    });
    expect(state.recoveryState).toBe("required");
    expect(state.streams.main).toBeUndefined();
    expect(state.nextSequence).toBe(2);
  });

  it("rejects non-duplicate events after terminal", () => {
    let state = reduceGenerationTask(createGenerationTaskViewState(handle), {
      type: "snapshot",
      snapshot: { ...snapshot(2), status: "running" },
    });
    state = reduceGenerationTask(state, {
      type: "event",
      event: event(3, {
        kind: "failed",
        data: {
          batch_summary: null,
          draft_disposition: "discarded",
          error: {
            code: "generation_parse_failed",
            message: "bad output",
            retryable: false,
            source_error: null,
            stage_code: "parse",
            trace_id: null,
          },
        },
      }),
    });
    state = reduceGenerationTask(state, {
      type: "event",
      event: event(4, {
        kind: "text_delta",
        data: { stream_id: "main", chunk_index: 0, offset: 0, delta: "late" },
      }),
    });
    expect(state.clientState).toBe("protocol_error");
    expect(state.streams.main).toBeUndefined();
  });

  it("stops recovery for expired summary", () => {
    const summary: GenerationTaskSummary = {
      handle: { ...handle, status: "failed" },
      started,
      updated_at_ms: 1,
      result_ref: null,
      snapshot_state: "expired",
      terminal_event: null,
    };
    const state = reduceGenerationTask(createGenerationTaskViewState(handle), {
      type: "summary",
      summary,
    });
    expect(state.clientState).toBe("detached");
    expect(state.recoveryState).toBe("expired");
  });

  it.each([
    ["consistency_error", "failed", "consistency_error"],
    ["detached", "expired", "expired"],
    ["detached", "corrupt", "corrupt"],
    ["detached", "failed", null],
  ] as const)("identifies %s/%s recovery issue", (clientState, recoveryState, issue) => {
    expect(generationTaskRecoveryIssue({ clientState, recoveryState })).toBe(issue);
  });

  it("keeps the canonical result reference without publishing an inconsistent terminal", () => {
    const resultRef = { kind: "chapter", id: "42", revision: "a".repeat(64) };
    const state = reduceGenerationTask(createGenerationTaskViewState(handle), {
      type: "consistency_error",
      resultRef,
    });

    expect(state.clientState).toBe("consistency_error");
    expect(state.recoveryState).toBe("failed");
    expect(state.resultRef).toEqual(resultRef);
    expect(state.terminalReceived).toBe(false);
    expect(state.taskStatus).not.toBe("completed");
  });

  it.each([
    "chapter",
    "blueprint",
    "optimize_turn",
    "inspiration_message",
  ] as const)("accepts verified terminal for %s without a second recovery path", (domain) => {
    const outcome = resolveFormalResultRecovery({
      recoveryIssue: null,
      consistent: true,
      hasPersisted: true,
      hasResultRef: true,
      domain,
    });
    expect(outcome.kind).toBe("ok");
    expect(outcome.shouldMarkConsistencyError).toBe(false);
    expect(outcome.warning).toBeNull();
  });

  it.each([
    ["chapter", "expired", true, "readonly", "已只读加载正式章节供核对"],
    ["chapter", "corrupt", false, "unrecoverable", "正式章节不可用，已恢复编辑器快照"],
    ["chapter", "consistency_error", false, "unrecoverable", "且没有正式结果引用，无法恢复；已恢复编辑器快照"],
    ["blueprint", "expired", true, "readonly", "当前展示正式蓝图，只供核对"],
    ["blueprint", null, false, "unrecoverable", "正式蓝图不可用"],
    ["optimize_turn", "corrupt", true, "readonly", "已只读加载正式 Turn 供核对"],
    ["optimize_turn", "consistency_error", false, "unrecoverable", "且没有正式结果引用，无法恢复"],
    ["inspiration_message", "expired", true, "readonly", "以下内容来自正式消息，只供核对"],
    ["inspiration_message", null, false, "unrecoverable", "正式消息不可用，请稍后从历史记录重新加载"],
  ] as const)(
    "resolves %s recovery issue=%s persisted=%s as %s",
    (domain, recoveryIssue, hasPersisted, kind, expectedFragment) => {
      const hasResultRef = expectedFragment !== "且没有正式结果引用，无法恢复"
        && expectedFragment !== "且没有正式结果引用，无法恢复；已恢复编辑器快照";
      const outcome = resolveFormalResultRecovery({
        recoveryIssue,
        consistent: false,
        hasPersisted,
        hasResultRef: recoveryIssue === "consistency_error" && !hasPersisted
          ? expectedFragment.includes("没有正式结果引用")
            ? false
            : true
          : hasResultRef,
        domain,
      });
      expect(outcome.kind).toBe(kind);
      expect(outcome.shouldMarkConsistencyError).toBe(recoveryIssue == null);
      if (recoveryIssue) {
        expect(outcome.issueMessage).toBe(generationTaskRecoveryIssueMessage(recoveryIssue));
      }
      if (kind === "readonly") {
        expect(outcome.warning).toContain(expectedFragment);
        expect(outcome.message).toContain(
          domain === "blueprint" ? "已只读加载正式蓝图供核对" : expectedFragment,
        );
      } else {
        expect(outcome.message).toContain(expectedFragment);
        expect(outcome.warning).toBeNull();
      }
    },
  );
});

function snapshot(coversThroughSequence: number): GenerationTaskSnapshot {
  return {
    schema_version: 1,
    task_id: taskId,
    attempt: 1,
    status: "reserved",
    started,
    covers_through_sequence: coversThroughSequence,
    updated_at_ms: 1,
    progress: null,
    streams: [],
    artifacts: [],
    usage: null,
    latest_checkpoint: null,
    cancel_reason: null,
    terminal_event: null,
  };
}
