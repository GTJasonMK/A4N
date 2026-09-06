import type { VisualEntityStateV1 } from "../../../api/chapterVisualPlan";

export type EntityResizeHandle = "nw" | "ne" | "se" | "sw";
export type EntityControlAnchor = EntityResizeHandle | "center" | "rotate";

export interface EntityTransform {
  bounds: VisualEntityStateV1["bounds"];
  rotation: number;
}

export interface EntityFrameSize {
  width: number;
  height: number;
}

export interface EntityControlPosition {
  left: number;
  top: number;
}

export const MIN_ENTITY_SIZE = 0.04;
export const MAX_RENDER_Z_INDEX = 1000;
export const ENTITY_RENDER_Z_INDEX_OFFSET = 10;
export const ENTITY_CONTROL_LAYER_Z_INDEX = ENTITY_RENDER_Z_INDEX_OFFSET + MAX_RENDER_Z_INDEX + 1;
export const ENTITY_CONTROL_SIZE = 44;
export const ENTITY_ROTATE_CONTROL_EXTENT = 26;

export const clampFrameValue = (value: number, minimum: number, maximum: number) =>
  Math.max(minimum, Math.min(maximum, value));

export const finiteOr = (value: unknown, fallback: number) =>
  typeof value === "number" && Number.isFinite(value) ? value : fallback;

export const renderOpacity = (value: unknown) => clampFrameValue(finiteOr(value, 1), 0, 1);

export const renderRotation = (value: unknown) => {
  const rotation = finiteOr(value, 0) % 360;
  return Object.is(rotation, -0) ? 0 : rotation;
};

export const renderZIndex = (value: unknown) =>
  Math.round(clampFrameValue(finiteOr(value, 0), -MAX_RENDER_Z_INDEX, MAX_RENDER_Z_INDEX));

export const defaultRenderBounds: VisualEntityStateV1["bounds"] = {
  x: 0,
  y: 0,
  width: MIN_ENTITY_SIZE,
  height: MIN_ENTITY_SIZE,
};

const MIN_CONTROL_BOUNDS = 0.0001;

export const normalizeEntityBounds = (
  bounds: VisualEntityStateV1["bounds"],
  fallback: VisualEntityStateV1["bounds"] = defaultRenderBounds,
  minimumSize = MIN_ENTITY_SIZE,
): VisualEntityStateV1["bounds"] => {
  const minimum = clampFrameValue(finiteOr(minimumSize, MIN_ENTITY_SIZE), MIN_CONTROL_BOUNDS, 1);
  const fallbackWidth = clampFrameValue(finiteOr(fallback?.width, minimum), minimum, 1);
  const fallbackHeight = clampFrameValue(finiteOr(fallback?.height, minimum), minimum, 1);
  const source = bounds && typeof bounds === "object" ? bounds : fallback;
  const width = clampFrameValue(finiteOr(source?.width, fallbackWidth), minimum, 1);
  const height = clampFrameValue(finiteOr(source?.height, fallbackHeight), minimum, 1);
  const xCandidate = finiteOr(source?.x, finiteOr(fallback?.x, 0));
  const yCandidate = finiteOr(source?.y, finiteOr(fallback?.y, 0));
  return {
    x: clampFrameValue(xCandidate, 0, 1 - width),
    y: clampFrameValue(yCandidate, 0, 1 - height),
    width,
    height,
  };
};

export const measuredEntityBounds = (
  transform: EntityTransform,
  measured: { left: number; top: number; width: number; height: number },
  frame: EntityFrameSize,
): VisualEntityStateV1["bounds"] => {
  const frameWidth = Math.max(1, finiteOr(frame.width, 1));
  const frameHeight = Math.max(1, finiteOr(frame.height, 1));
  const fallback = normalizeEntityBounds(transform.bounds, defaultRenderBounds, MIN_CONTROL_BOUNDS);
  const width = clampFrameValue(
    finiteOr(measured.width, fallback.width * frameWidth) / frameWidth,
    MIN_CONTROL_BOUNDS,
    1,
  );
  const height = clampFrameValue(
    finiteOr(measured.height, fallback.height * frameHeight) / frameHeight,
    MIN_CONTROL_BOUNDS,
    1,
  );
  return normalizeEntityBounds({
    x: finiteOr(transform.bounds.x, fallback.x) + finiteOr(measured.left, 0) / frameWidth,
    y: finiteOr(transform.bounds.y, fallback.y) + finiteOr(measured.top, 0) / frameHeight,
    width,
    height,
  }, fallback, MIN_CONTROL_BOUNDS);
};

export const rotatedBoundsSize = (
  bounds: VisualEntityStateV1["bounds"],
  rotation: number,
  frame: EntityFrameSize = { width: 1, height: 1 },
) => {
  const radians = renderRotation(rotation) * Math.PI / 180;
  const cos = Math.abs(Math.cos(radians));
  const sin = Math.abs(Math.sin(radians));
  const frameWidth = Math.max(1, finiteOr(frame.width, 1));
  const frameHeight = Math.max(1, finiteOr(frame.height, 1));
  return {
    width: (cos * bounds.width * frameWidth + sin * bounds.height * frameHeight) / frameWidth,
    height: (sin * bounds.width * frameWidth + cos * bounds.height * frameHeight) / frameHeight,
  };
};

export const fitEntityBoundsToFrame = (
  bounds: VisualEntityStateV1["bounds"],
  rotation: number,
  frame: EntityFrameSize = { width: 1, height: 1 },
): VisualEntityStateV1["bounds"] => {
  const normalized = normalizeEntityBounds(bounds);
  const rotated = rotatedBoundsSize(normalized, rotation, frame);
  const scale = Math.min(
    1,
    rotated.width > 0 ? 1 / rotated.width : 1,
    rotated.height > 0 ? 1 / rotated.height : 1,
  );
  const width = normalized.width * scale;
  const height = normalized.height * scale;
  const fittedRotated = rotatedBoundsSize({ ...normalized, width, height }, rotation, frame);
  const centerX = clampFrameValue(
    normalized.x + normalized.width / 2,
    fittedRotated.width / 2,
    1 - fittedRotated.width / 2,
  );
  const centerY = clampFrameValue(
    normalized.y + normalized.height / 2,
    fittedRotated.height / 2,
    1 - fittedRotated.height / 2,
  );
  return {
    x: centerX - width / 2,
    y: centerY - height / 2,
    width,
    height,
  };
};

export const normalizeEntityTransform = (
  bounds: VisualEntityStateV1["bounds"],
  rotation: number,
  fallback: VisualEntityStateV1["bounds"] = defaultRenderBounds,
  frame: EntityFrameSize = { width: 1, height: 1 },
): EntityTransform => {
  const nextRotation = renderRotation(rotation);
  return {
    bounds: fitEntityBoundsToFrame(normalizeEntityBounds(bounds, fallback), nextRotation, frame),
    rotation: nextRotation,
  };
};

export const renderedEntityBounds = (
  transform: EntityTransform,
  frame: EntityFrameSize,
): VisualEntityStateV1["bounds"] => {
  const rotated = rotatedBoundsSize(transform.bounds, transform.rotation, frame);
  const width = clampFrameValue(rotated.width, 0, 1);
  const height = clampFrameValue(rotated.height, 0, 1);
  const centerX = transform.bounds.x + transform.bounds.width / 2;
  const centerY = transform.bounds.y + transform.bounds.height / 2;
  return {
    x: clampFrameValue(centerX - width / 2, 0, 1 - width),
    y: clampFrameValue(centerY - height / 2, 0, 1 - height),
    width,
    height,
  };
};

export const entitySelectionTransform = (
  transform: EntityTransform,
): EntityTransform => {
  return {
    bounds: normalizeEntityBounds(transform.bounds, defaultRenderBounds, MIN_CONTROL_BOUNDS),
    rotation: renderRotation(transform.rotation),
  };
};

export const resizeEntitySelectionByScreenDelta = (
  transform: EntityTransform,
  handle: EntityResizeHandle,
  deltaX: number,
  deltaY: number,
  frame: EntityFrameSize,
): EntityTransform => resizeEntityTransformByScreenDelta(
  entitySelectionTransform(transform),
  handle,
  deltaX,
  deltaY,
  frame,
);

const constrainControlCenter = (value: number, frameSize: number, extent: number) => {
  if (frameSize <= extent * 2) return frameSize / 2;
  return clampFrameValue(value, extent, frameSize - extent);
};

export const entityControlPosition = (
  transform: EntityTransform,
  anchor: EntityControlAnchor,
  frame: EntityFrameSize,
  controlSize = ENTITY_CONTROL_SIZE,
): EntityControlPosition => {
  const bounds = normalizeEntityBounds(transform.bounds, defaultRenderBounds, MIN_CONTROL_BOUNDS);
  const frameWidth = Math.max(1, finiteOr(frame.width, 1));
  const frameHeight = Math.max(1, finiteOr(frame.height, 1));
  const radians = renderRotation(transform.rotation) * Math.PI / 180;
  const cos = Math.cos(radians);
  const sin = Math.sin(radians);
  const rotatedControlSize = controlSize * (Math.abs(cos) + Math.abs(sin));
  const framePosition = entityControlFramePosition(transform, anchor, frame, rotatedControlSize);
  const centerX = (bounds.x + bounds.width / 2) * frameWidth;
  const centerY = (bounds.y + bounds.height / 2) * frameHeight;
  const screenOffsetX = framePosition.left - centerX;
  const screenOffsetY = framePosition.top - centerY;
  const adjustedLocalX = screenOffsetX * cos + screenOffsetY * sin;
  const adjustedLocalY = -screenOffsetX * sin + screenOffsetY * cos;

  return {
    left: bounds.width * frameWidth / 2 + adjustedLocalX,
    top: bounds.height * frameHeight / 2 + adjustedLocalY,
  };
};

export const entityControlFramePosition = (
  transform: EntityTransform,
  anchor: EntityControlAnchor,
  frame: EntityFrameSize,
  controlSize = ENTITY_CONTROL_SIZE,
): EntityControlPosition => {
  const bounds = normalizeEntityBounds(transform.bounds, defaultRenderBounds, MIN_CONTROL_BOUNDS);
  const frameWidth = Math.max(1, finiteOr(frame.width, 1));
  const frameHeight = Math.max(1, finiteOr(frame.height, 1));
  const radians = renderRotation(transform.rotation) * Math.PI / 180;
  const halfSize = Math.max(1, finiteOr(controlSize, ENTITY_CONTROL_SIZE)) / 2;
  const rotateOffset = Math.max(ENTITY_ROTATE_CONTROL_EXTENT, halfSize + 4);
  const centerX = (bounds.x + bounds.width / 2) * frameWidth;
  const centerY = (bounds.y + bounds.height / 2) * frameHeight;
  const localX = anchor === "center"
    ? 0
    : anchor === "rotate"
      ? 0
      : (anchor.includes("w") ? -bounds.width / 2 : bounds.width / 2) * frameWidth;
  const localY = anchor === "center"
    ? 0
    : anchor === "rotate"
      ? -bounds.height / 2 * frameHeight - rotateOffset
      : (anchor.includes("n") ? -bounds.height / 2 : bounds.height / 2) * frameHeight;
  const pointX = centerX + localX * Math.cos(radians) - localY * Math.sin(radians);
  const pointY = centerY + localX * Math.sin(radians) + localY * Math.cos(radians);
  const horizontalExtent = halfSize;
  const verticalExtent = anchor === "rotate" ? Math.max(halfSize, ENTITY_ROTATE_CONTROL_EXTENT) : halfSize;
  return {
    left: constrainControlCenter(pointX, frameWidth, horizontalExtent),
    top: constrainControlCenter(pointY, frameHeight, verticalExtent),
  };
};

export const resizeEntityTransformByScreenDelta = (
  transform: EntityTransform,
  handle: EntityResizeHandle,
  deltaX: number,
  deltaY: number,
  frame: EntityFrameSize,
): EntityTransform => {
  const initial = normalizeEntityBounds(transform.bounds);
  const radians = renderRotation(transform.rotation) * Math.PI / 180;
  const cos = Math.cos(radians);
  const sin = Math.sin(radians);
  const frameWidth = Math.max(1, finiteOr(frame.width, 1));
  const frameHeight = Math.max(1, finiteOr(frame.height, 1));
  const localDx = (finiteOr(deltaX, 0) * cos + finiteOr(deltaY, 0) * sin) / frameWidth;
  const localDy = (-finiteOr(deltaX, 0) * sin + finiteOr(deltaY, 0) * cos) / frameHeight;
  let left = initial.x;
  let top = initial.y;
  let right = initial.x + initial.width;
  let bottom = initial.y + initial.height;

  if (handle.includes("w")) left = clampFrameValue(initial.x + localDx, 0, right - MIN_ENTITY_SIZE);
  if (handle.includes("e")) right = clampFrameValue(initial.x + initial.width + localDx, left + MIN_ENTITY_SIZE, 1);
  if (handle.includes("n")) top = clampFrameValue(initial.y + localDy, 0, bottom - MIN_ENTITY_SIZE);
  if (handle.includes("s")) bottom = clampFrameValue(initial.y + initial.height + localDy, top + MIN_ENTITY_SIZE, 1);

  return normalizeEntityTransform(
    { x: left, y: top, width: right - left, height: bottom - top },
    transform.rotation,
    defaultRenderBounds,
    frame,
  );
};
