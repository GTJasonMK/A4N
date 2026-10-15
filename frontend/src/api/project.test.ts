import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({ apiInvoke: vi.fn() }));

vi.mock("./index", () => ({ apiInvoke: mocks.apiInvoke }));

import {
  canEditProjectMaterials,
  getSourceSnapshotChapter,
  listSourceSnapshotChapterIndex,
  listAllProjects,
  projectLifecycleBlockedReason,
} from "./project";

beforeEach(() => mocks.apiInvoke.mockReset());

describe("project lifecycle gate", () => {
  it("keeps pending reset readable but not editable", () => {
    const project = { lifecycle_gate: "resetting", my_role: "owner" } as const;

    expect(canEditProjectMaterials(project)).toBe(false);
    expect(projectLifecycleBlockedReason(project)).toBe("resetting");
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

it("loads a narrow snapshot index and reads chapter details within that snapshot", async () => {
  const index = [{ id: 101, snapshot_id: 23, source_chapter_id: 11, chapter_number: 1, title: "第一章" }];
  const chapter = {
    id: 101,
    snapshot_id: 23,
    root_project_id: 2,
    source_chapter_id: 11,
    chapter_number: 1,
    title: "第一章",
    summary: "摘要",
    content: "正文",
    analysis_data_json: null,
    content_hash: "hash",
    analysis_hash: null,
  };
  mocks.apiInvoke.mockResolvedValueOnce(index).mockResolvedValueOnce(chapter);

  expect(await listSourceSnapshotChapterIndex(7)).toEqual(index);
  expect(await getSourceSnapshotChapter(7, 23, 11)).toEqual(chapter);
  expect(mocks.apiInvoke.mock.calls).toEqual([
    ["list_source_snapshot_chapter_index", { projectId: 7 }],
    ["get_source_snapshot_chapter", {
      projectId: 7,
      input: { snapshot_id: 23, source_chapter_id: 11 },
    }],
  ]);
});
