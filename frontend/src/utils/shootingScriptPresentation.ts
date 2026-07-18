import type {
  ShootingCameraAngle,
  ShootingCameraMovement,
  ShootingCoverage,
  ShootingShotSize,
} from "../api/shootingScript";

const SHOT_SIZE_LABELS: Record<ShootingShotSize, string> = {
  extreme_wide: "大远景",
  wide: "远景",
  medium_wide: "中远景",
  medium: "中景",
  medium_close_up: "中近景",
  close_up: "近景",
  extreme_close_up: "特写",
  insert: "插入镜头",
};

const CAMERA_ANGLE_LABELS: Record<ShootingCameraAngle, string> = {
  eye_level: "平视",
  high: "俯拍",
  low: "仰拍",
  overhead: "顶拍",
  dutch: "倾斜构图",
  subjective: "主观视角",
};

const CAMERA_MOVEMENT_LABELS: Record<ShootingCameraMovement, string> = {
  locked: "固定机位",
  pan: "横摇",
  tilt: "纵摇",
  push_in: "推近",
  pull_out: "拉远",
  track: "跟拍",
  dolly: "轨道移动",
  crane: "升降",
  handheld: "手持",
  steadicam: "稳定器跟拍",
  whip_pan: "甩摇",
};

const COVERAGE_LABELS: Record<ShootingCoverage, string> = {
  must_show: "必须呈现",
  should_show: "建议呈现",
  optional: "可选镜头",
};

export const shootingShotSizeLabel = (value: ShootingShotSize): string =>
  SHOT_SIZE_LABELS[value];

export const shootingCameraAngleLabel = (value: ShootingCameraAngle): string =>
  CAMERA_ANGLE_LABELS[value];

export const shootingCameraMovementLabel = (
  value: ShootingCameraMovement,
): string => CAMERA_MOVEMENT_LABELS[value];

export const shootingCoverageLabel = (value: ShootingCoverage): string =>
  COVERAGE_LABELS[value];
