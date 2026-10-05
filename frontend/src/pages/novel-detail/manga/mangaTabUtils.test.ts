import { describe, expect, it } from "vitest";
import {
  mangaImageStatusMatchesWorkspaceStage,
  mangaPipelineStageMatchesWorkspaceStage,
  mangaPageNumberForPipelineStage,
  mangaStageAllowsReferenceBinding,
  mangaStopAfterForStage,
} from "./mangaTabUtils";

describe("manga pipeline workspace stages", () => {
  it("maps each visible page stage to its backend stop point", () => {
    expect(mangaStopAfterForStage("page_plan")).toBe("page_slots");
    expect(mangaStopAfterForStage("page_adaptation")).toBe("page_adaptation");
    expect(mangaStopAfterForStage("storyboard")).toBe("storyboard");
    expect(mangaStopAfterForStage("prompts")).toBe("drawing_prompt");
    expect(mangaStopAfterForStage("images")).toBe("image");
  });

  it("scopes per-page stages to the selected page and keeps pagination chapter-wide", () => {
    expect(mangaPageNumberForPipelineStage("page_plan", 2)).toBeNull();
    expect(mangaPageNumberForPipelineStage("page_adaptation", 2)).toBe(2);
    expect(mangaPageNumberForPipelineStage("storyboard", 2)).toBe(2);
    expect(mangaPageNumberForPipelineStage(undefined, 2)).toBeNull();
  });

  it("allows reference binding in prompt and image stages", () => {
    expect(mangaStageAllowsReferenceBinding(undefined)).toBe(true);
    expect(mangaStageAllowsReferenceBinding("prompts")).toBe(true);
    expect(mangaStageAllowsReferenceBinding("images")).toBe(true);
    expect(mangaStageAllowsReferenceBinding("page_plan")).toBe(false);
    expect(mangaStageAllowsReferenceBinding("page_adaptation")).toBe(false);
    expect(mangaStageAllowsReferenceBinding("storyboard")).toBe(false);
  });

  it("keeps a pipeline error in the workspace stage that produced it", () => {
    expect(mangaPipelineStageMatchesWorkspaceStage("images", "image")).toBe(true);
    expect(mangaPipelineStageMatchesWorkspaceStage("storyboard", "image")).toBe(false);
    expect(mangaPipelineStageMatchesWorkspaceStage("page_plan", "page_content")).toBe(true);
    expect(mangaPipelineStageMatchesWorkspaceStage("page_plan", "image")).toBe(false);
    expect(mangaPipelineStageMatchesWorkspaceStage("adaptation", "adaptation")).toBe(false);
    expect(mangaPipelineStageMatchesWorkspaceStage("page_adaptation", "adaptation")).toBe(true);
    expect(mangaPipelineStageMatchesWorkspaceStage(undefined, "image")).toBe(true);
    expect(mangaPipelineStageMatchesWorkspaceStage("storyboard", null)).toBe(false);
  });

  it("shows image-generation status only in the image stage or combined workspace", () => {
    expect(mangaImageStatusMatchesWorkspaceStage("images")).toBe(true);
    expect(mangaImageStatusMatchesWorkspaceStage("storyboard")).toBe(false);
    expect(mangaImageStatusMatchesWorkspaceStage("materials")).toBe(false);
    expect(mangaImageStatusMatchesWorkspaceStage(undefined)).toBe(true);
  });
});
