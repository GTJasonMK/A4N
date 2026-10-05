import { describe, expect, it } from "vitest";
import type { MangaPagePromptSummary } from "../../../api/manga";
import { buildPagePrompts, type MangaPagePromptLabels } from "./mangaPagePromptModel";

const labels: MangaPagePromptLabels = {
  role: {
    opening: "opening",
    setup: "setup",
    rising: "rising",
    climax: "climax",
    falling: "falling",
    resolution: "resolution",
    transition: "transition",
  },
  pacing: { slow: "slow", medium: "medium", fast: "fast", explosive: "explosive" },
  importance: { critical: "critical", high: "high", normal: "normal", low: "low" },
  readingFlow: {
    top_to_bottom_left_to_right: "top-to-bottom",
    z_path: "z",
    s_path: "s",
    diagonal: "diagonal",
  },
  pageFunction: "function",
  pagePacing: "pacing",
  importanceLabel: "importance",
  readingFlowLabel: "reading flow",
  pageGoal: "goal",
  visualThesis: "visual thesis",
  entryState: "entry",
  exitState: "exit",
  dominantPanel: "dominant panel",
  panelCount: "panels",
  primaryPortrait: "primary portrait",
  primaryImage: "primary image",
  projectImage: "project image",
  otherPortrait: "other portrait",
};

describe("buildPagePrompts", () => {
  it("shows a linked completed image ahead of a stale failed pipeline checkpoint", () => {
    const prompt: MangaPagePromptSummary = {
      id: 1,
      chapter_id: 2,
      project_id: 3,
      page_number: 1,
      page_id: "page-0001",
      image_prompt: "",
      dialogue_text: null,
      page_content: {
        summary: "A character finds the letter.",
        source_span_ids: [],
        beat_refs: [{ unit_id: "unit-1", beat_id: "beat-1" }],
        action: "Finds the letter.",
        reaction: null,
        dialogue_intent: null,
        entry_state: "The desk is closed.",
        exit_state: "The letter is open.",
      },
      page_adaptation: null,
      page_plan: null,
      drawing_prompt: null,
      reference_bindings: [],
      source: {
        adaptation_plan_id: 4,
        adaptation_plan_revision: 1,
        adaptation_plan_sha256: "adaptation-hash",
        content_sha256: "source-hash",
        root_project_id: 3,
        source_chapter_id: 2,
        source_revision_key: "revision-1",
        source_snapshot_chapter_id: null,
        source_snapshot_id: null,
      },
      generated_image_id: null,
      status: "pending",
      pipeline_stage: "adaptation",
      pipeline_status: "running",
      pipeline_error: {
        stage: "adaptation",
        code: "provider_error",
        message: "temporarily unavailable",
        retryable: true,
      },
      created_at: "2026-09-29T00:00:00Z",
      updated_at: "2026-09-29T00:00:00Z",
    };

    const [page] = buildPagePrompts([prompt], null, labels);

    expect(page.pipelineStage).toBe("adaptation");
    expect(page.pipelineStatus).toBe("running");
    expect(page).not.toHaveProperty("pipelineError");
    expect(page.pageContent?.summary).toBe("A character finds the letter.");
    expect(page.promptCount).toBe(0);
    expect(page.characterNames).toEqual([]);
    expect(page.layoutItems).toEqual([]);

    const [completedPage] = buildPagePrompts(
      [{
        ...prompt,
        generated_image_id: 44,
        status: "completed",
        pipeline_stage: "image",
        pipeline_status: "failed",
      }],
      null,
      labels,
    );

    expect(completedPage.pipelineStatus).toBe("completed");
    expect(completedPage.prompt.status).toBe("completed");
  });
});
