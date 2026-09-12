import { expect, it } from "vitest";
import { buildModelDiscoveryField } from "./modelDiscovery";

it("changes discovery identity when credentials change within the same form", () => {
  const input = {
    provider: "openai", baseUrl: "https://api.example.test/v1", configId: 1,
    apiKey: "test-key-a", clearApiKey: false, hasApiKey: true, readonly: false,
    scope: "local" as const,
  };
  const initial = buildModelDiscoveryField(input);
  const changed = buildModelDiscoveryField({ ...input, apiKey: "test-key-b" });
  expect(changed.discoveryKey).not.toBe(initial.discoveryKey);
  expect(changed.input.api_key).toBe("test-key-b");
  expect(buildModelDiscoveryField({ ...input, apiKey: "  test-key-a  " }).discoveryKey)
    .toBe(initial.discoveryKey);
});
