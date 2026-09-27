import { expect, it } from "vitest";
import type { MessageKey } from "../i18n";
import { shootingShotDurationLabel } from "./shootingScriptPresentation";

it("shows directing time without using retired estimates or inventing missing timing", () => {
  const t = (key: MessageKey) => key === "units.seconds" ? "秒" : "尚未绑定导演时间";
  const shot = { planned_start_ms: 1_000, planned_end_ms: 9_000, estimated_natural_seconds: 999 };
  expect(shootingShotDurationLabel(shot, "zh-CN", t)).toBe("8 秒");
  expect(shootingShotDurationLabel({ planned_start_ms: 0, planned_end_ms: 1_250 }, "zh-CN", t)).toBe("1.25 秒");
  for (const end of [null, 0, -1, Number.NaN]) {
    expect(shootingShotDurationLabel({ planned_start_ms: 0, planned_end_ms: end }, "zh-CN", t)).toBe("尚未绑定导演时间");
  }
});
