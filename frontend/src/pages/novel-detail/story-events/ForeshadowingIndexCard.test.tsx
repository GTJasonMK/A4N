import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import type { Foreshadowing } from "../../../api/foreshadowing";
import { ForeshadowingIndexCard } from "./ForeshadowingIndexCard";
import { I18nProvider } from "../../../i18n";

describe("ForeshadowingIndexCard", () => {
  it("renders the formal index and keeps mutations behind editor permission", () => {
    const record: Foreshadowing = {
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
      <I18nProvider initialLocale="zh-CN">
        <ForeshadowingIndexCard
        projectId={3}
        records={[record]}
        canEdit
        loading={false}
        loadError={null}
        onReload={async () => undefined}
        onSaved={() => undefined}
        onDeleted={() => undefined}
        />
      </I18nProvider>,
    );
    expect(editable).toContain("正式伏笔索引");
    expect(editable).toContain("门后的脚步声");
    expect(editable).toContain("第 2 章埋设");
    expect(editable).toContain("新增伏笔");

    const readonly = renderToStaticMarkup(
      <I18nProvider initialLocale="zh-CN">
        <ForeshadowingIndexCard
        projectId={3}
        records={[record]}
        canEdit={false}
        loading={false}
        loadError={null}
        onReload={async () => undefined}
        onSaved={() => undefined}
        onDeleted={() => undefined}
        />
      </I18nProvider>,
    );
    expect(readonly).not.toContain("新增伏笔");
    expect(readonly).not.toContain("编辑伏笔：门后的脚步声");
  });

  it("does not present a failed index read as an empty collection", () => {
    const markup = renderToStaticMarkup(
      <I18nProvider initialLocale="zh-CN">
        <ForeshadowingIndexCard
          projectId={3}
          records={[]}
          canEdit
          loading={false}
          loadError="伏笔读取失败"
          onReload={async () => false}
          onSaved={() => undefined}
          onDeleted={() => undefined}
        />
      </I18nProvider>,
    );

    expect(markup).not.toContain("还没有正式伏笔");
    expect(markup).toContain("disabled");
  });
});
