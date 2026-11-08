import readline from "node:readline";
import { createRequire } from "node:module";

const require = createRequire(new URL("../../frontend/package.json", import.meta.url));
const Ajv = require("ajv");
const Ajv2020 = require("ajv/dist/2020");
const engines = new Map();
const validators = new Map();
const definitions = new Map();
const lines = readline.createInterface({ input: process.stdin, crlfDelay: Infinity });
for await (const line of lines) {
  const record = JSON.parse(line);
  if (record.kind === "schema") {
    if (!record.schema_id || definitions.has(record.schema_id)) throw new Error("missing or duplicate schema definition");
    definitions.set(record.schema_id, record.schema);
    continue;
  }
  const schema = definitions.get(record.schema_id);
  if (!schema) throw new Error("case references undefined schema");
  const schemaText = JSON.stringify(schema);
  let validate = validators.get(schemaText);
  if (!validate) {
    const draft = schema.$schema?.includes("2020-12") ? "2020" : "draft7";
    let ajv = engines.get(draft);
    if (!ajv) {
      ajv = new (draft === "2020" ? Ajv2020 : Ajv)({ strict: false, allErrors: true, validateFormats: false });
      engines.set(draft, ajv);
    }
    validate = ajv.compile(schema);
    validators.set(schemaText, validate);
  }
  const accepted = validate(record.value);
  process.stdout.write(JSON.stringify({ case_id: record.case_id, accepted, errors: validate.errors }) + "\n");
}
