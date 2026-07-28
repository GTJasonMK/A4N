import { describe, expect, it } from "vitest";
import type { GenerationTaskSummary } from "./generated/generationContracts";
import { chapterGenerationNumber } from "./chapter";

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
});
