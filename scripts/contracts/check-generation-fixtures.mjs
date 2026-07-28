import fs from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const require = createRequire(path.join(root, "frontend/package.json"));
const Ajv2020 = require("ajv/dist/2020").default;
const schemaPath = path.join(
  root,
  "frontend/src/api/generated/generation-contract.schema.json",
);
const fixtureDir = path.join(
  root,
  "crates/a4n-service-protocol/fixtures/generation-events-v1",
);
const schema = JSON.parse(fs.readFileSync(schemaPath, "utf8"));
const ajv = new Ajv2020({
  allErrors: true,
  strict: true,
  formats: { int32: true, int64: true, uint16: true, uint32: true, uint64: true },
});
ajv.addSchema(schema, "a4n-generation-contract");
const validateEvent = ajv.getSchema(
  "a4n-generation-contract#/$defs/GenerationTaskEvent",
);
if (!validateEvent) throw new Error("GenerationTaskEvent schema is missing");

const eventFiles = fs
  .readdirSync(fixtureDir)
  .filter((name) => name.endsWith(".json") && name !== "utf8-vectors.json")
  .sort();
const events = eventFiles.map((file) => {
  const value = JSON.parse(fs.readFileSync(path.join(fixtureDir, file), "utf8"));
  if (!validateEvent(value)) {
    throw new Error(`${file}: ${ajv.errorsText(validateEvent.errors)}`);
  }
  return value;
});

const vectors = JSON.parse(
  fs.readFileSync(path.join(fixtureDir, "utf8-vectors.json"), "utf8"),
);
const encoder = new TextEncoder();
for (const vector of vectors) {
  const actual = encoder.encode(vector.text).byteLength;
  if (actual !== vector.utf8_bytes) {
    throw new Error(`${vector.name}: expected ${vector.utf8_bytes}, got ${actual}`);
  }
}

const firstDelta = events.find((event) => event.sequence === 2).event.data;
const secondDelta = events.find((event) => event.sequence === 3).event.data;
if (secondDelta.offset !== encoder.encode(firstDelta.delta).byteLength) {
  throw new Error("cross-chunk UTF-8 offset does not match the previous delta");
}

const invalid = structuredClone(events[0]);
invalid.future_field = true;
if (validateEvent(invalid)) throw new Error("unknown event fields must be rejected");

delete invalid.future_field;
invalid.sequence = 9_007_199_254_740_992;
if (validateEvent(invalid)) throw new Error("unsafe integers must be rejected");

console.log(
  `generation fixtures valid: ${eventFiles.length} events, ${vectors.length} UTF-8 vectors`,
);
