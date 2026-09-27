import type {
  ShootingCameraAngleV1,
  ShootingCameraMovementV1,
  ShootingCoverageV1,
  ShootingShotSizeV1,
  ShootingShotV2,
} from "../api/shootingScript";
import { formatNumber, type MessageKey } from "../i18n";

type Translator = (key: MessageKey) => string;

export function shootingShotDurationLabel(
  shot: Pick<ShootingShotV2, "planned_start_ms" | "planned_end_ms">,
  locale: Parameters<typeof formatNumber>[1],
  t: Translator,
): string {
  const start = shot.planned_start_ms;
  const end = shot.planned_end_ms;
  if (start == null || end == null || !Number.isFinite(start) || !Number.isFinite(end) || end <= start) {
    return t("shootingScript.editor.missingPlannedDuration");
  }
  return `${formatNumber((end - start) / 1000, locale, { maximumFractionDigits: 3 })} ${t("units.seconds")}`;
}

const SHOT_SIZE_LABELS: Record<ShootingShotSizeV1, MessageKey> = {
  extreme_wide: "shootingScript.editor.cameraExtremeWide",
  wide: "shootingScript.editor.cameraWide",
  medium_wide: "shootingScript.editor.cameraMediumWide",
  medium: "shootingScript.editor.cameraMedium",
  medium_close_up: "shootingScript.editor.cameraMediumCloseUp",
  close_up: "shootingScript.editor.cameraCloseUp",
  extreme_close_up: "shootingScript.editor.cameraExtremeCloseUp",
  insert: "shootingScript.editor.cameraInsert",
};

const CAMERA_ANGLE_LABELS: Record<ShootingCameraAngleV1, MessageKey> = {
  eye_level: "shootingScript.editor.angleEyeLevel",
  high: "shootingScript.editor.angleHigh",
  low: "shootingScript.editor.angleLow",
  overhead: "shootingScript.editor.angleOverhead",
  dutch: "shootingScript.editor.angleDutch",
  subjective: "shootingScript.editor.angleSubjective",
};

const CAMERA_MOVEMENT_LABELS: Record<ShootingCameraMovementV1, MessageKey> = {
  locked: "shootingScript.editor.cameraLocked",
  pan: "shootingScript.editor.cameraPan",
  tilt: "shootingScript.editor.cameraTilt",
  push_in: "shootingScript.editor.cameraPushIn",
  pull_out: "shootingScript.editor.cameraPullOut",
  track: "shootingScript.editor.cameraTrack",
  dolly: "shootingScript.editor.cameraDolly",
  crane: "shootingScript.editor.cameraCrane",
  handheld: "shootingScript.editor.cameraHandheld",
  steadicam: "shootingScript.editor.cameraSteadicam",
  whip_pan: "shootingScript.editor.cameraWhipPan",
};

const COVERAGE_LABELS: Record<ShootingCoverageV1, MessageKey> = {
  must_show: "shootingScript.editor.coverageMustShow",
  should_show: "shootingScript.editor.coverageShouldShow",
  optional: "shootingScript.editor.coverageOptional",
};

export const shootingShotSizeLabel = (value: ShootingShotSizeV1, t: Translator): string =>
  t(SHOT_SIZE_LABELS[value]);

export const shootingCameraAngleLabel = (value: ShootingCameraAngleV1, t: Translator): string =>
  t(CAMERA_ANGLE_LABELS[value]);

export const shootingCameraMovementLabel = (
  value: ShootingCameraMovementV1,
  t: Translator,
): string => t(CAMERA_MOVEMENT_LABELS[value]);

export const shootingCoverageLabel = (value: ShootingCoverageV1, t: Translator): string =>
  t(COVERAGE_LABELS[value]);
