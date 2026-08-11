import { describe, expect, it } from "vitest";
import {
  parseReverseCandidateEnvelope,
  reverseCandidateFromCheckpoint,
} from "./importTabUtils";

describe("import candidate contracts", () => {
  it("maps the UI reverse type to the backend prompt target", () => {
    const candidate = parseReverseCandidateEnvelope(
      {
        candidate_id: 9,
        workflow_run_id: 9,
        origin: "import_reverse",
        target: "reverse_blueprint",
        state: "proposal",
      },
      "blueprint",
    );
    expect(candidate?.target).toBe("reverse_blueprint");
    expect(candidate?.warnings).toEqual([]);
  });

  it("keeps post-commit warnings on a confirmed reverse candidate", () => {
    expect(
      parseReverseCandidateEnvelope(
        {
          candidate_id: 9,
          workflow_run_id: 9,
          origin: "import_reverse",
          target: "reverse_outline",
          state: "confirmed",
          warnings: ["资料检索索引更新失败"],
        },
        "outline",
      )?.warnings,
    ).toEqual(["资料检索索引更新失败"]);
  });

  it("restores one persisted reverse candidate from its workflow checkpoint", () => {
    const restored = reverseCandidateFromCheckpoint(
      {
        id: 11,
        workflow_run_id: 9,
        step_key: "reverse_blueprint",
        step_order: 1,
        status: "completed",
        input_fingerprint_json: null,
        checkpoint_json: { parsed_output: { title: "旧城星门" } },
        artifacts_json: {
          candidate: {
            candidate_id: 9,
            workflow_run_id: 9,
            origin: "import_reverse",
            target: "reverse_blueprint",
            state: "proposal",
          },
        },
        generation_run_ids_json: null,
        error_message: null,
        created_at: "now",
        updated_at: "now",
        completed_at: "now",
      },
      "blueprint",
    );
    expect(restored?.workflow_run_id).toBe(9);
    expect(restored?.result).toEqual({ title: "旧城星门" });
  });

  it("restores the concrete apply result for a confirmed project aggregation", () => {
    const restored = reverseCandidateFromCheckpoint(
      {
        id: 12,
        workflow_run_id: 10,
        step_key: "reverse_part_outline",
        step_order: 1,
        status: "completed",
        input_fingerprint_json: null,
        checkpoint_json: { parsed_output: { parts: [] } },
        artifacts_json: {
          candidate: {
            candidate_id: 10,
            workflow_run_id: 10,
            origin: "import_reverse",
            target: "reverse_part_outline",
            state: "confirmed",
            apply_result: {
              part_outline_count: 2,
              inserted_count: 0,
              updated_count: 1,
              reused_count: 1,
            },
          },
        },
        generation_run_ids_json: null,
        error_message: null,
        created_at: "now",
        updated_at: "now",
        completed_at: "now",
      },
      "part_outline",
    );

    expect(restored?.applied).toEqual({
      part_outline_count: 2,
      inserted_count: 0,
      updated_count: 1,
      reused_count: 1,
    });
  });

  it("rejects a reverse candidate copied from another workflow run", () => {
    expect(
      reverseCandidateFromCheckpoint(
        {
          id: 11,
          workflow_run_id: 10,
          step_key: "reverse_blueprint",
          step_order: 1,
          status: "completed",
          input_fingerprint_json: null,
          checkpoint_json: { parsed_output: { title: "旧城星门" } },
          artifacts_json: {
            candidate: {
              candidate_id: 9,
              workflow_run_id: 9,
              origin: "import_reverse",
              target: "reverse_blueprint",
              state: "proposal",
            },
          },
          generation_run_ids_json: null,
          error_message: null,
          created_at: "now",
          updated_at: "now",
          completed_at: "now",
        },
        "blueprint",
      ),
    ).toBeNull();
  });
});
