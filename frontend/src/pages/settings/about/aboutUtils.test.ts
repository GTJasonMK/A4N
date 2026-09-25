import { expect, it } from "vitest";
import { sessionDeviceLabel } from "./aboutUtils";

it("shows browser and device without confusing compatibility identifiers", () => {
  expect(sessionDeviceLabel("Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 CriOS/140.0 Mobile/15E148 Safari/604.1")).toBe("Chrome · iPhone");
  expect(sessionDeviceLabel("Mozilla/5.0 (Windows NT 10.0) Chrome/140.0 Safari/537.36 Edg/140.0")).toBe("Edge · Windows");
  expect(sessionDeviceLabel("A4N Desktop / Windows 11")).toBe("A4N Desktop · Windows");
  expect(sessionDeviceLabel("custom-cli/2")).toBe("custom-cli/2");
  expect(sessionDeviceLabel(null)).toBe("");
});
