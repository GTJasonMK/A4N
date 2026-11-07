import assert from "node:assert/strict";
import { after, test } from "node:test";
import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { runInNewContext } from "node:vm";

const root = fileURLToPath(new URL("../../../.dev-runtime/fixture-bootstrap-tests/", import.meta.url));
await mkdir(root, { recursive: true });
const directory = await mkdtemp(join(root, "run-"));
const previousPath = process.env.A4N_UI_FIXTURE_PATH;
process.env.A4N_UI_FIXTURE_PATH = join(directory, "fixtures.json");
const { installUiFixtures } = await import("./fixtures.mjs");
after(async () => {
  if (previousPath == null) delete process.env.A4N_UI_FIXTURE_PATH;
  else process.env.A4N_UI_FIXTURE_PATH = previousPath;
  await rm(directory, { recursive: true });
});

async function bootstrapBeforeFixtureResponse(query, initialState = {}) {
  let preload;
  await installUiFixtures({
    sessionId: query,
    scriptAddPreloadScript: async ({ functionDeclaration }) => {
      preload = functionDeclaration;
      return { script: query };
    },
    scriptRemovePreloadScript: async () => {},
  });
  const state = new Map(Object.entries(initialState));
  const localStorage = {
    setItem: (key, value) => state.set(key, value),
    removeItem: (key) => state.delete(key),
  };
  runInNewContext(`(${preload})()`, {
    window: { fetch: () => new Promise(() => {}) },
    location: { href: `http://localhost/${query}` },
    localStorage,
    URL,
  });
  return state;
}

test("authenticated fixture identity exists before the first application module can run", async () => {
  const state = await bootstrapBeforeFixtureResponse("?ui-locale=en-US&ui-appearance=dark");
  assert.equal(state.get("a4n:current_access_token"), "ui-test-fixture");
  assert.equal(state.get("a4n:current_user_id"), "1");
  assert.equal(state.get("a4n:settings:locale"), "en-US");
  assert.equal(state.get("a4n:theme-appearance:guest"), "dark");
});

test("guest fixture removes previous identity before waiting for fixture data", async () => {
  const state = await bootstrapBeforeFixtureResponse("?ui-auth=1", {
    "a4n:current_user_id": "previous-user",
    "a4n:current_access_token": "previous-token",
  });
  assert.equal(state.has("a4n:current_access_token"), false);
  assert.equal(state.has("a4n:current_user_id"), false);
});
