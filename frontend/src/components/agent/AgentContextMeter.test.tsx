import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { AgentContextMeter } from "./AgentContextMeter";

describe("AgentContextMeter", () => {
  it("shows the fitted input percentage and token detail", () => {
    const html = renderToStaticMarkup(
      <AgentContextMeter
        run={{ context_input_tokens: 420, context_capacity_tokens: 1000 }}
        locale="zh-CN"
      />,
    );

    expect(html).toContain("上下文");
    expect(html).toContain("42%");
    expect(html).toContain("420 / 1,000 个可用输入 token");
    expect(html).toContain('style="width:42%"');
  });

  it("clamps an over-capacity estimate and hides incomplete data", () => {
    const empty = renderToStaticMarkup(
      <AgentContextMeter
        run={{ context_input_tokens: 0, context_capacity_tokens: 1000 }}
        locale="en-US"
      />,
    );
    expect(empty).toContain("0%");

    const over = renderToStaticMarkup(
      <AgentContextMeter
        run={{ context_input_tokens: 1200, context_capacity_tokens: 1000 }}
        locale="en-US"
      />,
    );
    expect(over).toContain("100%");

    expect(
      renderToStaticMarkup(
        <AgentContextMeter
          run={{ context_input_tokens: null, context_capacity_tokens: null }}
          locale="zh-CN"
        />,
      ),
    ).toBe("");
  });
});
