import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { SynopsisCard } from "./SynopsisCard";

describe("SynopsisCard nonlinear blueprint entry", () => {
  it("offers aggregation, direct entry, and inspiration when no blueprint exists", () => {
    const html = renderToStaticMarkup(
      <SynopsisCard
        blueprint={null}
        loading={false}
        refining={false}
        canEditBlueprint
        onOpenSynopsis={() => undefined}
        onRefine={() => undefined}
        onEditBlueprint={() => undefined}
        onOpenImports={() => undefined}
        onCreateBlueprint={() => undefined}
      />,
    );

    expect(html).toContain("前往导入/归纳蓝图");
    expect(html).toContain("直接录入蓝图");
    expect(html).toContain("开始灵感对话");
    expect(html).not.toContain("候选蓝图");
  });
});
