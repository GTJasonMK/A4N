import { describe, expect, it } from "vitest";
import type { MediaAuthoringWorkspace } from "../../api/mediaAuthoring";
import { authoringStageLabel } from "./useMediaAuthoring";

const translate = ((key: string) => key) as Parameters<typeof authoringStageLabel>[3];

function workspace(
  documents: Array<{
    kind?: string;
    source_chapter_id: number | null;
    source_snapshot_id: number;
    status: string;
    stale?: boolean;
  }>,
  chapterCount = 2,
): MediaAuthoringWorkspace {
  return {
    source_snapshot_id: 2,
    chapters: Array.from({ length: chapterCount }, (_, index) => ({
      chapter_number: index + 1,
      source: { source_chapter_id: index + 1 },
      source_spans: [],
      title: null,
    })),
    documents: documents.map((document, index) => ({
      record: {
        id: index + 1,
        kind: document.kind ?? "adaptation_plan",
        project_id: 1,
        revision: 1,
        sha256: `hash-${index}`,
        source_chapter_id: document.source_chapter_id,
        source_snapshot_id: document.source_snapshot_id,
        status: document.status,
        version: 1,
        inputs: { dependencies: [], preprocessing_fingerprint: null, sources: [], video_reference: null },
        document: {},
      },
      stale: document.stale ?? false,
      video_reference_status: null,
    })),
    artifacts: [],
    target: "manga",
  } as unknown as MediaAuthoringWorkspace;
}

describe("authoringStageLabel", () => {
  it("prefers a current draft over an adopted document for each scope", () => {
    const result = authoringStageLabel(workspace([
      { source_chapter_id: 1, source_snapshot_id: 2, status: "adopted" },
      { source_chapter_id: 1, source_snapshot_id: 2, status: "draft" },
      { source_chapter_id: 2, source_snapshot_id: 2, status: "adopted" },
    ]), "adaptation_plan", undefined, translate);
    expect(result).toBe("common.mediaAuthoring.draft");
  });

  it("marks scopes with only an older snapshot as needing an update", () => {
    const result = authoringStageLabel(workspace([
      { source_chapter_id: 1, source_snapshot_id: 1, status: "adopted" },
      { source_chapter_id: 2, source_snapshot_id: 2, status: "adopted" },
    ]), "adaptation_plan", undefined, translate);
    expect(result).toBe("common.mediaAuthoring.needsUpdate");
  });

  it("handles the null scope used by Galgame branch plans", () => {
    const result = authoringStageLabel(workspace([
      { kind: "galgame_branch_plan", source_chapter_id: null, source_snapshot_id: 2, status: "adopted" },
    ]), "galgame_branch_plan", undefined, translate);
    expect(result).toBe("common.mediaAuthoring.adopted");
  });
});
