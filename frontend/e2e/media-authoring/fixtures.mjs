import { createHash } from "node:crypto";

const defaultText = "雨从旧书店的屋檐落下。她推开门。";
const defaultHash = createHash("sha256").update(defaultText).digest("hex");

export function authoringChapters(projectId, sourceChapters = null) {
  if (sourceChapters) return sourceChapters;
  return [1, 2].map((number) => ({
    id: projectId * 100 + number,
    snapshot_id: projectId + 10,
    root_project_id: 1,
    source_chapter_id: number,
    chapter_number: number,
    title: number === 1 ? "旧书店" : "未寄出的信",
    content: defaultText,
    summary: null,
    analysis_data_json: null,
    content_hash: defaultHash,
    analysis_hash: null,
  }));
}

export function authoringWorkspace(projectId, target, sourceChapters = null) {
  const chapters = authoringChapters(projectId, sourceChapters);
  const snapshotId = chapters[0]?.snapshot_id ?? projectId + 10;
  const sources = chapters.map((chapter) => {
    const source = {
      root_project_id: chapter.root_project_id ?? 1,
      source_chapter_id: chapter.source_chapter_id,
      source_snapshot_id: chapter.snapshot_id ?? snapshotId,
      source_snapshot_chapter_id: chapter.id,
      content_sha256: chapter.content_hash ?? "a".repeat(64),
    };
    const spans = (chapter.content ?? defaultText).split("。")
      .filter(Boolean)
      .slice(0, 2)
      .map((text, index) => ({ id: `ch${chapter.chapter_number}:s${index + 1}:f1`, text: `${text}。` }));
    return {
      source,
      chapter_number: chapter.chapter_number,
      title: chapter.title,
      source_spans: spans.length > 0 ? spans : [{ id: `ch${chapter.chapter_number}:s1:f1`, text: "" }],
    };
  });
  return {
    target,
    source_snapshot_id: snapshotId,
    chapters: sources,
    artifacts: [],
    documents: sources.map((chapter, index) => ({
      stale: false,
      video_reference_status: null,
      record: {
        id: projectId * 1000 + index,
        project_id: projectId,
        source_snapshot_id: snapshotId,
        source_chapter_id: chapter.source.source_chapter_id,
        kind: "adaptation_plan",
        version: 1,
        revision: 1,
        status: "draft",
        sha256: "a".repeat(64),
        inputs: { sources: [chapter.source], dependencies: [], video_reference: null },
        document: {
          schema_version: "adaptation_plan_v1",
          target,
          page_count: target === "manga" ? 1 : null,
          intention: "保留推门前的犹豫",
          decisions: [],
          units: [{
            id: "door",
            origin: "source",
            title: "走进旧书店",
            location: "旧书店门口",
            participants: [],
            dramatic_question: "门后是否有人",
            objective: "进入书店",
            obstacle: "门关着",
            entry_state: "门关闭",
            exit_state: "门打开",
            source_span_ids: [chapter.source_spans.at(-1).id],
            beats: [{
              id: "open",
              kind: "action",
              is_climax: false,
              origin: "source",
              importance: "required",
              action: "推开门",
              reaction: "停顿片刻",
              dialogue_intent: null,
              entry_state: "门关闭",
              exit_state: "门打开",
              source_span_ids: [chapter.source_spans.at(-1).id],
            }],
          }],
        },
      },
    })),
  };
}
