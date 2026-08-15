import { describe, expect, it } from "vitest";
import {
  createReferenceMediaDraft,
  normalizeReferenceMediaRows,
  referenceMediaDraftScopeKey,
  referenceMediaDraftFromRecipe,
  referenceMediaDraftError,
  referenceMediaRecipeFromDraft,
} from "./referenceMediaWorkspace";

describe("reference media draft persistence", () => {
  it("binds a draft to the plan and stable segment identity", () => {
    const current = referenceMediaDraftScopeKey("plan:41", 1, "shot-1", 1);
    expect(current).toBeTruthy();
    expect(referenceMediaDraftScopeKey("plan:41", 1, "shot-1", 1)).toBe(current);
    expect(referenceMediaDraftScopeKey("plan:41", 2, "shot-1", 1)).not.toBe(current);
    expect(referenceMediaDraftScopeKey("plan:42", 1, "shot-1", 1)).not.toBe(current);
    expect(referenceMediaDraftScopeKey("plan:41", 1, "shot-2", 1)).not.toBe(current);
    expect(referenceMediaDraftScopeKey("plan:41", 1, "shot-1", 0)).toBeNull();
  });

  it("keeps a selected local file row and requires reselection after reload", () => {
    const draft = createReferenceMediaDraft();
    draft.imageRows = [{
      id: "image-1",
      url: "",
      usage: "",
      uploadSha256: "abc123",
      uploadFileName: "reference.png",
      uploadMimeType: "image/png",
    }];

    const recipe = referenceMediaRecipeFromDraft(draft);
    expect(recipe.image_rows).toHaveLength(1);
    expect(recipe.image_rows[0]).toMatchObject({
      upload_sha256: "abc123",
      upload_file_name: "reference.png",
      upload_mime_type: "image/png",
    });

    const restored = referenceMediaDraftFromRecipe(recipe);
    expect(normalizeReferenceMediaRows(restored.imageRows)).toHaveLength(1);
    expect(referenceMediaDraftError(restored.prompt, {
      image: restored.imageRows,
      video: restored.videoRows,
      audio: restored.audioRows,
    })).toContain("缺少 URL 或本地文件");
  });
});
