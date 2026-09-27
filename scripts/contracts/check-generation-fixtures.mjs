import fs from "node:fs";
import assert from "node:assert/strict";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { compileGenerationValidators } from "./generate-generation-validators.mjs";
import * as generated from "../../frontend/src/api/generated/generationValidators.js";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const schemaPath = path.join(
  root,
  "frontend/src/api/generated/generation-contract.schema.json",
);
const fixtureDir = path.join(
  root,
  "crates/process/a4n-service-protocol/fixtures/generation-events-v1",
);
const schema = JSON.parse(fs.readFileSync(schemaPath, "utf8"));
const { ajv, validators } = compileGenerationValidators(schema);
const validateEvent = generated.eventValidator;

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

// The generated browser code must preserve AJV's decisions and complete errors,
// including malformed input and Unicode boundaries, for every exported validator.
const candidates = [null, false, 0, "", [], {}, ...events];
for (const event of events) {
  candidates.push({ ...event, future_field: true });
  const visit = (object, parentPath = []) => {
    for (const [key, child] of Object.entries(object)) {
      for (const replacement of [undefined, null, false, -1, 9_007_199_254_740_992, "", "猫🙂"]) {
        const copy = structuredClone(event);
        const parent = parentPath.reduce((value, part) => value[part], copy);
        if (replacement === undefined) delete parent[key];
        else parent[key] = replacement;
        candidates.push(copy);
      }
      if (child && typeof child === "object" && !Array.isArray(child)) visit(child, [...parentPath, key]);
    }
  };
  visit(event);
}
for (const [name, reference] of Object.entries(validators)) {
  for (const candidate of candidates) {
    assert.equal(generated[name](candidate), reference(candidate), `${name} decision changed`);
    assert.deepEqual(generated[name].errors, reference.errors, `${name} errors changed`);
  }
}

console.log(
  `generation fixtures valid: ${eventFiles.length} events, ${vectors.length} UTF-8 vectors; ${candidates.length * Object.keys(validators).length} standalone/reference checks`,
);
