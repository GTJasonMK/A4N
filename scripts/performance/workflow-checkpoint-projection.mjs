import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { gzipSync } from "node:zlib";
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";

assert.ok(process.env.A4N_TEST_DATABASE_URL);
const url = new URL(process.env.A4N_TEST_DATABASE_URL);
const table = "workflow_checkpoint_projection_probe";
const runCounts = new Map([[42, 1000], [43, 5000], [44, 10000]]);
const rows = 100_000;
const rounds = 7;
const fullProjection = `id, workflow_run_id, step_key, step_order, status, input_fingerprint_sha256,
  input_fingerprint_json, checkpoint_json, artifacts_json, generation_run_ids_json, error_message,
  created_at, updated_at, completed_at`;
const displayProjection = `id, workflow_run_id, step_key, step_order, status,
  NULL::text AS input_fingerprint_sha256, NULL::jsonb AS input_fingerprint_json,
  NULL::jsonb AS checkpoint_json, artifacts_json, NULL::jsonb AS generation_run_ids_json,
  error_message, created_at, updated_at, completed_at`;
const artifacts = path.resolve(process.env.A4N_PERF_ARTIFACTS ?? "data/performance-audit-20261015/workflow-checkpoint-projection");
const containers = execFileSync("docker", ["ps", "--format", "{{.Names}}\t{{.ID}}\t{{.Ports}}"], { encoding: "utf8" })
  .trim().split("\n").filter((line) => {
    const [name, , ports] = line.split("\t");
    return name.startsWith("a4n-perf-workflow-checkpoint-projection-") && ports?.includes(`127.0.0.1:${url.port}->5432/tcp`);
  });
assert.equal(containers.length, 1);
const container = containers[0].split("\t")[1];

function sql(value) {
  return execFileSync("docker", ["exec", "-i", container, "sh", "-c", 'PGPASSWORD="$POSTGRES_PASSWORD" psql -h 127.0.0.1 -p 5432 -X -q -A -t -v ON_ERROR_STOP=1 -U a4n_test -d a4n_test'], {
    input: value,
    encoding: "utf8",
    maxBuffer: 128 * 1024 * 1024,
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
  jsonb_build_object('provider_job_id', 'job-' || id, 'artifacts', repeat('a', 128)), jsonb_build_array(id), NULL,
  TIMESTAMP '2026-01-01' + id * INTERVAL '1 second',
  TIMESTAMP '2026-01-01' + id * INTERVAL '1 second', NULL
FROM generate_series(1, ${rows}) AS series(id)
ORDER BY md5(id::text);
CREATE UNIQUE INDEX ON ${table} (workflow_run_id, step_key);
CREATE INDEX ON ${table} (workflow_run_id, step_key, input_fingerprint_sha256);
ANALYZE ${table};
`);

function planNodes(plan) {
  const nodes = [];
  const visit = (node) => { nodes.push(node["Node Type"]); for (const child of node.Plans ?? []) visit(child); };
  visit(plan);
  return nodes;
}

function sample(workflowRunId, projection) {
  const statement = `SELECT ${projection} FROM ${table} WHERE workflow_run_id = ${workflowRunId} ORDER BY step_order ASC, id ASC`;
  const plan = JSON.parse(sql(`SET max_parallel_workers_per_gather = 0; EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON, TIMING OFF) ${statement};`))[0];
  const json = sql(`SELECT jsonb_agg(to_jsonb(result) ORDER BY result.step_order, result.id)::text FROM (${statement}) AS result;`);
  const records = JSON.parse(json);
  const displayValues = records.map((record) => ({
    id: record.id,
    workflow_run_id: record.workflow_run_id,
    step_key: record.step_key,
    step_order: record.step_order,
    status: record.status,
    artifacts_json: record.artifacts_json,
    error_message: record.error_message,
    created_at: record.created_at,
    updated_at: record.updated_at,
    completed_at: record.completed_at,
  }));
  const parseTimes = [];
  for (let index = 0; index < 9; index += 1) {
    const started = process.hrtime.bigint();
    const parsed = JSON.parse(json);
    const elapsed = Number(process.hrtime.bigint() - started) / 1e6;
    assert.equal(parsed.length, records.length);
    if (index >= 2) parseTimes.push(elapsed);
  }
  return {
    sql_ms: plan["Execution Time"],
    nodes: planNodes(plan.Plan),
    result_count: records.length,
    json_bytes: Buffer.byteLength(json),
    gzip_bytes: gzipSync(json).length,
    parse_p50_ms: [...parseTimes].sort((left, right) => left - right)[Math.floor(parseTimes.length / 2)],
    display_sha256: createHash("sha256").update(JSON.stringify(displayValues)).digest("hex"),
  };
}

const results = [...runCounts].map(([workflowRunId, resultCount]) => {
  const samples = { full: [], display: [] };
  for (let round = 0; round < rounds; round += 1) {
    for (const projection of (round % 2 === 0 ? ["full", "display"] : ["display", "full"])) {
      const queryProjection = projection === "full" ? fullProjection : displayProjection;
      const result = sample(workflowRunId, queryProjection);
      assert.equal(result.result_count, resultCount);
      samples[projection].push(result);
    }
  }
  const baseline = samples.full[0];
  for (const result of [...samples.full, ...samples.display]) {
    assert.equal(result.display_sha256, baseline.display_sha256);
  }
  const median = (values) => [...values].sort((left, right) => left - right)[Math.floor(values.length / 2)];
  const fullSql = median(samples.full.map(({ sql_ms }) => sql_ms));
  const displaySql = median(samples.display.map(({ sql_ms }) => sql_ms));
  const fullParse = median(samples.full.map(({ parse_p50_ms }) => parse_p50_ms));
  const displayParse = median(samples.display.map(({ parse_p50_ms }) => parse_p50_ms));
  const fullBytes = baseline.json_bytes;
  const displayBytes = samples.display[0].json_bytes;
  const fullGzipBytes = baseline.gzip_bytes;
  const displayGzipBytes = samples.display[0].gzip_bytes;
  return {
    workflow_run_id: workflowRunId,
    checkpoints: resultCount,
    full_sql_p50_ms: fullSql,
    display_sql_p50_ms: displaySql,
    sql_reduction_percent: Number(((1 - displaySql / fullSql) * 100).toFixed(2)),
    full_parse_p50_ms: fullParse,
    display_parse_p50_ms: displayParse,
    parse_reduction_percent: Number(((1 - displayParse / fullParse) * 100).toFixed(2)),
    full_json_bytes: fullBytes,
    display_json_bytes: displayBytes,
    json_reduction_percent: Number(((1 - displayBytes / fullBytes) * 100).toFixed(2)),
    full_gzip_bytes: fullGzipBytes,
    display_gzip_bytes: displayGzipBytes,
    gzip_reduction_percent: Number(((1 - displayGzipBytes / fullGzipBytes) * 100).toFixed(2)),
    display_sha256: baseline.display_sha256,
    full_plan: baseline.nodes,
    display_plan: samples.display[0].nodes,
    samples: {
      full: samples.full.map(({ sql_ms, parse_p50_ms }) => ({ sql_ms, parse_p50_ms })),
      display: samples.display.map(({ sql_ms, parse_p50_ms }) => ({ sql_ms, parse_p50_ms })),
    },
  };
});
const report = {
  table_rows: rows,
  repeated_json_bytes_per_row: { input_fingerprint_json: 128, checkpoint_json: 512, artifacts_json: 128 },
  rounds,
  parse_warmups: 2,
  parse_samples_per_query: 7,
  results,
};
await mkdir(artifacts, { recursive: true });
await writeFile(path.join(artifacts, "report.json"), JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
