import type { ProgressEvent } from "../api/generated/generationContracts";
import type { ProgressMeta } from "../api/progress";

/**
 * 把 generation 传输合同的 ProgressEvent 映射为展示用 ProgressMeta。
 *
 * phase_kind 与 Host 展示合同均来自 a4n-domain::progress；前端只转换字段命名，
 * 不再维护第二套 stage→phase 或阶段文案映射。
 */

export function generationProgressMeta(
  event: ProgressEvent,
  taskKind: string,
): ProgressMeta {
  return {
    taskKind,
    stageCode: event.stage_code,
    // Transport events carry a provider-locale message; the UI resolves stageCode/phaseKind.
    stageLabel: event.stage_code,
    phaseKind: event.phase_kind,
    detail: null,
    unitLabel: event.unit_label ?? null,
    itemIndex: event.item_index ?? null,
    itemTotal: event.item_total ?? null,
    parentStageCode: null,
    severity: event.severity,
    estimated: event.estimated,
    persistent: false,
  };
}
