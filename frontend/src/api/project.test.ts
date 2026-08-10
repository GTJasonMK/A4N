import { describe, expect, it } from "vitest";
import {
  canEditProjectMaterials,
  projectLifecycleBlockedReason,
} from "./project";

describe("project lifecycle gate", () => {
  it("keeps pending reset readable but not editable", () => {
    const project = { lifecycle_gate: "resetting", my_role: "owner" } as const;

    expect(canEditProjectMaterials(project)).toBe(false);
    expect(projectLifecycleBlockedReason(project)).toContain("完成前暂不可编辑");
  });
});
