import { describe, expect, it } from "vitest";
import {
  mangaPageNumberForPipelineStage,
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
});
