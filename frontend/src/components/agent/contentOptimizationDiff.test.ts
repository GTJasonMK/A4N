import { describe, expect, it } from "vitest";
import type {
  AgentExecutionView,
  AgentMessageView,
  AgentThreadSnapshot,
} from "../../api/agent";
import type { OptimizationPatch } from "../../api/generated/host";
import {
  buildContentDiffSegments,
  pendingContentDiffForSnapshot,
} from "./contentOptimizationDiff";

function patch(
  operation: OptimizationPatch["operation"],
  old_text: string,
  new_text: string,
  patch_id = "patch-1",
): OptimizationPatch {
  return {
    patch_id,
    issue_id: null,
    operation,
    old_text,
    new_text,
    reason: "测试修改",
    risk: null,
  };
}

describe("content optimization diff", () => {
  it("renders replace, delete and insert patches in source order", () => {
    const segments = buildContentDiffSegments(
      "开头。保留。删除。结尾。",
      [
        patch("replace", "保留。", "更新。", "replace-1"),
        patch("delete", "删除。", "", "delete-1"),
        patch("insert_before", "结尾。", "转折。", "insert-1"),
      ],
    );

    expect(segments).toEqual([
      { kind: "text", text: "开头。" },
      { kind: "removed", text: "保留。", patchId: "replace-1" },
      { kind: "added", text: "更新。", patchId: "replace-1" },
      { kind: "removed", text: "删除。", patchId: "delete-1" },
      { kind: "added", text: "转折。", patchId: "insert-1" },
      { kind: "text", text: "结尾。" },
    ]);
  });

  it("refuses to render ambiguous or overlapping patches", () => {
    expect(
      buildContentDiffSegments("重复。重复。", [patch("delete", "重复。", "")]),
    ).toBeNull();
    expect(
      buildContentDiffSegments("甲乙丙", [
        patch("delete", "甲乙", "", "first"),
        patch("delete", "乙丙", "", "second"),
      ]),
    ).toBeNull();
  });

  it("keeps a proposal pending until a successful apply execution follows it", () => {
    const proposal: AgentMessageView = {
      id: "message-1",
      thread_id: "thread-1",
      turn_id: "turn-1",
      run_id: "run-1",
      role: "tool",
      status: "complete",
      content: "",
      source_kind: "tool",
      source_id: null,
      created_at: "2026-09-20T00:00:00Z",
      updated_at: null,
      metadata: {
        tool: "edit_article",
        execution_id: "11",
        result: {
          status: "success",
          data: {
            tool: "edit_article",
            mode: "pending_confirmation",
            chapter_id: 13,
            summary: "补足动机",
            patches: [patch("replace", "旧句", "新句")],
          },
          execution: { id: "11" },
        },
      },
    };
    const editExecution = {
      id: "11",
      status: "succeeded",
      sequence: 2,
      tool_name: "edit_article",
      result_refs: [],
    } as unknown as AgentExecutionView;
    const snapshot = {
      messages: [proposal],
      executions: [editExecution],
    } as unknown as AgentThreadSnapshot;

    expect(pendingContentDiffForSnapshot(snapshot, 13)?.summary).toBe("补足动机");

    const applied = {
      id: "12",
      status: "succeeded",
      sequence: 3,
      tool_name: "apply_optimization_patches",
      result_refs: [{ kind: "chapter", id: "13" }],
    } as unknown as AgentExecutionView;
    expect(
      pendingContentDiffForSnapshot(
        { ...snapshot, executions: [editExecution, applied] } as AgentThreadSnapshot,
        13,
      ),
    ).toBeNull();
  });
});
