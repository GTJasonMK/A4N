import { describe, expect, it } from "vitest";
import type { GenerationTaskViewState } from "../utils/generationEventReducer";
import {
  generatedMediaResultIds,
  verifyGeneratedMediaResult,
} from "./generationResultVerification";

interface TestRecord {
  id: number;
  project_id: number;
}

const spec = {
  projectId: 7,
  singleKind: "generated_image",
  batchKind: "generated_image_batch",
  artifactKind: "generated_image",
  artifactIdPrefix: "generated-image:",
  recordInScope: (record: TestRecord) => record.project_id === 7,
  errorMessage: "generation result is invalid",
};

function task(resultRef: unknown, ids: number[]): GenerationTaskViewState {
  return {
    taskId: "task-1",
    attempt: 1,
    resultRef,
    artifacts: ids.map((id) => ({
      artifact_id: `generated-image:${id}`,
      artifact_kind: "generated_image",
      byte_size: 4,
      content_type: "image/png",
      revision: `sha-${id}`,
      sha256: `sha-${id}`,
      temporary: false,
    })),
  } as unknown as GenerationTaskViewState;
}

describe("generated media result IDs", () => {
  it("extracts only the referenced single image before loading records", async () => {
    const generationTask = task(
      { kind: "generated_image", id: "42", revision: "sha-42" },
      [42],
    );

    expect(generatedMediaResultIds(generationTask, spec)).toEqual([42]);
    await expect(
      verifyGeneratedMediaResult(
        generationTask,
        [{ id: 41, project_id: 7 }, { id: 42, project_id: 7 }],
        spec,
      ),
    ).resolves.toEqual([{ id: 42, project_id: 7 }]);
  });

  it("sorts batch IDs and rejects duplicate or mismatched artifacts", () => {
    const batch = task(
      { kind: "generated_image_batch", id: "7", revision: "batch-revision" },
      [44, 42],
    );
    const duplicate = task(
      { kind: "generated_image_batch", id: "7", revision: "batch-revision" },
      [42, 42],
    );
    const mismatched = task(
      { kind: "generated_image", id: "42", revision: "sha-42" },
      [41],
    );

    expect(generatedMediaResultIds(batch, spec)).toEqual([42, 44]);
    expect(generatedMediaResultIds(duplicate, spec)).toBeNull();
    expect(generatedMediaResultIds(mismatched, spec)).toBeNull();
  });
});
