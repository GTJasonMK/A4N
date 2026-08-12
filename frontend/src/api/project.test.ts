import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ apiInvoke: vi.fn() }));

vi.mock("./index", () => ({ apiInvoke: mocks.apiInvoke }));

import {
  canEditProjectMaterials,
  listAllProjects,
  projectLifecycleBlockedReason,
} from "./project";

beforeEach(() => mocks.apiInvoke.mockReset());

describe("project lifecycle gate", () => {
  it("keeps pending reset readable but not editable", () => {
    const project = { lifecycle_gate: "resetting", my_role: "owner" } as const;

    expect(canEditProjectMaterials(project)).toBe(false);
    expect(projectLifecycleBlockedReason(project)).toContain("完成前暂不可编辑");
  });
});

it("loads every project page", async () => {
  mocks.apiInvoke
    .mockResolvedValueOnce({ items: [{ id: 2 }], total: 2, page: 1, page_size: 100 })
    .mockResolvedValueOnce({ items: [{ id: 1 }], total: 2, page: 2, page_size: 100 });

  const projects = await listAllProjects({ access_scope: "all" });

  expect(projects.map((project) => project.id)).toEqual([2, 1]);
  expect(mocks.apiInvoke.mock.calls).toEqual([
    ["list_projects", {
      page: 1,
      pageSize: 100,
      projectKind: undefined,
      rootProjectId: undefined,
      accessScope: "all",
    }],
    ["list_projects", {
      page: 2,
      pageSize: 100,
      projectKind: undefined,
      rootProjectId: undefined,
      accessScope: "all",
    }],
  ]);
});
