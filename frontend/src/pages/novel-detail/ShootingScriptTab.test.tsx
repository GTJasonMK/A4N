import { describe, expect, it } from "vitest";

import type { ChapterShootingScriptUnitProgressView } from "../../api/shootingScript";
import { assertNextUnitProgressPreservesScenes } from "./ShootingScriptTab";

const progress = (
  completed: number,
  sceneIndexes: number[],
): ChapterShootingScriptUnitProgressView => ({
  workflow_run_id: 349,
  completed_unit_count: completed,
  total_unit_count: 25,
  completed_record: null,
  script: {
    schema_version: "chapter_shooting_script_v1",
    directing_statement:
      {} as ChapterShootingScriptUnitProgressView["script"]["directing_statement"],
    scenes: sceneIndexes.map((scene_index) => ({ scene_index }) as never),
  },
});

describe("assertNextUnitProgressPreservesScenes", () => {
  it("accepts one appended scene and rejects a shorter replacement", () => {
    const previous = progress(4, [1, 2, 3, 4]);

    expect(() =>
      assertNextUnitProgressPreservesScenes(
        previous,
        progress(5, [1, 2, 3, 4, 5]),
      ),
    ).not.toThrow();
    expect(() =>
      assertNextUnitProgressPreservesScenes(previous, progress(1, [1])),
    ).toThrow("已拒绝覆盖当前页面数据");
  });
});
