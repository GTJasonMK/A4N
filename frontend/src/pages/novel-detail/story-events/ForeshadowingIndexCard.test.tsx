import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { ForeshadowingIndexCard } from "./ForeshadowingIndexCard";

describe("ForeshadowingIndexCard", () => {
  it("renders the formal index and keeps mutations behind editor permission", () => {
    const record = {
      id: 7,
      project_id: 3,
      description: "门后的脚步声",
      setup_chapter: 2,
      payoff_chapter: null,
      status: "pending",
      related_characters: ["主角"],
      importance: 4,
      created_at: "2026-08-12 10:00:00",
      updated_at: "2026-08-12 10:00:00",
    };

    const editable = renderToStaticMarkup(
      <ForeshadowingIndexCard
        projectId={3}
        records={[record]}
        canEdit
        loading={false}
        onReload={async () => undefined}
      />,
    );
    expect(editable).toContain("正式伏笔索引");
    expect(editable).toContain("门后的脚步声");
    expect(editable).toContain("第 2 章埋设");
    expect(editable).toContain("新增伏笔");

    const readonly = renderToStaticMarkup(
      <ForeshadowingIndexCard
        projectId={3}
        records={[record]}
        canEdit={false}
        loading={false}
        onReload={async () => undefined}
      />,
    );
    expect(readonly).not.toContain("新增伏笔");
    expect(readonly).not.toContain("编辑伏笔：门后的脚步声");
  });
});
