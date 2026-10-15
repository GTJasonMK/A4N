import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

assert.ok(process.env.A4N_TEST_DATABASE_URL);
const url = new URL(process.env.A4N_TEST_DATABASE_URL);
const table = "workflow_checkpoint_order_index_probe";
const runCounts = new Map([[42, 1000], [43, 5000], [44, 10000]]);
const rows = 100_000;
const rounds = 7;
const existingUniqueIndex = "uidx_workflow_checkpoint_probe_run_step";
const existingFingerprintIndex = "idx_workflow_checkpoint_probe_run_step_fingerprint";
const candidateIndex = "idx_workflow_checkpoint_probe_run_order";
const artifacts = path.resolve(process.env.A4N_PERF_ARTIFACTS ?? "data/performance-audit-20261015/workflow-checkpoint-order-index");
const projection = `id, workflow_run_id, step_key, step_order, status, input_fingerprint_sha256,
  input_fingerprint_json, checkpoint_json, artifacts_json, generation_run_ids_json, error_message,
  created_at, updated_at, completed_at`;
const containers = execFileSync("docker", ["ps", "--format", "{{.Names}}\t{{.ID}}\t{{.Ports}}"], { encoding: "utf8" })
  .trim().split("\n").filter((line) => {
    const [name, , ports] = line.split("\t");
    return name.startsWith("a4n-perf-workflow-checkpoint-index-") && ports?.includes(`127.0.0.1:${url.port}->5432/tcp`);
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
  id bigint PRIMARY KEY, workflow_run_id bigint NOT NULL, step_key text NOT NULL, step_order integer NOT NULL,
  status text NOT NULL, input_fingerprint_sha256 text, input_fingerprint_json jsonb, checkpoint_json jsonb,
  artifacts_json jsonb, generation_run_ids_json jsonb, error_message text, created_at timestamp NOT NULL,
  updated_at timestamp NOT NULL, completed_at timestamp
);
INSERT INTO ${table}
SELECT id,
  CASE WHEN id <= 1000 THEN 42 WHEN id <= 6000 THEN 43 WHEN id <= 16000 THEN 44 ELSE (id % 996) + 1000 END,
  'step-' || id,
  CASE WHEN id <= 1000 THEN id WHEN id <= 6000 THEN id - 1000 WHEN id <= 16000 THEN id - 6000 ELSE id % 10000 END,
  CASE WHEN id % 17 = 0 THEN 'failed' ELSE 'completed' END,
  md5(id::text), jsonb_build_object('request', repeat('i', 128), 'seed', id),
  jsonb_build_object('checkpoint', repeat('p', 512), 'id', id),
  jsonb_build_object('artifacts', repeat('a', 128)), jsonb_build_array(id), NULL,
  TIMESTAMP '2026-01-01' + id * INTERVAL '1 second',
  TIMESTAMP '2026-01-01' + id * INTERVAL '1 second', NULL
FROM generate_series(1, ${rows}) AS series(id)
ORDER BY md5(id::text);
CREATE UNIQUE INDEX ${existingUniqueIndex} ON ${table} (workflow_run_id, step_key);
CREATE INDEX ${existingFingerprintIndex} ON ${table} (workflow_run_id, step_key, input_fingerprint_sha256);
ANALYZE ${table};
`);

function setCandidate(enabled) {
  sql(enabled
    ? `CREATE INDEX IF NOT EXISTS ${candidateIndex} ON ${table} (workflow_run_id, step_order ASC, id ASC); ANALYZE ${table};`
    : `DROP INDEX IF EXISTS ${candidateIndex}; ANALYZE ${table};`);
}

function planNodeTypes(plan) {
  const nodes = [];
  const visit = (node) => { nodes.push(node["Node Type"]); for (const child of node.Plans ?? []) visit(child); };
  visit(plan);
  return nodes;
}

function sample(workflowRunId) {
  const statement = `SELECT ${projection} FROM ${table} WHERE workflow_run_id = ${workflowRunId} ORDER BY step_order ASC, id ASC`;
  const plan = JSON.parse(sql(`SET max_parallel_workers_per_gather = 0; EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON, TIMING OFF) ${statement};`))[0];
  const json = sql(`SELECT jsonb_agg(to_jsonb(result) ORDER BY result.step_order, result.id)::text FROM (${statement}) AS result;`);
  const ids = sql(`SELECT id FROM (${statement}) AS result;`).split("\n").map(Number);
  return {
    execution_ms: plan["Execution Time"],
    nodes: planNodeTypes(plan.Plan),
    result_count: ids.length,
    ids,
    json_bytes: Buffer.byteLength(json),
    sha256: createHash("sha256").update(json).digest("hex"),
  };
}

const samples = Object.fromEntries([...runCounts.keys()].map((runId) => [runId, { baseline: [], indexed: [] }]));
for (let round = 0; round < rounds; round += 1) {
  for (const enabled of (round % 2 === 0 ? [false, true] : [true, false])) {
    setCandidate(enabled);
    for (const workflowRunId of runCounts.keys()) {
      const result = sample(workflowRunId);
      const batch = samples[workflowRunId];
      batch[enabled ? "indexed" : "baseline"].push(result);
    }
  }
}

const results = [...runCounts].map(([workflowRunId, resultCount]) => {
  const batch = samples[workflowRunId];
  const baseline = batch.baseline[0];
  for (const result of [...batch.baseline, ...batch.indexed]) {
    assert.equal(result.result_count, resultCount);
    assert.deepEqual(result.ids, baseline.ids);
    assert.equal(result.sha256, baseline.sha256);
    assert.equal(result.json_bytes, baseline.json_bytes);
  }
  const median = (values) => [...values].sort((left, right) => left - right)[Math.floor(values.length / 2)];
  const baselineP50 = median(batch.baseline.map((sample) => sample.execution_ms));
  const indexedP50 = median(batch.indexed.map((sample) => sample.execution_ms));
  return {
    workflow_run_id: workflowRunId,
    checkpoints: resultCount,
    baseline_p50_ms: baselineP50,
    indexed_p50_ms: indexedP50,
    reduction_percent: Number(((1 - indexedP50 / baselineP50) * 100).toFixed(2)),
    json_bytes: baseline.json_bytes,
    sha256: baseline.sha256,
    baseline_plan: batch.baseline[0].nodes,
    indexed_plan: batch.indexed[0].nodes,
    sample_timings_ms: Object.fromEntries(
      Object.entries(batch).map(([condition, values]) => [condition, values.map(({ execution_ms }) => execution_ms)]),
    ),
  };
});
setCandidate(true);
const report = {
  table_rows: rows,
  checkpoint_json_bytes_per_row: 512,
  existing_indexes: ["(workflow_run_id, step_key)", "(workflow_run_id, step_key, input_fingerprint_sha256)"],
  candidate_index_bytes: Number(sql(`SELECT pg_relation_size('${candidateIndex}');`)),
  rounds,
  results,
};
await mkdir(artifacts, { recursive: true });
await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
