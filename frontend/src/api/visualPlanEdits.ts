import { apiInvoke, authenticatedWebFetch } from ".";
import type { ChapterVisualPlanReadResult, ChapterVisualPlanView } from "./generated/host";
import { shouldUseHttpBackend } from "../utils/runtimeMode";
import { isPositiveSafeInteger } from "../utils/numberRange";
import { readSseStream } from "../utils/sseParser";

export class VisualPlanInvalidatedError extends Error {}
// Editor receipts carry a document and inverse field values, unlike small
// generation events. Keep a domain-specific bound without weakening other streams.
const MAX_EDITOR_EVENT_BYTES = 128 * 1024 * 1024;

export async function subscribeVisualPlanEdits(
  projectId: number, chapterId: number, afterSequence: number, sourceHash: string, recordId: number | null,
  onEdit: (saved: ChapterVisualPlanView) => void, signal: AbortSignal,
): Promise<void> {
  let cursor = afterSequence;
  const receive = (saved: ChapterVisualPlanView) => {
    const receipt = saved.edit_receipt;
    if (!receipt || receipt.project_id !== projectId || receipt.source_chapter_id !== chapterId
      || !isPositiveSafeInteger(receipt.sequence)) {
      throw new Error("编辑回执身份或顺序无效");
    }
    if (receipt.sequence <= cursor) return;
    cursor = receipt.sequence;
    recordId = receipt.after.record_id;
    onEdit(saved);
  };
  if (!shouldUseHttpBackend()) {
    while (!signal.aborted) {
      const result = await apiInvoke<ChapterVisualPlanReadResult>("read_chapter_visual_plan", {
        input: { project_id: projectId, source_chapter_id: chapterId, view: { kind: "edits", after_sequence: cursor } },
      });
      if (signal.aborted) return;
      if (result.projection.kind !== "edits") throw new Error("编辑回执协议不兼容");
      if (result.source_hash !== sourceHash || (recordId != null && result.projection.items.length === 0 && result.resource?.id !== String(recordId))) {
        throw new VisualPlanInvalidatedError("编排来源或保存记录已变化，请保留本地草稿并重新读取");
      }
      result.projection.items.forEach(receive);
      await new Promise<void>((resolve) => {
        const finish = () => { clearTimeout(timer); signal.removeEventListener("abort", finish); resolve(); };
        const timer = setTimeout(finish, 500);
        signal.addEventListener("abort", finish, { once: true });
      });
    }
    return;
  }
  const params = new URLSearchParams({ project_id: String(projectId), source_chapter_id: String(chapterId), after_sequence: String(cursor) });
  params.set("expected_source_hash", sourceHash);
  if (recordId != null) params.set("expected_record_id", String(recordId));
  const response = await authenticatedWebFetch(`/visual-plan-edits?${params}`, { signal, headers: { Accept: "text/event-stream" } });
  if ([403, 404, 409].includes(response.status)) throw new VisualPlanInvalidatedError("编排来源、保存记录或访问权限已变化，请保留本地草稿并重新读取");
  if (!response.ok || !response.body) throw new Error(`编辑同步连接失败（${response.status}）`);
  await readSseStream(response.body, (frame) => {
    if (frame.event !== "visual_plan_edit") throw new Error("编辑回执事件不兼容");
    const saved = JSON.parse(frame.data) as ChapterVisualPlanView;
    if (frame.id !== String(saved.edit_receipt?.sequence)) throw new Error("编辑回执游标不一致");
    receive(saved);
  }, signal, MAX_EDITOR_EVENT_BYTES);
}
