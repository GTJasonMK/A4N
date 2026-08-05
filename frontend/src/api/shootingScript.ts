import { apiInvoke } from "./index";
import { readCurrentUserId } from "./authStorage";
import type {
  ChapterShootingScriptUnitProgressView as ApiChapterShootingScriptUnitProgressView,
  ChapterShootingScriptView as ApiChapterShootingScriptView,
  EditedChapterShootingScriptView as ApiEditedChapterShootingScriptView,
} from "./generated/host";

const currentUserId = (): number => {
  const id = readCurrentUserId();
  if (id == null) throw new Error("未登录，无法访问拍摄剧本");
  return id;
};

type ShootingViewpointMode =
  | "direct_subjective"
  | "modified_subjective"
  | "limited_third_person"
  | "omniscient_objective";
type ShootingContinuityIntent =
  | "transparent"
  | "deliberate_disorientation"
  | "open_ambiguity";
export type ShootingCoverage = "must_show" | "should_show" | "optional";
export type ShootingShotSize =
  | "extreme_wide" | "wide" | "medium_wide" | "medium" | "medium_close_up"
  | "close_up" | "extreme_close_up" | "insert";
export type ShootingCameraAngle = "eye_level" | "high" | "low" | "overhead" | "dutch" | "subjective";
type ShootingCameraHeight = "ground" | "low" | "eye" | "high" | "overhead";
export type ShootingCameraMovement =
  | "locked" | "pan" | "tilt" | "push_in" | "pull_out" | "track"
  | "dolly" | "crane" | "handheld" | "steadicam" | "whip_pan";

export interface ShootingPerformanceBeat {
  beat_index: number;
  beat_id: string;
  source_span_ids: string[];
  actor: string | null;
  objective: string;
  obstacle: string;
  visible_action: string;
  reaction: string | null;
  expression: string | null;
  gaze: string | null;
  pose_or_movement: string | null;
  dialogue_intent: string | null;
  audience_question: string;
  answer_or_delay: string;
  knowledge_delta: string[];
  relationship_delta: string | null;
  spatial_change: string | null;
  entry_state: string;
  exit_state: string;
  required: boolean;
  estimated_seconds: number;
}

export interface ShootingShot {
  shot_index: number;
  shot_id: string;
  title: string;
  source_span_ids: string[];
  coverage: ShootingCoverage;
  narrative_purpose: string;
  subject: string;
  shot_size: ShootingShotSize;
  camera_angle: ShootingCameraAngle;
  camera_height: ShootingCameraHeight;
  lens_intent: string;
  camera_movement: ShootingCameraMovement;
  composition: string;
  spatial_anchors: string[];
  blocking: { start: string; movement: string; end: string };
  performance_beats: ShootingPerformanceBeat[];
  dialogue_cues: Array<{ beat_id: string; speaker: string; line_or_intent: string; listener_reaction: string | null }>;
  sound: { dialogue: string[]; environment: string[]; effects: string[]; music: string[]; silence: string[] };
  camera_start_state: string;
  camera_end_state: string;
  start_state: string;
  end_state: string;
  transition_in: string;
  transition_out: string;
  continuity_requirements: string[];
  estimated_natural_seconds: number;
}

export interface ShootingScene {
  scene_index: number;
  scene_identity: string;
  sequence_index: number;
  narrative_unit_index: number;
  timeline_position: number;
  intercut_group_id: string | null;
  editing_mode: "continuity" | "intercut" | "montage" | "flashback" | "parallel_action" | "sound_bridge";
  transition_contracts: Array<{
    to_narrative_unit_index: number;
    transition_type: string;
    preserved_action: string | null;
    preserved_sound: string | null;
    preserved_knowledge: string[];
    required_cut_point: string | null;
  }>;
  title: string;
  slugline: string;
  location: string | null;
  time_of_day: string | null;
  participants: string[];
  focal_character: string | null;
  source_span_ids: string[];
  dramatic_question: string;
  scene_objective: string;
  scene_obstacle: string;
  relationship_before: string;
  relationship_after: string;
  audience_knowledge_before: string[];
  audience_knowledge_after: string[];
  withheld_information: string[];
  viewpoint: { mode: ShootingViewpointMode; anchor_character: string | null; information_strategy: string };
  spatial_continuity: {
    continuity_intent: ShootingContinuityIntent;
    primary_axis: string | null;
    screen_direction: string | null;
    reorientation_strategy: string | null;
  };
  sound_strategy: string;
  entry_state: string;
  exit_state: string;
  shots: ShootingShot[];
}

export interface ChapterShootingScript {
  schema_version: "chapter_shooting_script_v1";
  directing_statement: {
    central_dramatic_question: string;
    protagonist_throughline: string;
    audience_experience: string;
    viewpoint_strategy: string;
    visual_strategy: string;
    spatial_strategy: string;
    sound_strategy: string;
    rhythm_strategy: string;
  };
  scenes: ShootingScene[];
}

interface ChapterShootingScriptRecord {
  id: number;
  root_project_id: number;
  source_chapter_id: number;
  source_revision_key: string;
  source_content_sha256: string;
  contract_version: string;
  revision: number;
  script_sha256: string;
  workflow_run_id: number | null;
  created_at: string;
  updated_at: string;
}

export type ChapterShootingScriptView = Omit<
  ApiChapterShootingScriptView,
  "record" | "script"
> & {
  record: ChapterShootingScriptRecord;
  script: ChapterShootingScript;
};

export type ChapterShootingScriptUnitProgressView = Omit<
  ApiChapterShootingScriptUnitProgressView,
  "script" | "shooting_script"
> & {
  script: ChapterShootingScript;
  shooting_script: ChapterShootingScriptView | null;
};

type EditedChapterShootingScriptView = Omit<
  ApiEditedChapterShootingScriptView,
  "shooting_script"
> & {
  shooting_script: ChapterShootingScriptView;
};

export const getChapterShootingScript = (projectId: number, sourceChapterId: number) =>
  apiInvoke<ChapterShootingScriptView | null>("get_chapter_shooting_script", {
    userId: currentUserId(),
    input: { project_id: projectId, source_chapter_id: sourceChapterId },
  });

export const getChapterShootingScriptUnitProgress = (
  projectId: number,
  sourceChapterId: number,
) => apiInvoke<ChapterShootingScriptUnitProgressView | null>(
  "get_chapter_shooting_script_unit_progress",
  {
    userId: currentUserId(),
    input: { project_id: projectId, source_chapter_id: sourceChapterId },
  },
);

export const generateChapterShootingScript = (
  projectId: number,
  sourceChapterId: number,
  taskId?: string | null,
) =>
  apiInvoke<ChapterShootingScriptView>("generate_chapter_shooting_script", {
    userId: currentUserId(),
    input: {
      project_id: projectId,
      source_chapter_id: sourceChapterId,
    },
    taskId: taskId ?? null,
  });

export const generateNextChapterShootingScriptUnit = (
  projectId: number,
  sourceChapterId: number,
  taskId?: string | null,
) => apiInvoke<ChapterShootingScriptUnitProgressView>(
  "generate_next_chapter_shooting_script_unit",
  {
    userId: currentUserId(),
    input: {
      project_id: projectId,
      source_chapter_id: sourceChapterId,
    },
    taskId: taskId ?? null,
  },
);

export const editChapterShootingScript = (
  projectId: number,
  sourceChapterId: number,
  view: ChapterShootingScriptView,
  script: ChapterShootingScript,
) => apiInvoke<EditedChapterShootingScriptView>(
  "edit_chapter_shooting_script",
  {
    userId: currentUserId(),
    input: {
      project_id: projectId,
      source_chapter_id: sourceChapterId,
      record_id: view.record.id,
      expected_revision: view.record.revision,
      script,
    },
  },
);
