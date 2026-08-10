import { describe, expect, it } from "vitest";
import type { GenerationTaskSummary } from "./generated/generationContracts";
import {
  chapterContentCommitToastType,
  chapterGenerationNumber,
  shouldPropagateGenerationRagPrefetchError,
} from "./chapter";

const summary = (targetId: string, kind = "chapter"): GenerationTaskSummary => ({
  handle: {
    attempt: 1,
    event_schema_version: 1,
    status: "running",
    task_id: "123e4567-e89b-42d3-a456-426614174000",
  },
  result_ref: null,
  snapshot_state: "available",
  started: {
    delivery_mode: "stream",
    project_id: 7,
    root_task_id: "123e4567-e89b-42d3-a456-426614174000",
    target: { id: targetId, kind },
    task_kind: "llm_call",
  },
  terminal_event: null,
  updated_at_ms: 1,
});

describe("chapter generation target", () => {
  it("accepts only the canonical target for the current project", () => {
    expect(chapterGenerationNumber(summary("7:3"), 7)).toBe(3);
    expect(chapterGenerationNumber(summary("8:3"), 7)).toBeNull();
    expect(chapterGenerationNumber(summary("7:3:1"), 7)).toBeNull();
    expect(chapterGenerationNumber(summary("7:3", "image"), 7)).toBeNull();
  });

  it("propagates RAG contract errors but allows optional capability failures", () => {
    expect(
      shouldPropagateGenerationRagPrefetchError("Embedding identity 不匹配"),
    ).toBe(true);
    expect(
      shouldPropagateGenerationRagPrefetchError("参数无效：query vector 维度不匹配"),
    ).toBe(true);
    expect(
      shouldPropagateGenerationRagPrefetchError("嵌入计算失败：本机模型未安装"),
    ).toBe(false);
    expect(
      shouldPropagateGenerationRagPrefetchError("向量检索失败：服务暂时不可用"),
    ).toBe(false);
  });

  it("keeps a committed chapter successful when derived sync needs recovery", () => {
    expect(chapterContentCommitToastType("failed")).toBe("info");
    expect(chapterContentCommitToastType("running")).toBe("info");
    expect(chapterContentCommitToastType("completed")).toBe("success");
  });
});
