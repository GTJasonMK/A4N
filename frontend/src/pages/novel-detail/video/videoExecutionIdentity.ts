import {
  VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN,
  type ChapterVideoPrompt,
  type GeneratedVideo,
} from "../../../api/video";
import { asJsonRecord as asRecord } from "../../../utils/jsonRecord";
import { isPositiveSafeInteger } from "../../../utils/numberRange";
import {
  VIDEO_EXECUTION_CONTRACT_EPOCH,
  VIDEO_FINAL_PROMPT_TYPE,
  VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE,
  isVideoFramePromptType,
} from "./videoTabConstants";

const VIDEO_EXECUTION_BASE_IDENTITY_FIELDS = [
  "source_content_sha256",
  "style_sha256",
  "duration_secs",
  "aspect_ratio",
] as const;

const VIDEO_EXECUTION_LINEAGE_BASE_IDENTITY_FIELDS = [
  "source_content_sha256",
  "style_sha256",
  "aspect_ratio",
] as const;

const VIDEO_EXECUTION_SEGMENT_IDENTITY_FIELDS = [
  "source_shot_id",
  "segment_index_in_shot",
  "segment_sha256",
] as const;

export function videoExecutionIdentityOfPrompt(
  prompt: ChapterVideoPrompt | null | undefined,
): Record<string, unknown> | null {
  const identity = asRecord(
    asRecord(prompt?.metadata_json)?.video_execution_identity,
  );
  if (identity?.contract_epoch !== VIDEO_EXECUTION_CONTRACT_EPOCH) return null;
  return identity;
}

const videoExecutionIdentityMatchesExpected = (
  actual: Record<string, unknown> | null,
  expected: Record<string, unknown> | null,
  fields: readonly string[],
): boolean =>
  Boolean(
    actual &&
    expected &&
    fields.every(
      (field) =>
        expected[field] !== undefined && actual[field] === expected[field],
    ),
  );

type SegmentIdentityIndex = Map<number, Record<string, unknown> | null>;

export interface VideoExecutionSegmentIdentityIndex {
  segments: unknown[];
  bySegmentNumber: SegmentIdentityIndex;
  byIdentity: Map<string, number>;
}

const segmentIdentityKey = (
  identity: Record<string, unknown> | null | undefined,
): string =>
  JSON.stringify(
    VIDEO_EXECUTION_SEGMENT_IDENTITY_FIELDS.map((field) => identity?.[field]),
  );

export function videoExecutionSegmentIdentityIndex(
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
): VideoExecutionSegmentIdentityIndex {
  const metadata = asRecord(segmentPlanPrompt?.metadata_json);
  const segments = Array.isArray(metadata?.segments) ? metadata.segments : [];
  const bySourceAndIndex = new Map<string, Map<number, Record<string, unknown>>>();
  const identities = videoExecutionIdentityOfPrompt(segmentPlanPrompt)?.segment_identities;
  if (Array.isArray(identities)) {
    for (const value of identities) {
      const identity = asRecord(value);
      const sourceShotId = identity?.source_shot_id;
      const segmentIndexInShot = identity?.segment_index_in_shot;
      if (
        typeof sourceShotId !== "string" ||
        typeof segmentIndexInShot !== "number" ||
        typeof identity?.segment_sha256 !== "string" ||
        identity.segment_sha256.length === 0
      ) {
        continue;
      }
      const byIndex = bySourceAndIndex.get(sourceShotId) ?? new Map();
      if (!byIndex.has(segmentIndexInShot)) byIndex.set(segmentIndexInShot, identity);
      bySourceAndIndex.set(sourceShotId, byIndex);
    }
  }

  const bySegmentNumber: SegmentIdentityIndex = new Map();
  const byIdentity = new Map<string, number>();
  for (const value of segments) {
    const segment = asRecord(value);
    const segmentNumber = segment?.segment_number;
    if (
      !isPositiveSafeInteger(segmentNumber) ||
      bySegmentNumber.has(segmentNumber)
    ) {
      continue;
    }
    const sourceShotId = segment?.source_shot_id;
    const segmentIndexInShot = segment?.segment_index_in_shot;
    if (
      typeof sourceShotId !== "string" ||
      !sourceShotId.trim() ||
      !isPositiveSafeInteger(segmentIndexInShot)
    ) {
      bySegmentNumber.set(segmentNumber, null);
      continue;
    }
    const expected =
      bySourceAndIndex.get(sourceShotId)?.get(segmentIndexInShot) ?? null;
    bySegmentNumber.set(segmentNumber, expected);
    if (expected) {
      const key = segmentIdentityKey(expected);
      if (!byIdentity.has(key)) byIdentity.set(key, segmentNumber);
    }
  }
  return { segments, bySegmentNumber, byIdentity };
}

const videoExecutionSegmentIdentityMatchesPlan = (
  actual: Record<string, unknown> | null,
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
  segmentNumber: number,
  fields: readonly string[],
  identityIndex?: VideoExecutionSegmentIdentityIndex,
): boolean => {
  return (
    videoExecutionSegmentNumberInPlanMatching(
      actual,
      segmentPlanPrompt,
      fields,
      identityIndex,
    ) === segmentNumber
  );
};

export const segmentPlanExecutionIdentityIsComplete = (
  prompt: ChapterVideoPrompt,
): boolean => {
  const { segments, bySegmentNumber } = videoExecutionSegmentIdentityIndex(prompt);
  const identities = videoExecutionIdentityOfPrompt(prompt)?.segment_identities;
  return (
    segments.length > 0 &&
    Array.isArray(identities) &&
    identities.length === segments.length &&
    segments.every((value) => {
      const segmentNumber = asRecord(value)?.segment_number;
      return (
        isPositiveSafeInteger(segmentNumber) &&
        (bySegmentNumber.get(segmentNumber) ?? null) != null
      );
    })
  );
};

const videoExecutionSegmentNumberInPlanMatching = (
  actual: Record<string, unknown> | null,
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
  fields: readonly string[],
  identityIndex?: VideoExecutionSegmentIdentityIndex,
): number | null => {
  const staleShotIds = asRecord(segmentPlanPrompt?.metadata_json)?.stale_source_shot_ids;
  if (Array.isArray(staleShotIds) && staleShotIds.includes(actual?.source_shot_id)) return null;
  const planIdentity = videoExecutionIdentityOfPrompt(segmentPlanPrompt);
  if (
    !videoExecutionIdentityMatchesExpected(actual, planIdentity, fields) ||
    actual?.segment_plan_prompt_id !== segmentPlanPrompt?.id
  ) {
    return null;
  }
  return (
    identityIndex ?? videoExecutionSegmentIdentityIndex(segmentPlanPrompt)
  ).byIdentity.get(segmentIdentityKey(actual)) ?? null;
};

export function videoExecutionSegmentNumberInPlan(
  actual: Record<string, unknown> | null,
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
  identityIndex?: VideoExecutionSegmentIdentityIndex,
): number | null {
  return videoExecutionSegmentNumberInPlanMatching(
    actual,
    segmentPlanPrompt,
    VIDEO_EXECUTION_BASE_IDENTITY_FIELDS,
    identityIndex,
  );
}

export function videoExecutionSegmentNumberInPlanByLineage(
  actual: Record<string, unknown> | null,
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
  identityIndex?: VideoExecutionSegmentIdentityIndex,
): number | null {
  return videoExecutionSegmentNumberInPlanMatching(
    actual,
    segmentPlanPrompt,
    VIDEO_EXECUTION_LINEAGE_BASE_IDENTITY_FIELDS,
    identityIndex,
  );
}

export function videoExecutionIdentityMatches(
  prompt: ChapterVideoPrompt | null | undefined,
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
): boolean {
  return videoExecutionIdentityMatchesExpected(
    videoExecutionIdentityOfPrompt(prompt),
    videoExecutionIdentityOfPrompt(segmentPlanPrompt),
    VIDEO_EXECUTION_BASE_IDENTITY_FIELDS,
  );
}

export function videoExecutionIdentityOfVideo(
  video: GeneratedVideo | null | undefined,
): Record<string, unknown> | null {
  const identity = asRecord(
    asRecord(video?.metadata_json)?.video_execution_identity,
  );
  return identity?.contract_epoch === VIDEO_EXECUTION_CONTRACT_EPOCH
    ? identity
    : null;
}

export function videoExecutionSegmentIdentityMatchesPrompt(
  prompt: ChapterVideoPrompt | null | undefined,
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
  segmentNumber: number,
  recipeFingerprint: string | null | undefined,
  identityIndex?: VideoExecutionSegmentIdentityIndex,
): boolean {
  const identity = videoExecutionIdentityOfPrompt(prompt);
  return (
    Boolean(recipeFingerprint) &&
    videoExecutionSegmentIdentityMatchesPlan(
      identity,
      segmentPlanPrompt,
      segmentNumber,
      prompt?.prompt_type === VIDEO_FINAL_PROMPT_TYPE
        ? VIDEO_EXECUTION_LINEAGE_BASE_IDENTITY_FIELDS
        : VIDEO_EXECUTION_BASE_IDENTITY_FIELDS,
      identityIndex,
    ) &&
    identity?.segment_recipe_fingerprint === recipeFingerprint
  );
}

export function videoExecutionSegmentIdentityMatchesVideo(
  video: GeneratedVideo | null | undefined,
  segmentPlanPrompt: ChapterVideoPrompt | null | undefined,
  segmentNumber: number,
  recipeFingerprint: string | null | undefined,
  identityIndex?: VideoExecutionSegmentIdentityIndex,
): boolean {
  const identity = videoExecutionIdentityOfVideo(video);
  return (
    Boolean(recipeFingerprint) &&
    videoExecutionSegmentIdentityMatchesPlan(
      identity,
      segmentPlanPrompt,
      segmentNumber,
      VIDEO_EXECUTION_LINEAGE_BASE_IDENTITY_FIELDS,
      identityIndex,
    ) &&
    identity?.segment_recipe_fingerprint === recipeFingerprint
  );
}

const promptIsSharedVideoUpstream = (prompt: ChapterVideoPrompt): boolean =>
  prompt.workflow_mode === VIDEO_WORKFLOW_MODE_SHARED_SEGMENT_PLAN &&
  prompt.prompt_type === VIDEO_FRAME_SEGMENT_PLAN_PROMPT_TYPE;

export function videoExecutionPromptsShareGenerationChain(
  left: ChapterVideoPrompt,
  right: ChapterVideoPrompt,
): boolean {
  if (
    left.project_id !== right.project_id ||
    left.chapter_id !== right.chapter_id ||
    left.style !== right.style ||
    left.aspect_ratio !== right.aspect_ratio
  ) {
    return false;
  }
  const leftShared = promptIsSharedVideoUpstream(left);
  const rightShared = promptIsSharedVideoUpstream(right);
  if (
    !leftShared &&
    !rightShared &&
    left.workflow_mode !== right.workflow_mode
  ) {
    return false;
  }
  if (!videoExecutionIdentityMatches(left, right)) return false;
  const leftIdentity = videoExecutionIdentityOfPrompt(left);
  const rightIdentity = videoExecutionIdentityOfPrompt(right);
  if (!leftIdentity || !rightIdentity) return false;
  const identityExistsInPlan = (
    planIdentity: Record<string, unknown>,
    segmentIdentity: Record<string, unknown>,
  ): boolean =>
    Array.isArray(planIdentity.segment_identities) &&
    planIdentity.segment_identities
      .map(asRecord)
      .some(
        (candidate) =>
          candidate != null &&
          VIDEO_EXECUTION_SEGMENT_IDENTITY_FIELDS.every(
            (field) => candidate[field] === segmentIdentity[field],
          ),
      );
  if (leftShared) {
    return (
      rightIdentity.segment_plan_prompt_id === left.id &&
      identityExistsInPlan(leftIdentity, rightIdentity)
    );
  }
  if (rightShared) {
    return (
      leftIdentity.segment_plan_prompt_id === right.id &&
      identityExistsInPlan(rightIdentity, leftIdentity)
    );
  }
  const videoAndFramePromptPair =
    (left.prompt_type === VIDEO_FINAL_PROMPT_TYPE &&
      isVideoFramePromptType(right.prompt_type)) ||
    (right.prompt_type === VIDEO_FINAL_PROMPT_TYPE &&
      isVideoFramePromptType(left.prompt_type));
  return (
    leftIdentity.segment_plan_prompt_id != null &&
    leftIdentity.segment_plan_prompt_id ===
      rightIdentity.segment_plan_prompt_id &&
    VIDEO_EXECUTION_SEGMENT_IDENTITY_FIELDS.every(
      (field) => leftIdentity[field] === rightIdentity[field],
    ) &&
    (videoAndFramePromptPair ||
      leftIdentity.segment_recipe_fingerprint ===
        rightIdentity.segment_recipe_fingerprint)
  );
}
