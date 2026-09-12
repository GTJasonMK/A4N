import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it } from "vitest";
import { MarkdownText } from "./MarkdownText";

describe("MarkdownText", () => {
  it("renders the block types commonly returned by models", () => {
    const html = renderToStaticMarkup(
      <MarkdownText
        text={`# 结论

这是一段 **重点**、*补充* 和 \`代码\`。

- 第一项
- 第二项

1. 第一步
2. 第二步

> 需要留意的事项

| 字段 | 状态 |
| --- | --- |
| 结果 | **完成** |

\`\`\`json
{"ok":true}
\`\`\``}
      />,
    );

    expect(html).toContain("<h3");
    expect(html).toContain("<strong>重点</strong>");
    expect(html).toContain("<em>补充</em>");
    expect(html).toContain("<code");
    expect(html).toContain("<ul");
    expect(html).toContain("<ol");
    expect(html).toContain("<blockquote");
    expect(html).toContain("<table");
    expect(html).toContain("<pre");
    expect(html).toContain('{&quot;ok&quot;:true}');
  });

  it("keeps unsafe link protocols as text while allowing normal links", () => {
    const html = renderToStaticMarkup(
      <MarkdownText
        text="[安全链接](https://example.com) [危险链接](javascript:alert(1))"
      />,
    );

    expect(html).toContain('href="https://example.com"');
    expect(html).not.toContain('href="javascript:');
    expect(html).toContain("危险链接");
  });
});
