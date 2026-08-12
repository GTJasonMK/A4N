import type { ProgressEvent } from "../api/generated/generationContracts";
import type { ProgressMeta, ProgressDisplayPhase } from "../api/progress";

/**
 * 把 generation 传输合同的 ProgressEvent 映射为展示用 ProgressMeta。
 *
 * 后端有两层进度语义，服务于不同抽象层级：
 *  - 传输层（a4n-service-protocol）：粗粒度 phase_kind + 细粒度 stage_code，
 *    二者映射由合同强制校验（events/generation.rs 的 public_progress_phase）
 *  - 展示层（a4n-domain::progress）：细粒度 phase_kind，供 ETA / 语气判定
 *
 * 传输层的 `provider` 覆盖了「连接中 / 等待首 token / 正在输出」三种状态，
 * 展示层需要区分它们（等待期不该显示线性 ETA）。区分信息在 stage_code 里，
 * 所以本表以 stage_code 为主键，phase_kind 仅作未知 stage 的回退。
 */

/** 传输层 stage_code → 展示层 phase kind。键必须覆盖后端 public_progress_phase 的全部公共 stage。 */
const STAGE_PHASE: Record<string, ProgressDisplayPhase> = {
  reserved: "queue",
  provider_queue: "queue",
  prepare: "prepare",
  context: "context",
  provider_connect: "model_connect",
  provider_wait: "model_wait",
  provider_run: "model_wait",
  provider_stream: "model_stream",
  repair: "structured_output",
  parse: "parse",
  validate: "validate",
  media_validate: "validate",
  save: "save",
  transfer: "transfer",
  cleanup: "cleanup",
  complete: "complete",
};

/** 传输层粗粒度 phase_kind → 展示层回退值（stage_code 未登记时使用）。 */
const PHASE_FALLBACK: Record<string, ProgressDisplayPhase> = {
  queue: "queue",
  prepare: "prepare",
  context: "context",
  provider: "model_wait",
  stream: "model_stream",
  parse: "parse",
  validate: "validate",
  persist: "save",
  transfer: "transfer",
  cleanup: "cleanup",
  complete: "complete",
};

/** 展示用中文阶段名。未登记的 stage 回退到事件自带 message。 */
const STAGE_LABEL: Record<string, string> = {
  reserved: "排队中",
  provider_queue: "排队中",
  prepare: "准备中",
  context: "组装上下文",
  provider_connect: "连接模型",
  provider_wait: "等待模型响应",
  provider_run: "模型处理中",
  provider_stream: "模型输出中",
  repair: "结构化修复",
  parse: "解析结果",
  validate: "校验结果",
  media_validate: "校验媒体",
  save: "保存中",
  transfer: "传输中",
  cleanup: "清理中",
  complete: "已完成",
};

const displayPhaseForStage = (
  stageCode: string,
  phaseKind: string,
): ProgressDisplayPhase =>
  STAGE_PHASE[stageCode] ?? PHASE_FALLBACK[phaseKind] ?? "prepare";

const stageLabelFor = (stageCode: string, message: string): string =>
  STAGE_LABEL[stageCode] ?? message;

/**
 * 传输层事件 → 展示层 meta。
 *
 * `taskKind` 由调用方提供（generation handle 不携带该信息）；缺省用 stage_code
 * 作为占位，仅影响 meta.taskKind 的可读性，不参与展示判定。
 */
export function generationProgressMeta(
  event: ProgressEvent,
  taskKind = "generation",
): ProgressMeta {
  return {
    taskKind,
    stageCode: event.stage_code,
    stageLabel: stageLabelFor(event.stage_code, event.message),
    phaseKind: displayPhaseForStage(event.stage_code, event.phase_kind),
    detail: event.message,
    unitLabel: event.unit_label ?? null,
    itemIndex: event.item_index ?? null,
    itemTotal: event.item_total ?? null,
    parentStageCode: null,
    severity: event.severity,
    estimated: event.estimated,
    persistent: false,
  };
}
