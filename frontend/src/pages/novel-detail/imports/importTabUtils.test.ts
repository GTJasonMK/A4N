import { describe, expect, it } from "vitest";
import {
  formalReverseNotice,
  importAnalysisDialogFromProgress,
  parseReverseCandidateEnvelope,
  reverseCandidateFromCheckpoint,
} from "./importTabUtils";

describe("import candidate contracts", () => {
  it("maps strict import analysis progress to the completed dialog state", () => {
    expect(
      importAnalysisDialogFromProgress({
        cancelled_at: null,
        completed_at: "now",
        current_stage: "generating_analysis_data",
        error: null,
        failed_at: null,
        message: "导入项目分析已完成",
        overall_progress: 100,
        stages: {
          generating_analysis_data: {
            completed: 3,
            name: "生成分析数据",
            status: "completed",
            total: 3,
          },
        },
        started_at: "now",
        status: "completed",
      }),
    ).toEqual({
      open: true,
      status: "success",
      current: 3,
      total: 3,
      message: "导入项目分析已完成",
      error: null,
    });
  });

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

  it("reports a committed aggregation with sync warnings as information", () => {
    expect(formalReverseNotice("蓝图已归纳并写入正式资料", ["资料检索索引更新失败"])).toEqual({
      message: "蓝图已归纳并写入正式资料；资料检索索引更新失败",
      tone: "info",
    });
  });

  it("restores one persisted reverse candidate from its workflow checkpoint", () => {
    const restored = reverseCandidateFromCheckpoint(
      {
        id: 11,
        workflow_run_id: 9,
        step_key: "reverse_blueprint",
        step_order: 1,
        status: "completed",
        input_fingerprint_sha256: null,
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
        input_fingerprint_sha256: null,
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
          input_fingerprint_sha256: null,
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
