import { describe, expect, it } from "vitest";
import { mangaStopAfterForStage } from "./mangaTabUtils";

describe("manga pipeline workspace stages", () => {
  it("maps each visible page stage to its backend stop point", () => {
    expect(mangaStopAfterForStage("page_plan")).toBe("page_slots");
    expect(mangaStopAfterForStage("page_adaptation")).toBe("page_adaptation");
    expect(mangaStopAfterForStage("storyboard")).toBe("storyboard");
    expect(mangaStopAfterForStage("prompts")).toBe("drawing_prompt");
    expect(mangaStopAfterForStage("images")).toBe("image");
  });
});
