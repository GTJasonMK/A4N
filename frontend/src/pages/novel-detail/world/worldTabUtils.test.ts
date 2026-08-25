import { describe, expect, it } from "vitest";
import { translateMessage, type MessageTranslator } from "../../../i18n";

import { worldSettingSaveNotice } from "./worldTabUtils";

const t: MessageTranslator = (key, params) => translateMessage("zh-CN", key, params);

describe("worldSettingSaveNotice", () => {
  it("keeps a committed save successful when indexing reports a warning", () => {
    expect(worldSettingSaveNotice(["资料检索索引更新失败：服务不可用"], t)).toEqual({
      message: "世界观已保存；资料检索索引更新失败：服务不可用",
      type: "info",
    });
  });
});
