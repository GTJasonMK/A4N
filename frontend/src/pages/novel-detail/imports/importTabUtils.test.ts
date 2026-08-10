import { describe, expect, it } from "vitest";
import {
  formatImportCandidateSourceScope,
  parseImportBlueprintCandidate,
  parseImportPartOutlineCandidate,
  parseReverseCandidateEnvelope,
  reverseCandidateFromCheckpoint,
} from "./importTabUtils";

const blueprintProposal = {
  state: "proposal",
  origin: "import_analysis",
  project_id: 7,
  title: "星门调查",
  genre: "长篇小说",
  one_sentence_summary: "调查员追查失落星门。",
  full_synopsis: "调查员沿废弃航线追查失落星门的真相。",
  world_setting: { locations: ["旧城"], items: ["星图"], tags: ["悬疑"] },
  total_chapters: 25,
  needs_part_outlines: false,
  chapters_per_part: null,
  source_scope: { chapter_start: 1, chapter_end: 25, chapter_count: 25 },
};

const part = {
  part_number: 1,
  title: "第一部分",
  start_chapter: 1,
  end_chapter: 25,
  summary: "调查从旧城开始。",
  theme: "导入正文回顾",
  key_events: [],
  character_focus: [],
  source_scope: { chapter_start: 1, chapter_end: 25, chapter_count: 25 },
};

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

  it("keeps proposal apply_result optional but requires complete display fields", () => {
    expect(parseImportBlueprintCandidate(blueprintProposal, 7)).not.toBeNull();
    expect(
      parseImportBlueprintCandidate(
        { ...blueprintProposal, source_scope: undefined },
        7,
      ),
    ).toBeNull();
  });

  it("requires the structured result on confirmed candidates", () => {
    expect(
      parseImportBlueprintCandidate(
        { ...blueprintProposal, state: "confirmed", confirmed_at: "now" },
        7,
      ),
    ).toBeNull();
    const confirmed = parseImportBlueprintCandidate(
      {
        ...blueprintProposal,
        state: "confirmed",
        confirmed_at: "now",
        apply_result: { version: "1.0", updated_fields: ["genre"] },
      },
      7,
    );
    expect(confirmed?.state).toBe("confirmed");
    expect(
      parseImportBlueprintCandidate(
        {
          ...blueprintProposal,
          state: "confirmed",
          confirmed_at: "now",
          apply_result: { version: "1.0", updated_fields: [1] },
        },
        7,
      ),
    ).toBeNull();
  });

  it("uses one complete evidence-scope formatter for both candidate panels", () => {
    expect(
      formatImportCandidateSourceScope({
        chapter_start: 1,
        chapter_end: 25,
        chapter_count: 25,
      }),
    ).toBe("第 1–25 章（25 章）");
    const proposal = {
      state: "proposal",
      origin: "import_analysis",
      project_id: 7,
      parts: [part],
    };
    expect(parseImportPartOutlineCandidate(proposal, 7)).not.toBeNull();
    expect(
      parseImportPartOutlineCandidate(
        {
          ...proposal,
          state: "confirmed",
          confirmed_at: "now",
          apply_result: {},
        },
        7,
      ),
    ).toBeNull();
    expect(
      parseImportPartOutlineCandidate(
        {
          ...proposal,
          state: "confirmed",
          confirmed_at: "now",
          apply_result: {
            part_count: 1,
            inserted_count: 0,
            updated_count: -1,
            reused_count: 1,
          },
        },
        7,
      ),
    ).toBeNull();
  });
});
