import {
  VIDEO_SEGMENT_WORKFLOW_MODES,
  VIDEO_TYPE_CHAPTER,
  VIDEO_TYPE_REFERENCE_MEDIA,
  VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES,
  VIDEO_WORKFLOW_MODE_CHAT_VIDEO,
  VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN,
  VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA,
  VIDEO_WORKFLOW_MODE_STORYBOARD,
  type GeneratedVideo,
  type VideoSegmentWorkflowMode,
} from "../../../api/video";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import type { MessageTranslator } from "../../../i18n";
import { VIDEO_EXECUTION_CONTRACT_EPOCH } from "./videoTabConstants";

type AssemblyPushStatus = "pending" | "accepted" | "rejected";

export const assemblyPushMetadata = (
  video: GeneratedVideo,
): Record<string, unknown> | null =>
  asRecord(asRecord(video.metadata_json)?.assembly_push);

export const assemblyPushStatus = (
  video: GeneratedVideo,
): AssemblyPushStatus | null => {
  const value = assemblyPushMetadata(video)?.status;
  return value === "pending" || value === "accepted" || value === "rejected"
    ? value
    : null;
};

export const assemblyPushIndex = (video: GeneratedVideo): number | null => {
  const value = assemblyPushMetadata(video)?.push_index;
  return Number.isInteger(value) && Number(value) > 0 ? Number(value) : null;
};

export const assemblyPushSourceIds = (
  video: GeneratedVideo,
): number[] | null => {
  const value = assemblyPushMetadata(video)?.source_video_ids;
  if (!Array.isArray(value) || value.length === 0) return null;
  const ids = value.filter(
    (item): item is number => Number.isInteger(item) && Number(item) > 0,
  );
  return ids.length === value.length && new Set(ids).size === ids.length
    ? ids
    : null;
};

export const latestNonRejectedAssemblyPush = (
  pushes: readonly GeneratedVideo[],
): GeneratedVideo | null => {
  for (let index = pushes.length - 1; index >= 0; index -= 1) {
    const push = pushes[index];
    if (assemblyPushStatus(push) !== "rejected") return push;
  }
  return null;
};

export function videoWorkflowModeOfVideo(
  video: GeneratedVideo,
): VideoSegmentWorkflowMode | null {
  const raw = video.workflow_mode;
  if (
    typeof raw === "string" &&
    VIDEO_SEGMENT_WORKFLOW_MODES.includes(raw as VideoSegmentWorkflowMode)
  ) {
    return raw as VideoSegmentWorkflowMode;
  }
  return null;
}

const videoProviderRequestSnapshotFromRecord = (
  video: GeneratedVideo,
): Record<string, unknown> | null =>
  asRecord(asRecord(video.metadata_json)?.provider_request_snapshot);

const snapshotTextField = (
  snapshot: Record<string, unknown>,
  field: string,
): string | null => {
  const value = snapshot[field];
  return typeof value === "string" && value.trim() ? value.trim() : null;
};

const videoProviderRequestSnapshotBlockedReason = (
  video: GeneratedVideo,
  t: MessageTranslator,
): string | null => {
  const snapshot = videoProviderRequestSnapshotFromRecord(video);
  if (!snapshot) {
    return t("video.chapter.providerSnapshotMissing");
  }
  if (snapshot.schema_version !== 1) {
    return t("video.chapter.providerSnapshotVersionInvalid");
  }
  for (const field of ["key", "title", "kind", "state"]) {
    if (!snapshotTextField(snapshot, field)) {
      return t("video.chapter.providerSnapshotFieldMissing", { field });
    }
  }
  if (!asRecord(snapshot.request)) {
    return t("video.chapter.providerSnapshotRequestMissing");
  }
  return null;
};

export const videoProviderRequestPreviewBlockedReason = (
  video: GeneratedVideo,
  t: MessageTranslator,
): string | null => {
  const workflowMode = videoWorkflowModeOfVideo(video);
  if (workflowMode === VIDEO_WORKFLOW_MODE_CHAT_VIDEO) {
    return t("video.chapter.externalChatNoProvider");
  }
  if (
    video.video_type !== VIDEO_TYPE_CHAPTER &&
    video.video_type !== VIDEO_TYPE_REFERENCE_MEDIA
  ) {
    return t("video.chapter.trimmedNoProvider");
  }
  const snapshotBlockedReason =
    videoProviderRequestSnapshotBlockedReason(video, t);
  if (snapshotBlockedReason) {
    return snapshotBlockedReason;
  }
  if (workflowMode === VIDEO_WORKFLOW_MODE_REFERENCE_MEDIA) {
    if (video.video_type !== VIDEO_TYPE_REFERENCE_MEDIA) {
      return t("video.chapter.trimmedNoProvider");
    }
    return null;
  }
  if (
    workflowMode === VIDEO_WORKFLOW_MODE_BOUNDARY_FRAMES ||
    workflowMode === VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN ||
    workflowMode === VIDEO_WORKFLOW_MODE_STORYBOARD
  ) {
    if (video.video_type !== VIDEO_TYPE_CHAPTER) {
      return t("video.chapter.trimmedNoProvider");
    }
    const snapshot = videoProviderRequestSnapshotFromRecord(video);
    const identity = asRecord(snapshot?.video_execution_identity);
    if (identity?.contract_epoch !== VIDEO_EXECUTION_CONTRACT_EPOCH) {
      return t("video.chapter.staleExecutionIdentity");
    }
    return null;
  }
  return t("video.chapter.invalidGenerationMode");
};
