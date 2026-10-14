import type { ChapterVisualPlanV3 } from "../../src/api/chapterVisualPlan";
import { pointsInsideSegment, segmentAt, segmentTimes } from "../../src/pages/novel-detail/visual-plan/timeline";

const pointCount = Number(new URLSearchParams(location.search).get("points") ?? 1000);
const points = Array.from({ length: pointCount }, (_, index) => ({
  id: String(index),
  at_ms: index * 1000,
  label: "Point " + index,
  description: "",
  image_id: null,
  reference_image_ids: [],
}));
const targetIndex = Math.floor(pointCount / 2);
const plan = {
  schema_version: "chapter_visual_plan_v3",
  frame: { width: 1920, height: 1080 },
  points,
  segments: Array.from({ length: pointCount }, (_, index) => ({
    id: index === targetIndex ? "target" : "segment-" + index,
    title: "Segment " + index,
    start_point_id: String(index),
    end_point_id: String(Math.min(index + 1, pointCount - 1)),
    prompt: "",
    scene_binding: null,
    use_end_point_image: false,
    ...(index === targetIndex
      ? {
          start_point_id: String(Math.floor(pointCount / 4)),
          end_point_id: String(Math.floor((pointCount * 3) / 4)),
        }
      : {}),
  })),
} as unknown as ChapterVisualPlanV3;

function baselineProjection() {
  return plan.points.filter((point) => {
    const target = plan.segments.find((segment) => segment.id === "target")!;
    const [start, end] = segmentTimes(plan, target);
    return point.at_ms > start && point.at_ms < end;
  });
}

const variants = {
  baseline: baselineProjection,
  candidate: () => pointsInsideSegment(plan, "target"),
};
const median = (values: number[]) =>
  [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)];
const samples: Record<keyof typeof variants, number[]> = {
  baseline: [],
  candidate: [],
};
let expectedIds: string[] | null = null;
for (let round = 0; round < 2; round += 1) {
  for (const variant of ["baseline", "candidate"] as const) {
    const ids = variants[variant]().map((point) => point.id);
    if (expectedIds == null) expectedIds = ids;
    else if (ids.length !== expectedIds.length || ids.some((id, index) => id !== expectedIds?.[index])) {
      throw new Error("Split point projection changed its point IDs or order");
    }
  }
}
for (let round = 0; round < 7; round += 1) {
  const order = round % 2 === 0 ? ["baseline", "candidate"] : ["candidate", "baseline"];
  for (const variant of order as Array<keyof typeof variants>) {
    const startedAt = performance.now();
    const ids = variants[variant]().map((point) => point.id);
    samples[variant].push(performance.now() - startedAt);
    if (expectedIds == null) expectedIds = ids;
    else if (ids.length !== expectedIds.length || ids.some((id, index) => id !== expectedIds?.[index])) {
      throw new Error("Split point projection changed its point IDs or order");
    }
  }
}
const baselineP50 = median(samples.baseline);
const candidateP50 = median(samples.candidate);
const queryTime = Math.floor(pointCount * 0.9) * 1000 + 500;
const baselineSegmentAt = () => plan.segments.find((segment) => {
  const [start, end] = segmentTimes(plan, segment);
  return queryTime >= start && queryTime < end;
}) ?? (queryTime === plan.points.at(-1)?.at_ms ? plan.segments.at(-1) : undefined);
const segmentLookupVariants = {
  baseline: baselineSegmentAt,
  candidate: () => segmentAt(plan, queryTime),
};
const segmentLookupSamples: Record<keyof typeof segmentLookupVariants, number[]> = {
  baseline: [],
  candidate: [],
};
const expectedSegmentId = segmentLookupVariants.baseline()?.id;
for (let round = 0; round < 2; round += 1) {
  for (const variant of ["baseline", "candidate"] as const) {
    if (segmentLookupVariants[variant]()?.id !== expectedSegmentId) {
      throw new Error("Segment lookup changed the selected segment");
    }
  }
}
for (let round = 0; round < 7; round += 1) {
  const order = round % 2 === 0 ? ["baseline", "candidate"] : ["candidate", "baseline"];
  for (const variant of order as Array<keyof typeof segmentLookupVariants>) {
    const startedAt = performance.now();
    const segmentId = segmentLookupVariants[variant]()?.id;
    segmentLookupSamples[variant].push(performance.now() - startedAt);
    if (segmentId !== expectedSegmentId) throw new Error("Segment lookup changed the selected segment");
  }
}
const baselineSegmentLookupP50 = median(segmentLookupSamples.baseline);
const candidateSegmentLookupP50 = median(segmentLookupSamples.candidate);
const report = {
  success: true,
  pointCount,
  segmentCount: plan.segments.length,
  optionCount: expectedIds?.length ?? 0,
  warmupRounds: 2,
  rounds: 7,
  samples,
  baselineP50Ms: Number(baselineP50.toFixed(3)),
  candidateP50Ms: Number(candidateP50.toFixed(3)),
  reductionPercent: Number(((1 - candidateP50 / baselineP50) * 100).toFixed(2)),
  segmentLookup: {
    queryTime,
    selectedSegmentId: expectedSegmentId ?? null,
    warmupRounds: 2,
    rounds: 7,
    samples: segmentLookupSamples,
    baselineP50Ms: Number(baselineSegmentLookupP50.toFixed(3)),
    candidateP50Ms: Number(candidateSegmentLookupP50.toFixed(3)),
    reductionPercent: Number(((1 - candidateSegmentLookupP50 / baselineSegmentLookupP50) * 100).toFixed(2)),
  },
};
document.getElementById("timeline-split-report")!.textContent = JSON.stringify(report);
