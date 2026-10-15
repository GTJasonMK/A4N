import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

assert.ok(process.env.A4N_TEST_DATABASE_URL);
const url = new URL(process.env.A4N_TEST_DATABASE_URL);
const table = "workflow_generation_run_index_probe";
const existingIndex = "idx_workflow_generation_run_probe_created";
const candidateIndex = "idx_workflow_generation_run_probe_workflow_created";
const rows = 100_000;
const rounds = 7;
const artifacts = path.resolve(process.env.A4N_PERF_ARTIFACTS ?? "data/performance-audit-20261015/workflow-generation-run-index");
const projection = `id, workflow_run_id, workflow, step_key, call_key, title, model_kind, provider, model,
  status, input_fingerprint_sha256, input_fingerprint_json, raw_response_json, parsed_output_json,
  validation_json, artifact_refs_json, attempt, response_summary_json, error_message, created_at, completed_at`;
const statement = `SELECT ${projection} FROM ${table} WHERE workflow_run_id = 42 ORDER BY created_at ASC LIMIT 300`;
const countStatement = `SELECT COUNT(*) FROM (SELECT 1 FROM ${table} WHERE workflow_run_id = 42 LIMIT 300) AS limited_runs`;
const containers = execFileSync("docker", ["ps", "--format", "{{.Names}}\t{{.ID}}\t{{.Ports}}"], { encoding: "utf8" })
  .trim().split("\n").filter((line) => {
    const [name, , ports] = line.split("\t");
    return name.startsWith("a4n-perf-workflow-generation-index-") && ports?.includes(`127.0.0.1:${url.port}->5432/tcp`);
  });
assert.equal(containers.length, 1);
const container = containers[0].split("\t")[1];

function sql(value) {
  return execFileSync("docker", ["exec", "-i", container, "sh", "-c", 'PGPASSWORD="$POSTGRES_PASSWORD" psql -h 127.0.0.1 -p 5432 -X -q -A -t -v ON_ERROR_STOP=1 -U a4n_test -d a4n_test'], {
    input: value,
    encoding: "utf8",
    maxBuffer: 64 * 1024 * 1024,
  }).trim();
}

sql(`
DROP TABLE IF EXISTS ${table};
CREATE TABLE ${table} (
  id bigint PRIMARY KEY, workflow_run_id bigint, workflow text NOT NULL, step_key text NOT NULL,
  call_key text, title text NOT NULL, model_kind text NOT NULL, provider text NOT NULL, model text NOT NULL,
  status text NOT NULL, input_fingerprint_sha256 text, input_fingerprint_json jsonb, raw_response_json jsonb,
  parsed_output_json jsonb, validation_json jsonb, artifact_refs_json jsonb, attempt integer NOT NULL,
  response_summary_json jsonb, error_message text, created_at timestamp NOT NULL, completed_at timestamp
);
INSERT INTO ${table}
SELECT id,
  CASE WHEN id % 333 = 0 THEN 42 WHEN id % 2 = 0 THEN NULL ELSE (id % 997) + 1000 END,
  'video_canvas_graph', 'canvas_node:' || (id % 1000), 'call-' || id, 'Generation run ' || id,
  'text', 'provider', 'model', CASE WHEN id % 9 = 0 THEN 'failed' ELSE 'completed' END,
  md5(id::text), jsonb_build_object('request', repeat('i', 256), 'seed', id),
  jsonb_build_object('response', repeat('r', 512)), jsonb_build_object('parsed', repeat('p', 256)),
  jsonb_build_object('valid', true), jsonb_build_object('artifacts', jsonb_build_array(id)),
  1, jsonb_build_object('summary', repeat('s', 128)), NULL,
  TIMESTAMP '2026-01-01' + id * INTERVAL '1 second',
  TIMESTAMP '2026-01-01' + id * INTERVAL '1 second'
FROM generate_series(1, ${rows}) AS series(id);
CREATE INDEX ${existingIndex} ON ${table} (created_at DESC);
ANALYZE ${table};
`);

function setCandidate(enabled) {
  sql(enabled
    ? `CREATE INDEX IF NOT EXISTS ${candidateIndex} ON ${table} (workflow_run_id, created_at ASC) WHERE workflow_run_id IS NOT NULL; ANALYZE ${table};`
    : `DROP INDEX IF EXISTS ${candidateIndex}; ANALYZE ${table};`);
}

function sample() {
  const plan = JSON.parse(sql(`SET max_parallel_workers_per_gather = 0; EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON, TIMING OFF) ${statement};`))[0];
  const json = sql(`SELECT jsonb_agg(to_jsonb(result) ORDER BY result.created_at)::text FROM (${statement}) AS result;`);
  const ids = sql(`SELECT id FROM (${statement}) AS result;`).split("\n").map(Number);
  return { execution_ms: plan["Execution Time"], nodes: planNodeTypes(plan.Plan), result_count: ids.length, ids, json_bytes: Buffer.byteLength(json), sha256: createHash("sha256").update(json).digest("hex") };
}

function planNodeTypes(plan) {
  const nodes = [];
  const visit = (node) => { nodes.push(node["Node Type"]); for (const child of node.Plans ?? []) visit(child); };
  visit(plan);
  return nodes;
}

const samples = { baseline: [], indexed: [] };
const orders = { baseline: [], indexed: [] };
for (let round = 0; round < rounds; round += 1) {
  for (const enabled of (round % 2 === 0 ? [false, true] : [true, false])) {
    setCandidate(enabled);
    const result = sample();
    const name = enabled ? "indexed" : "baseline";
    orders[name].push(result.ids);
    samples[name].push({
      execution_ms: result.execution_ms,
      nodes: result.nodes,
      result_count: result.result_count,
      json_bytes: result.json_bytes,
      sha256: result.sha256,
    });
  }
}

assert.equal(orders.baseline[0].length, 300);
for (const ids of [...orders.baseline, ...orders.indexed]) assert.deepEqual(ids, orders.baseline[0]);
for (const result of [...samples.baseline, ...samples.indexed]) {
  assert.equal(result.sha256, samples.baseline[0].sha256);
  assert.equal(result.json_bytes, samples.baseline[0].json_bytes);
}
setCandidate(true);
const countSamples = [];
for (let round = 0; round < rounds; round += 1) {
  const plan = JSON.parse(sql(`SET max_parallel_workers_per_gather = 0; EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON, TIMING OFF) ${countStatement};`))[0];
  const count = Number(sql(`${countStatement};`));
  countSamples.push({ execution_ms: plan["Execution Time"], nodes: planNodeTypes(plan.Plan), count, result_bytes: Buffer.byteLength(String(count)) });
}

const median = (values) => [...values].sort((left, right) => left - right)[Math.floor(values.length / 2)];
const baseline = median(samples.baseline.map((sample) => sample.execution_ms));
const indexed = median(samples.indexed.map((sample) => sample.execution_ms));
const countOnly = median(countSamples.map((sample) => sample.execution_ms));
const report = {
  table_rows: rows,
  target_workflow_rows: 300,
  projection: "complete GenerationRunSummaryRecord columns including JSON fields",
  rounds,
  baseline_p50_ms: baseline,
  indexed_p50_ms: indexed,
  reduction_percent: Number(((1 - indexed / baseline) * 100).toFixed(2)),
  count_only: {
    p50_ms: countOnly,
    reduction_from_indexed_full_projection_percent: Number(((1 - countOnly / indexed) * 100).toFixed(2)),
    returned_count: countSamples[0].count,
    result_bytes: countSamples[0].result_bytes,
    samples: countSamples,
  },
  candidate_index_bytes: Number(sql(`SELECT pg_relation_size('${candidateIndex}');`)),
  full_json_bytes: samples.baseline[0].json_bytes,
  result_sha256: samples.baseline[0].sha256,
  result_order_preserved: true,
  samples,
};
await mkdir(artifacts, { recursive: true });
await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
