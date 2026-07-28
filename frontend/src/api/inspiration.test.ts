import { describe, expect, it } from "vitest";
import type { GenerationTaskSummary } from "./generated/generationContracts";
import { inspirationGenerationProject } from "./inspiration";

const summary = (
  projectId: number,
  targetId: string,
  kind = "inspiration_conversation",
): GenerationTaskSummary => ({
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
    project_id: projectId,
    root_task_id: "123e4567-e89b-42d3-a456-426614174000",
    target: { id: targetId, kind },
    task_kind: "llm_call",
  },
  terminal_event: null,
  updated_at_ms: 1,
});

describe("inspiration generation target", () => {
  it("accepts only the canonical target for the current project", () => {
    expect(inspirationGenerationProject(summary(7, "7"), 7)).toBe(7);
    expect(inspirationGenerationProject(summary(8, "8"), 7)).toBeNull();
    expect(inspirationGenerationProject(summary(7, "7", "chapter"), 7)).toBeNull();
    expect(inspirationGenerationProject(summary(7, "8"), 7)).toBeNull();
  });
});
